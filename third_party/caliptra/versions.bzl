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
    "caliptra_cfi": "a98e499d279e81ae85881991b1e9eee354151189",
    "caliptra_dpe": "f56f66ef4ada62bd99b5670c8384dc2e97e04e94",
    "caliptra_mcu_sw": "b7e45fc139620754e8d32e70d6cc90845d1756df",
    "caliptra_sw": "2fe38a094bd06188714ff7c040252bc3059d9699",
    "release_tag": "",
    "ureg": "412ca40146d5d2012417e493b4a01096b04edf4b",
}
