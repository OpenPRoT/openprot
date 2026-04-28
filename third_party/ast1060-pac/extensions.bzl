# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Module extension for the ast1060-pac source repository."""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")
load("//third_party/ast1060-pac:versions.bzl", "AST1060_PAC_VERSIONS")

def _ast1060_pac_repos_impl(module_ctx):
    git_repository(
        name = "ast1060_pac",
        build_file = "//third_party/ast1060-pac:overlay.BUILD",
        commit = AST1060_PAC_VERSIONS["ast1060_pac"],
        remote = "https://github.com/OpenPRoT/ast1060-pac",
    )

ast1060_pac_repos = module_extension(
    implementation = _ast1060_pac_repos_impl,
)
