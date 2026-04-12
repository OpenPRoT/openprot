# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Module extension for caliptra source repositories.

Declares @caliptra_sw and @caliptra_mcu_sw as git_repository rules
with SHAs pulled from versions.bzl at BUILD load time. The root
MODULE.bazel instantiates this extension via use_extension() and
lifts the repos into its label space via use_repo().

bzlmod forbids `load()` in MODULE.bazel directly, which is why
versions.bzl cannot be consumed from there. But `.bzl` files backing
module extensions CAN use `load()`, so this file reads versions.bzl
to keep the git_repository commits in lockstep with the canonical
pin dict.
"""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")
load("//third_party/caliptra:versions.bzl", "CALIPTRA_VERSIONS")

def _caliptra_repos_impl(module_ctx):
    git_repository(
        name = "caliptra_mcu_sw",
        build_file = "//third_party/caliptra/caliptra-mcu-sw:overlay.BUILD",
        commit = CALIPTRA_VERSIONS["caliptra_mcu_sw"],
        remote = "https://github.com/chipsalliance/caliptra-mcu-sw",
    )
    git_repository(
        name = "caliptra_sw",
        build_file = "//third_party/caliptra/caliptra-sw:overlay.BUILD",
        commit = CALIPTRA_VERSIONS["caliptra_sw"],
        remote = "https://github.com/chipsalliance/caliptra-sw",
        patches = ["//third_party/caliptra/caliptra-sw:caliptra_sw_rustcrypto.patch"],
        patch_args = ["-p0"],
    )

caliptra_repos = module_extension(
    implementation = _caliptra_repos_impl,
)
