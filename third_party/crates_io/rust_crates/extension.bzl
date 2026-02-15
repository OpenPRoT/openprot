# Licensed under the Apache-2.0 license
"""Out-of-tree crates_io hub that routes crates based on no_std/std constraint.

This extension creates the @rust_crates// repository that aliases crates to
either @oot_crates_no_std// (for embedded) or @rules_rust++crate+rust_crates//
(for host builds).
"""

def _crates_io_hub_impl(ctx):
    build_file_path = ctx.path(ctx.attr._alias_hub_build_file)
    ctx.watch(build_file_path)
    build_file_contents = ctx.read(build_file_path)
    ctx.file("BUILD.bazel", content = build_file_contents)

_crates_io_hub = repository_rule(
    implementation = _crates_io_hub_impl,
    attrs = {
        "_alias_hub_build_file": attr.label(
            allow_single_file = True,
            default = "//third_party/crates_io/rust_crates:alias_hub.BUILD",
        ),
    },
)

def _oot_rust_crates_extension_impl(_ctx):
    _crates_io_hub(name = "rust_crates")

oot_rust_crates_extension = module_extension(
    implementation = _oot_rust_crates_extension_impl,
)
