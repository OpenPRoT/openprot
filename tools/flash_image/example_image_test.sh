#!/usr/bin/env bash
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
#
# Checks the image the flash_image rule built: both payloads at their slot
# bases, erased everywhere else, and exactly flash_size bytes. This covers
# the Bazel wiring; the layout rules themselves are covered by
# //tools/flash_image:flash_image_test.

set -euo pipefail

image="${TEST_SRCDIR}/_main/tools/flash_image/example_image.img"

# wc rather than stat: runfiles entries are symlinks, and GNU stat reports
# the link itself unless told otherwise.
actual_size=$(wc -c < "${image}")
if [ "${actual_size}" -ne 64 ]; then
    echo "expected a 64-byte image, got ${actual_size}" >&2
    exit 1
fi

# od keeps this readable: slot A at 0x00, slot B at 0x20, 0xFF between.
expected=$(
    cat <<'EOF'
0000000   S   L   O   T   -   A   -   C   O   N   T   E   N   T 377 377
0000020 377 377 377 377 377 377 377 377 377 377 377 377 377 377 377 377
0000040   S   L   O   T   -   B 377 377 377 377 377 377 377 377 377 377
0000060 377 377 377 377 377 377 377 377 377 377 377 377 377 377 377 377
0000100
EOF
)

if ! diff <(od -c "${image}") <(echo "${expected}"); then
    echo "image contents do not match the declared layout" >&2
    exit 1
fi
