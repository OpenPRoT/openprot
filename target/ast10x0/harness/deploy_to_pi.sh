#!/usr/bin/env bash
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
#
# Copies everything the PLDM firmware update demo needs onto the Pi, so the
# demo can then be run entirely from an SSH session with no Bazel and no
# workstation involved:
#
#     ssh rot-ast-ctrl-2
#     cd ~/demo && python3 pi_local_test_runner.py
#
# Run this from the machine that did the Bazel build; it reads the images out
# of bazel-bin and does not build anything itself.

set -euo pipefail

HOST="${1:-rot-ast-ctrl-2}"
DEST="${2:-demo}"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../.." && pwd)"
BIN="$REPO_ROOT/bazel-bin/target/ast10x0/tests/pldm/firmware_update"

# .bin is what gets uploaded to a board; .elf is only read for its tokens, so
# the logs say words instead of $base64. pldm_ua_image has no .bin here because
# it is never uploaded directly -- it travels inside bootstrap_image.
ARTIFACTS=(
    "$BIN/pldm_fd_image.bin"
    "$BIN/pldm_fd_image.elf"
    "$BIN/bootstrap/bootstrap_image.bin"
    "$BIN/bootstrap/bootstrap_image.elf"
    "$BIN/pldm_ua_image.elf"
)

missing=()
for f in "${ARTIFACTS[@]}"; do
    [[ -f "$f" ]] || missing+=("$f")
done

if (( ${#missing[@]} )); then
    echo "Missing build artifacts:" >&2
    printf '  %s\n' "${missing[@]}" >&2
    echo >&2
    echo "Build them first:" >&2
    echo "  bazelisk build --config=k_ast1060_evb \\" >&2
    echo "    //target/ast10x0/tests/pldm/firmware_update:pldm_fd_image \\" >&2
    echo "    //target/ast10x0/tests/pldm/firmware_update:pldm_ua_image \\" >&2
    echo "    //target/ast10x0/tests/pldm/firmware_update/bootstrap:bootstrap_image" >&2
    exit 1
fi

echo "Deploying to $HOST:$DEST/"

# Bazel leaves its outputs read-only and scp copies that mode across, so the
# second deploy cannot overwrite the first. Clear them out before copying.
names=()
for f in "${ARTIFACTS[@]}"; do
    names+=("$(basename "$f")")
done
ssh "$HOST" "mkdir -p '$DEST' && cd '$DEST' && rm -f ${names[*]}"

# The vendored pw_tokenizer goes along so the Pi needs nothing installed.
scp -q \
    "$HERE/pi_test_runner.py" \
    "$HERE/pi_local_test_runner.py" \
    "${ARTIFACTS[@]}" \
    "$HOST:$DEST/"
scp -qr "$HERE/pw_tokenizer" "$HOST:$DEST/"

echo
echo "Done. On the Pi:"
echo "  ssh $HOST"
echo "  cd ~/$DEST && python3 pi_local_test_runner.py"
echo
echo "Then from two more SSH sessions, to watch each board on its own:"
echo "  tail -F ~/$DEST/logs/A.log      # RoT"
echo "  tail -F ~/$DEST/logs/B.log      # mock BMC"
