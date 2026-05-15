# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Bazel rule for stress test images.

Stress tests loop forever and only exit on TEST_RESULT:FAIL. This rule
produces a test target that relies on --run_under to supply the runner,
exactly like system_image_test. Use the stress_virt_ast10x0 or
stress_k_ast1060_evb configs which pass --no-timeout to the runner.
"""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

def _stress_image_test_impl(ctx):
    executable_symlink = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(
        output = executable_symlink,
        target_file = ctx.attr.image[SystemImageInfo].elf,
    )

    return [
        DefaultInfo(
            executable = executable_symlink,
            runfiles = ctx.attr.image[DefaultInfo].default_runfiles,
        ),
    ]

stress_image_test = rule(
    implementation = _stress_image_test_impl,
    test = True,
    attrs = {
        "image": attr.label(
            doc = "The system_image target to test.",
            mandatory = True,
            providers = [DefaultInfo, SystemImageInfo],
            executable = True,
            cfg = "target",
        ),
    },
    doc = "Defines a stress test for a system_image. Use with stress_virt_ast10x0 or stress_k_ast1060_evb config.",
)
