# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Common definitions used by all ast10x0 targets."""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

TARGET_COMPATIBLE_WITH = select({
    "//target/ast10x0:target_ast10x0": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

def _kernel_build_transition_impl(_settings, _attr):
    return {str(Label("@pigweed//pw_kernel/userspace:is_app_build")): False}

# system_image's kernel transition clears userspace_build but leaves
# is_app_build alone, so an image reached from inside an app build keeps the
# userspace log backend and pulls in syscall_user, which userspace_build =
# False marks incompatible. Clear the flag before entering the image.
_kernel_build_transition = transition(
    implementation = _kernel_build_transition_impl,
    inputs = [],
    outputs = [str(Label("@pigweed//pw_kernel/userspace:is_app_build"))],
)

def _system_image_bin_impl(ctx):
    # A transitioned label attr arrives as a list, one entry per output config.
    return DefaultInfo(files = depset([ctx.attr.image[0][SystemImageInfo].bin]))

system_image_bin = rule(
    implementation = _system_image_bin_impl,
    doc = "Exposes a system_image's raw .bin on its own, so a genrule can " +
          "consume it without also picking up the .elf.",
    attrs = {
        "image": attr.label(
            doc = "The system_image to take the .bin from.",
            mandatory = True,
            providers = [SystemImageInfo],
            cfg = _kernel_build_transition,
        ),
    },
)

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

    # Images whose code runs on a board but which are never uploaded by the
    # harness, so their format strings would otherwise be missing from the
    # detokenizer and print as $base64.
    for i, img in enumerate(getattr(ctx.attr, "token_images", [])):
        token_elf_symlink = ctx.actions.declare_file(
            "%s.tokens%d.elf" % (ctx.label.name, i),
        )
        ctx.actions.symlink(
            output = token_elf_symlink,
            target_file = img[SystemImageInfo].elf,
        )
        runfiles = ctx.runfiles(files = [token_elf_symlink]).merge(runfiles)

    providers = [DefaultInfo(
        executable = executable_symlink,
        runfiles = runfiles,
    )]

    # Absent on flash_system_image_test, which shares this implementation.
    env = {}
    if getattr(ctx.attr, "reboot_from_flash", False):
        env["AST10X0_REBOOT_FROM_FLASH"] = "1"
    if getattr(ctx.attr, "slave_stages_to_flash", False):
        env["AST10X0_SLAVE_STAGES_TO_FLASH"] = "1"
    if env:
        providers.append(RunEnvironmentInfo(environment = env))
    return providers

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
        "reboot_from_flash": attr.bool(
            doc = "After the first verdict, restart the board with FWSPICK low " +
                  "and demand a second verdict from the image in flash.",
            default = False,
        ),
        "slave_image": attr.label(
            doc = "Optional slave system_image for paired two-device tests.",
            mandatory = False,
            default = None,
            providers = [SystemImageInfo],
            cfg = "target",
        ),
        "slave_stages_to_flash": attr.bool(
            doc = "The slave image only stages a payload into its own boot " +
                  "flash, so restart it with FWSPICK low once it reports and " +
                  "take the real verdict from what boots.",
            default = False,
        ),
        "token_images": attr.label_list(
            doc = "Extra system_images whose logs appear on a UART but which " +
                  "the harness never uploads, such as a payload carried by " +
                  "the slave image. Only their tokens are used.",
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
