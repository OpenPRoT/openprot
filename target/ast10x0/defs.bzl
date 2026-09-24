# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Common definitions used by all ast10x0 targets."""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

TARGET_COMPATIBLE_WITH = select({
    "//target/ast10x0:target_ast10x0": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

def _system_image_test_impl(ctx):
    image_info = ctx.attr.image[SystemImageInfo]
    executable_symlink = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = executable_symlink, target_file = image_info.elf)

    bin_symlink = ctx.actions.declare_file(ctx.label.name + ".bin")
    ctx.actions.symlink(output = bin_symlink, target_file = image_info.bin)

    runfiles = ctx.runfiles(files = [bin_symlink]).merge(
        ctx.attr.image[DefaultInfo].default_runfiles,
    )

    if ctx.attr.slave_image:
        slave_info = ctx.attr.slave_image[SystemImageInfo]
        slave_elf_symlink = ctx.actions.declare_file(ctx.label.name + ".slave.elf")
        ctx.actions.symlink(output = slave_elf_symlink, target_file = slave_info.elf)
        slave_bin_symlink = ctx.actions.declare_file(ctx.label.name + ".slave.bin")
        ctx.actions.symlink(output = slave_bin_symlink, target_file = slave_info.bin)
        runfiles = ctx.runfiles(files = [slave_elf_symlink, slave_bin_symlink]).merge(
            runfiles.merge(ctx.attr.slave_image[DefaultInfo].default_runfiles),
        )

    return [DefaultInfo(
        executable = executable_symlink,
        runfiles = runfiles,
    )]

def _flash_system_image_test_impl(ctx):
    default_info = _system_image_test_impl(ctx)[0]
    providers = [default_info]

    # fmc_model describes the QEMU FMC device uniformly (JEDEC ID + SFDP
    # geometry), shared by both chip selects. The qemu_runner seeds fresh
    # images at $TEST_TMPDIR/<name> and attaches each present CS image as
    # FMC flash (if=mtd): cs0_image at index 0, cs1_image at index 1.
    env = {
        "AST10X0_FLASH_SIZE": str(ctx.attr.flash_size),
        "AST10X0_FMC_MODEL": ctx.attr.fmc_model,
    }
    if ctx.attr.cs0_image:
        env["AST10X0_CS0_IMAGE"] = ctx.attr.cs0_image
        env["AST10X0_CS0_FILL"] = str(ctx.attr.cs0_fill)
    if ctx.attr.cs1_image:
        env["AST10X0_CS1_IMAGE"] = ctx.attr.cs1_image
        env["AST10X0_CS1_FILL"] = str(ctx.attr.cs1_fill)
    if ctx.attr.cs0_image or ctx.attr.cs1_image:
        providers.append(RunEnvironmentInfo(environment = env))
    return providers

system_image_test = rule(
    implementation = _system_image_test_impl,
    test = True,
    attrs = {
        "image": attr.label(
            doc = "The system_image target to test.",
            mandatory = True,
            providers = [SystemImageInfo],
            executable = True,
            cfg = "target",
        ),
        "slave_image": attr.label(
            doc = "Optional slave system_image for paired two-device tests.",
            mandatory = False,
            default = None,
            providers = [SystemImageInfo],
            cfg = "target",
        ),
    },
)

flash_system_image_test = rule(
    implementation = _flash_system_image_test_impl,
    test = True,
    attrs = {
        "cs0_fill": attr.int(
            doc = "Byte value the qemu_runner seeds cs0_image with (default 0xFF, erased).",
            default = 0xFF,
        ),
        "cs0_image": attr.string(
            doc = "Basename of the SPI-NOR image seeded in $TEST_TMPDIR and " +
                  "attached as FMC CS0 flash (if=mtd, index=0).",
            default = "",
        ),
        "cs1_fill": attr.int(
            doc = "Byte value the qemu_runner seeds cs1_image with (default 0xFF, erased).",
            default = 0xFF,
        ),
        "cs1_image": attr.string(
            doc = "Basename of the SPI-NOR image seeded in $TEST_TMPDIR and " +
                  "attached as FMC CS1 flash (if=mtd, index=1).",
            default = "",
        ),
        "flash_size": attr.int(
            doc = "Size in bytes of each seeded flash image.",
            default = 8 * 1024 * 1024,
        ),
        "fmc_model": attr.string(
            doc = "QEMU fmc-model applied controller-wide (JEDEC ID + SFDP " +
                  "geometry). Default is the 1 MiB w25q80bl.",
            default = "w25q80bl",
        ),
        "image": attr.label(
            doc = "The system_image target to test.",
            mandatory = True,
            providers = [SystemImageInfo],
            executable = True,
            cfg = "target",
        ),
        "slave_image": attr.label(
            doc = "Optional slave system_image for paired two-device tests.",
            mandatory = False,
            default = None,
            providers = [SystemImageInfo],
            cfg = "target",
        ),
    },
)
