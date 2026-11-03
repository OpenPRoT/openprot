# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Wraps a pw_kernel unit-test runner binary as a system image with tests on.

Pigweed's `system_image` rule forces `@pigweed//pw_kernel:enable_tests` off on
the kernel it wraps ("only unittests should ever enable tests"). That drops
`--cfg=test` from `KERNEL_TEST_RUSTC_FLAGS`, so every `#[cfg(test)]` module
behind `#[unittest::test]` is compiled out, no `.init_array` constructor
registers a test, and `unittest_core::run_all_tests!()` reports PASS over an
empty list. Pigweed's own unit-test runners are plain `rust_test` binaries that
never pass through `system_image`, so the flag set by `bazel test` reaches them.

The VeeR and Earl Grey harnesses consume a `SystemImageInfo` (ELF plus raw
`.bin`), so this rule produces one directly from the kernel ELF, compiled for
the given platform with `enable_tests` forced on. It is only meant for
unit-test runners; app-carrying images keep using `system_image`.
"""

load("@bazel_tools//tools/build_defs/cc:action_names.bzl", "ACTION_NAMES")
load("@bazel_tools//tools/cpp:toolchain_utils.bzl", "use_cpp_toolchain")
load("@pigweed//pw_build:binary_tools.bzl", "run_action_on_executable")
load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

def _unittest_kernel_transition_impl(_, attr):
    return {
        "//command_line_option:platforms": str(attr.platform),
        str(Label("@pigweed//pw_kernel:enable_tests")): True,
    }

_unittest_kernel_transition = transition(
    implementation = _unittest_kernel_transition_impl,
    inputs = [],
    outputs = [
        "//command_line_option:platforms",
        str(Label("@pigweed//pw_kernel:enable_tests")),
    ],
)

def _unittest_image_impl(ctx):
    kernel = ctx.file.kernel

    # Re-expose the kernel under a stable name so downstream tooling sees the
    # same `<name>.elf` / `<name>.bin` pair that `system_image` produces.
    output_elf = ctx.actions.declare_file(ctx.attr.name + ".elf")
    ctx.actions.symlink(output = output_elf, target_file = kernel)

    output_bin = ctx.actions.declare_file(ctx.attr.name + ".bin")
    run_action_on_executable(
        ctx = ctx,
        action_name = ACTION_NAMES.objcopy_embed_data,
        action_args = "-Obinary {input} {output}".format(
            input = kernel.path,
            output = output_bin.path,
        ),
        inputs = [kernel],
        output = output_bin,
        additional_outputs = [],
        output_executable = True,
    )

    return [
        DefaultInfo(files = depset([output_elf, output_bin])),
        SystemImageInfo(bin = output_bin, elf = output_elf),
    ]

unittest_image = rule(
    implementation = _unittest_image_impl,
    attrs = {
        "kernel": attr.label(
            doc = "The unit-test runner kernel binary.",
            allow_single_file = True,
            mandatory = True,
            cfg = _unittest_kernel_transition,
        ),
        "platform": attr.label(
            doc = "Bazel platform to compile the kernel for.",
            mandatory = True,
        ),
    },
    toolchains = use_cpp_toolchain(),
    fragments = ["cpp"],
    doc = "A pw_kernel unit-test runner image with kernel tests compiled in.",
)
