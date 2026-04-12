# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Sample versions.bzl for uprev_test.py fixtures.

Edge case: caliptra_sw and caliptra_cfi intentionally share the same
SHA (bbbb...) to verify key-anchored regex — bumping one must not
touch the other.
"""

CALIPTRA_VERSIONS = {
    "caliptra_cfi": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "caliptra_dpe": "dddddddddddddddddddddddddddddddddddddddd",
    "caliptra_mcu_sw": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "caliptra_sw": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "release_tag": "",
    "ureg": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
}
