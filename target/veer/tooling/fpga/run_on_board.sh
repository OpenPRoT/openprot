#!/usr/bin/env bash
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
#
# Board-side half of run_remote.sh: runs on the VCK190 (as root, via sudo) to
# bootstrap the FPGA and run one test binary.
#
# Usage: run_on_board.sh <runfiles-dir> <test-binary> [args...]
#
#   runfiles-dir  The test's runfiles tree, as synced by run_remote.sh.
#   test-binary   Path to the test binary, relative to <runfiles-dir>/_main.
#
# steps:
#   1. take the board lock, to serialize tests using this board
#   2. Bootstrap the FPGA with the appropriate bitstream
#   3. Run the test with RUNFILES_DIR set.

set -euo pipefail

readonly LOCK_FILE=/run/lock/openprot-fpga.lock
readonly MAX_LOCK_WAIT_SECONDS=120
# Our bitstream, synced with the test (path relative to <runfiles-dir>/_main).
readonly BITSTREAM=target/veer/tooling/fpga/caliptra-bitstream.pdi
# Where the kernel FPGA manager loads bitstreams from (by name). Upstream xtask
# loads its bitstream from the same path, so comparing it against BITSTREAM
# tells whether the last-loaded bitstream is ours.
readonly LOADED_BITSTREAM=/lib/firmware/caliptra-bitstream.pdi
# Upstream xtask's record of the loaded FPGA configuration. Lives in /dev/shm,
# so it is cleared on power cycle.
readonly CONFIG_MARKER=/dev/shm/fpga-config
readonly FPGA_MGR=/sys/class/fpga_manager/fpga0

log() { echo "[run_on_board] $*" >&2; }
die() {
  log "error: $*"
  exit 2
}

take_lock() {
  exec 9>"$LOCK_FILE"
  flock -w "$MAX_LOCK_WAIT_SECONDS" 9 ||
    die "board is busy; $LOCK_FILE held for over ${MAX_LOCK_WAIT_SECONDS}s"
}

sha256() { { sha256sum "$1" 2>/dev/null || true; } | awk '{print $1}'; }

bootstrap_fpga() {
  # The Caliptra image ships the kernel modules and supports loading segmented
  # bitstreams at runtime; stock Ubuntu images do neither.
  [[ "$(hostname)" == "caliptra-fpga" ]] ||
    die "unsupported board image (hostname '$(hostname)'); use the Caliptra" \
      "FPGA image from caliptra-sw"
  [[ -r "$BITSTREAM" ]] || die "bitstream not found: $BITSTREAM"
  [[ -d "$FPGA_MGR" ]] || die "FPGA manager sysfs node missing: $FPGA_MGR"

  # Reload unless the subsystem bitstream is loaded and it's ours.
  local sha
  sha="$(sha256 "$BITSTREAM")"
  if [[ "$(cat "$CONFIG_MARKER" 2>/dev/null || true)" != "subsystem" ||
    "$(sha256 "$LOADED_BITSTREAM")" != "$sha" ]]; then
    log "Loading bitstream (${sha:0:12}...) into fpga0"
    rm -f "$CONFIG_MARKER" # So a failed load is retried next time.
    mkdir -p "$(dirname "$LOADED_BITSTREAM")"
    cp -f "$BITSTREAM" "$LOADED_BITSTREAM"
    basename "$LOADED_BITSTREAM" >"$FPGA_MGR/firmware" ||
      die "failed to write bitstream to $FPGA_MGR/firmware"
    echo subsystem >"$CONFIG_MARKER"
    chmod 666 "$CONFIG_MARKER" # Upstream xtask runs as a regular user.
  fi

  local state
  state="$(cat "$FPGA_MGR/state" 2>/dev/null || true)"
  [[ "$state" == "operating" ]] || die "fpga0 state is '$state' (expected 'operating')"
  for uio in /dev/uio0 /dev/uio1; do
    [[ -e "$uio" ]] || die "required UIO device $uio is missing"
  done
}

[[ $# -ge 2 ]] || die "usage: run_on_board.sh <runfiles-dir> <test-binary> [args...]"
((EUID == 0)) || die "must run as root"
runfiles="$(realpath "$1")"
test_bin="$2"
shift 2
cd "$runfiles/_main"

take_lock
bootstrap_fpga
RUNFILES_DIR="$runfiles" exec "$test_bin" "$@"
