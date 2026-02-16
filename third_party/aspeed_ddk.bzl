# Licensed under the Apache-2.0 license

"""Module extension to fetch the aspeed-ddk crate."""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")

def _aspeed_ddk_impl(module_ctx):
    git_repository(
        name = "aspeed_ddk",
        remote = "https://github.com/OpenPRoT/aspeed-rust.git",
        branch = "i2c-core",
        build_file = "@@//third_party:aspeed_ddk.BUILD.bazel",
    )
    return module_ctx.extension_metadata(
        reproducible = True,
        root_module_direct_deps = ["aspeed_ddk"],
        root_module_direct_dev_deps = [],
    )

aspeed_ddk_ext = module_extension(
    implementation = _aspeed_ddk_impl,
)
