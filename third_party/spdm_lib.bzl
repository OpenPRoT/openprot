# Licensed under the Apache-2.0 license

"""Module extension to fetch the spdm-lib Rust library."""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")

def _spdm_lib_impl(module_ctx):
    git_repository(
        name = "spdm_lib",
        remote = "https://github.com/9elements/spdm-lib.git",
        commit = "3cd80a56980fef9cc6eb1c5dc980fb36574f6e8c",
        build_file = "@@//third_party:spdm_lib.BUILD.bazel",
    )
    return module_ctx.extension_metadata(
        reproducible = True,
        root_module_direct_deps = ["spdm_lib"],
        root_module_direct_dev_deps = [],
    )

spdm_lib_ext = module_extension(
    implementation = _spdm_lib_impl,
)
