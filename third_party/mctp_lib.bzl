# Licensed under the Apache-2.0 license

"""Module extension to fetch the mctp-lib Rust library."""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")

def _mctp_lib_impl(module_ctx):
    git_repository(
        name = "mctp_lib",
        remote = "https://github.com/OpenPRoT/mctp-lib.git",
        commit = "86aa1f6b902a2a9b5b78d4bbf12f0983722e044b",
        build_file = "@@//third_party:mctp_lib.BUILD.bazel",
    )
    return module_ctx.extension_metadata(
        reproducible = True,
        root_module_direct_deps = ["mctp_lib"],
        root_module_direct_dev_deps = [],
    )

mctp_lib_ext = module_extension(
    implementation = _mctp_lib_impl,
)
