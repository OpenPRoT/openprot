# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Common definitions used by all veer targets.
"""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "system_image")
load("//target/veer/tooling:caliptra_runner.bzl", "caliptra_mcu_image")

TARGET_COMPATIBLE_WITH = select({
    "//target/veer:target_veer": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

def veer_system_config(name, src, **kwargs):
    """Renders a pw_kernel system config for the selected target type.

    `src` is a system config template containing `@KERNEL_FLASH_START@`,
    `@KERNEL_RAM_START@` and/or `@MCI_BASE@`. They are filled in from upstream
    caliptra-mcu-sw's EMULATOR_MEMORY_MAP / FPGA_MEMORY_MAP by
    //target/veer/tooling:render_system_config. The output is `<name>.json5`.
    """
    tool = "//target/veer/tooling:render_system_config"
    cmd = "$(execpath {}) {{}} $(execpath {}) $@".format(tool, src)
    native.genrule(
        name = name,
        srcs = [src],
        outs = [name + ".json5"],
        cmd = select({
            "//target/veer:fpga": cmd.format("fpga"),
            "//conditions:default": cmd.format("emulator"),
        }),
        tools = [tool],
        **kwargs
    )

def veer_system_image(name, visibility = None, tags = None, **kwargs):
    """Defines a Pigweed `system_image` plus its signed `<name>_image_emulator` and `<name>_image_fpga` `caliptra_mcu_image` targets."""
    system_image(
        name = name,
        tags = tags,
        visibility = visibility,
        **kwargs
    )
    caliptra_mcu_image(
        name = name + "_image_emulator",
        image = ":" + name,
        interface = "emulator",
        tags = tags,
        visibility = visibility,
    )
    caliptra_mcu_image(
        name = name + "_image_fpga",
        image = ":" + name,
        interface = "fpga",
        tags = tags,
        visibility = visibility,
    )
