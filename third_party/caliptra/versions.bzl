# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Canonical caliptra git pins for this openprot checkout.

Downstream integrators: fork this file to target a different
caliptra release. The uprev tool (`bazel run
//third_party/caliptra:uprev -- <subcommand>`) keeps the derived
Cargo.toml and MODULE.bazel entries in sync with whatever values
are set here.

Layout contract — ONE key per line, quoted string values, no
multi-line strings. The uprev regex-edit path depends on this
format. Do not "improve" the formatting.

Subcommands:
  verify               — cross-check that MODULE.bazel,
                         crates_io/embedded/Cargo.toml, and
                         crates_io/host/Cargo.toml all agree with
                         the values set here plus the derived
                         Cargo.lock in caliptra-mcu-sw.
  bump <new-mcu-sw-sha> — set caliptra_mcu_sw to the given SHA,
                         derive caliptra_sw + caliptra_cfi from
                         its Cargo.lock, update all files.
  latest               — resolve caliptra-mcu-sw HEAD, then bump.
  release <tag>        — resolve a caliptra-mcu-sw tag via
                         `git ls-remote --tags`, then bump, AND
                         set release_tag to the tag string.

NOTE: bzlmod forbids `load()` in MODULE.bazel, so this file is
NOT consumed by MODULE.bazel's caliptra_repos() calls directly.
uprev dual-writes — versions.bzl + Cargo.toml files + MODULE.bazel
are updated in lockstep by every bump/latest/release transaction,
and verify cross-checks them. This is documented in
plans/uprevpy.md under "bump / release / latest — dual-write path".
"""

CALIPTRA_VERSIONS = {
    "caliptra_cfi": "72c75dc70cde4120a9ce149d4f5cb21621399958",
    "caliptra_dpe": "f2b7121b69aa2bd05e69cee57f7a6993a07e5fef",
    "caliptra_mcu_sw": "8c9c8ff27de1babf63c0be74e973185837439040",
    "caliptra_sw": "b99535ded32492d6c23bbc56f95e787a1e275b59",
    "release_tag": "rt-sdk-2.1.1",
    "ureg": "4762ed5eef323a4267720a6b6da287c88738f901",
}
