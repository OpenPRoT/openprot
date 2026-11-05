#!/usr/bin/env bash
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
#
# Bazel `--run_under` wrapper (`bazel test --config=fpga`) that runs a
# cross-compiled aarch64 FPGA test binary on a remote VCK190 board.
#
# Invoked by Bazel as `run_remote <test-binary> [args...]`. It:
#   1. Copies the test's runfiles tree ($TEST_SRCDIR) to the board. The tree
#      includes run_on_board.sh and the bitstream (added as data by
#      `caliptra_test`).
#   2. Runs run_on_board.sh there as root, which bootstraps the FPGA and runs
#      the test. See that script for details.
#
# Environment:
#   VCK190_FPGA_HOST - Anything `ssh` accepts: an alias from ~/.ssh/config or
#                      user@host.

set -euo pipefail

readonly REMOTE_RUNFILES=openprot-fpga/runfiles # Relative to the SSH user's home.
# Test environment to forward to the board, if set.
readonly FORWARDED_ENV=(CALIPTRA_BOOT_ENV CPTRA_UIO_NUM RUST_BACKTRACE RUST_LOG RUST_TEST_THREADS)
# Share one SSH connection between rsync, ssh, and consecutive tests (kept open
# for 60s after the last use) instead of paying ~2s of connection setup for
# each. BatchMode makes ssh fail rather than prompt.
readonly SSH_OPTS=(
  -o BatchMode=yes
  -o ControlMaster=auto
  -o "ControlPath=/tmp/openprot-fpga-ssh-%C"
  -o ControlPersist=60
)

: "${VCK190_FPGA_HOST:?is not set (pass --test_env=VCK190_FPGA_HOST=<host>)}"
: "${TEST_SRCDIR:?is not set; run_remote must be used via bazel test --run_under}"
: "${TEST_BINARY:?is not set; run_remote must be used via bazel test --run_under}"
[[ $# -ge 1 ]] || {
  echo "usage: run_remote <test-binary> [args...]" >&2
  exit 2
}
shift # The test binary; $TEST_BINARY is the same path, relative to _main.

# 1. Copy the runfiles tree, resolving Bazel's symlinks. Bazel's MANIFEST maps
# to absolute host paths and would put the `runfiles` crate into manifest mode
# on the board, so skip it.
rsync -a --copy-links --exclude=/MANIFEST \
  -e "ssh ${SSH_OPTS[*]}" \
  --rsync-path="mkdir -p $REMOTE_RUNFILES && rsync" \
  "$TEST_SRCDIR/" "$VCK190_FPGA_HOST:$REMOTE_RUNFILES/" || {
  echo "error: failed to copy runfiles to $VCK190_FPGA_HOST" >&2
  exit 2
}

# 2. Run on the board. `sudo` drops the environment, so forward it explicitly.
env_vars=()
for var in "${FORWARDED_ENV[@]}"; do
  if [[ -n "${!var:-}" ]]; then
    env_vars+=("$var=${!var}")
  fi
done
remote_cmd=(
  sudo env "${env_vars[@]}"
  "$REMOTE_RUNFILES/_main/target/veer/tooling/fpga/run_on_board.sh"
  "$REMOTE_RUNFILES" "./$TEST_BINARY" "$@"
)

rc=0
ssh -tt "${SSH_OPTS[@]}" -o LogLevel=ERROR "$VCK190_FPGA_HOST" \
  "$(printf '%q ' "${remote_cmd[@]}")" | tr -d '\r' || rc=$?

if ((rc == 255)); then
  echo "error: SSH connection to $VCK190_FPGA_HOST failed" >&2
  exit 2
fi
exit "$rc"
