# Licensed under the Apache-2.0 license

"""Module extension to fetch the AST1060 PAC and its dependency vcell."""

load("@bazel_tools//tools/build_defs/repo:git.bzl", "git_repository")
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def _ast1060_pac_impl(module_ctx):
    # vcell 0.1.2 — tiny VolatileCell crate required by svd2rust PACs.
    # Not in Pigweed's crate universe, so we fetch it directly.
    http_archive(
        name = "vcell",
        urls = ["https://crates.io/api/v1/crates/vcell/0.1.2/download"],
        type = "tar.gz",
        strip_prefix = "vcell-0.1.2",
        build_file = "@@//third_party:vcell.BUILD.bazel",
        sha256 = "876e32dcadfe563a4289e994f7cb391197f362b6315dc45e8ba4aa6f564a4b3c",
    )

    git_repository(
        name = "ast1060_pac",
        remote = "https://github.com/AspeedTech-BMC/ast1060-pac.git",
        commit = "35ce8190e9b40deff918300b69d23079ca15a3f4",
        build_file = "@@//third_party:ast1060_pac.BUILD.bazel",
    )
    return module_ctx.extension_metadata(
        reproducible = True,
        root_module_direct_deps = ["ast1060_pac", "vcell"],
        root_module_direct_dev_deps = [],
    )

ast1060_pac_ext = module_extension(
    implementation = _ast1060_pac_impl,
)
