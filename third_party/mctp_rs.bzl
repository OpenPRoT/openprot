# Licensed under the Apache-2.0 license

"""Module extension to fetch the CodeConstruct mctp-rs crates (mctp + mctp-estack)."""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")

def _mctp_rs_impl(module_ctx):
    git_repository(
        name = "mctp_rs",
        remote = "https://github.com/CodeConstruct/mctp-rs.git",
        commit = "9e52b626863b916d900ca1ddfdd9215baf0f80fc",
        build_file = "@@//third_party:mctp_rs.BUILD.bazel",
    )
    return module_ctx.extension_metadata(
        reproducible = True,
        root_module_direct_deps = ["mctp_rs"],
        root_module_direct_dev_deps = [],
    )

mctp_rs_ext = module_extension(
    implementation = _mctp_rs_impl,
)
