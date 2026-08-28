# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

def _cli_transition_impl(settings, attr):
    return {
        "//target/earlgrey/firmware/hwe:cli": True,
    }

_cli_transition = transition(
    implementation = _cli_transition_impl,
    inputs = [],
    outputs = ["//target/earlgrey/firmware/hwe:cli"],
)

def _cli_system_image_impl(ctx):
    actual = ctx.attr.target[0]
    actual_info = actual[SystemImageInfo]

    elf_symlink = ctx.actions.declare_file(ctx.label.name + ".elf")
    ctx.actions.symlink(output = elf_symlink, target_file = actual_info.elf)

    bin_symlink = ctx.actions.declare_file(ctx.label.name + ".bin")
    ctx.actions.symlink(output = bin_symlink, target_file = actual_info.bin)

    return [
        DefaultInfo(
            files = depset([elf_symlink, bin_symlink]),
            runfiles = ctx.runfiles(files = [bin_symlink, elf_symlink]),
        ),
        SystemImageInfo(
            bin = bin_symlink,
            elf = elf_symlink,
            apps = actual_info.apps,
        ),
    ]

cli_system_image = rule(
    implementation = _cli_system_image_impl,
    attrs = {
        "target": attr.label(
            cfg = _cli_transition,
            mandatory = True,
            providers = [SystemImageInfo],
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
