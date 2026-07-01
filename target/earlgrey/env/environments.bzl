# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("//target/earlgrey/tooling:qemu.bzl", "gen_flash")

TestEnvironmentInfo = provider(
    doc = "Provides information about a test environment.",
    fields = {
        "boot_cmd": "String: Command template to boot the firmware.",
        "interface": "String: The interface name (e.g., 'verilator', 'qemu', 'hyper310').",
        "need_flash": "Bool: True if the environment requires a flash image to be generated from the firmware.",
        "opentitantool_args": "List of strings: Extra global arguments for opentitantool.",
        "prepare": "Function: (ctx, env, firmware_bin, tools) -> struct(boot_image, extra_runfiles, output_groups)",
        "rom_ext": "File: Optional ROM extension binary to assemble with the firmware.",
        "runfiles": "Depset: Files needed at runtime by this environment.",
        "runner": "File: The executable to run if use_custom_runner is True.",
        "runner_args": "List of strings: Arguments for the custom runner.",
        "setup_cmds": "List of strings: Setup commands to run before the test.",
        "test_args": "List of strings: Extra arguments for opentitantool during the test phase.",
        "use_custom_runner": "Bool: True if this environment uses a custom runner instead of opentitantool.",
    },
)

def _qemu_prepare(ctx, env, firmware_bin, tools):
    flash_file = gen_flash(
        ctx,
        flashgen = tools.flashgen,
        firmware_bin = firmware_bin,
        firmware_elf = tools.elf,
    )
    return struct(
        boot_image = flash_file,
        extra_runfiles = [flash_file],
        output_groups = {"flash": depset([flash_file])},
    )

def _verilator_prepare(ctx, env, firmware_bin, tools):
    return struct(
        boot_image = firmware_bin,
        extra_runfiles = [],
        output_groups = {},
    )

def _fpga_prepare(ctx, env, firmware_bin, tools):
    if env.rom_ext:
        boot_image_file = ctx.actions.declare_file(ctx.attr.name + "_boot.img")
        ctx.actions.run(
            outputs = [boot_image_file],
            inputs = [env.rom_ext, firmware_bin],
            executable = tools.opentitantool,
            arguments = [
                "image",
                "assemble",
                "--mirror=false",
                "--output=" + boot_image_file.path,
                env.rom_ext.path + "@0",
                firmware_bin.path + "@0x10000",
            ],
            mnemonic = "OtpImageAssemble",
            progress_message = "Assembling boot image: %{output}",
        )
        return struct(
            boot_image = boot_image_file,
            extra_runfiles = [boot_image_file],
            output_groups = {"boot_image": depset([boot_image_file])},
        )
    return struct(
        boot_image = firmware_bin,
        extra_runfiles = [],
        output_groups = {},
    )

def _silicon_prepare(ctx, env, firmware_bin, tools):
    return struct(
        boot_image = firmware_bin,
        extra_runfiles = [],
        output_groups = {},
    )

def _qemu_environment_impl(ctx):
    qemu_bin = ctx.file.qemu_bin
    qemu_rom = ctx.file.qemu_rom
    qemu_start = ctx.file.qemu_start
    cfg_file = ctx.attr.qemu_cfg[DefaultInfo].files.to_list()[0]
    otp_file = ctx.attr.qemu_otp[DefaultInfo].files.to_list()[0]
    qemu_runner = ctx.executable.qemu_runner

    runfiles = depset(
        [qemu_bin, qemu_rom, qemu_start, cfg_file, otp_file],
        transitive = [ctx.attr.qemu_runner[DefaultInfo].default_runfiles.files],
    )

    runner_args = [
        "--qemu-start",
        qemu_start.short_path,
        "--qemu-bin",
        qemu_bin.short_path,
        "--qemu-config",
        cfg_file.short_path,
        "--qemu-rom",
        qemu_rom.short_path,
        "--qemu-otp",
        otp_file.short_path,
        "--qemu-flash",
        "{flash}",
        "--firmware-elf",
        "{elf}",
        "--icount",
        "6",
        "--timeout-seconds",
        "120",
        "--exit-success={exit_success}",
        "--exit-failure={exit_failure}",
    ]

    return [
        TestEnvironmentInfo(
            interface = "qemu",
            runfiles = runfiles,
            setup_cmds = [],
            boot_cmd = "",
            opentitantool_args = [],
            rom_ext = None,
            use_custom_runner = True,
            runner = qemu_runner,
            runner_args = runner_args,
            test_args = [],
            need_flash = True,
            prepare = _qemu_prepare,
        ),
    ]

qemu_environment = rule(
    implementation = _qemu_environment_impl,
    attrs = {
        "qemu_bin": attr.label(
            allow_single_file = True,
            cfg = "exec",
            default = "//third_party/qemu:qemu-system-riscv32",
        ),
        "qemu_cfg": attr.label(
            default = "//target/earlgrey/tooling:qemu_earlgrey_cfg",
        ),
        "qemu_otp": attr.label(
            default = "//target/earlgrey/tooling:qemu_rma_otp",
        ),
        "qemu_rom": attr.label(
            allow_single_file = True,
            cfg = "exec",
            default = "@opentitan_devbundle//:earlgrey/test_rom/test_rom_sim_verilator.elf",
        ),
        "qemu_runner": attr.label(
            executable = True,
            cfg = "exec",
            default = "//target/earlgrey/tooling:qemu_runner",
        ),
        "qemu_start": attr.label(
            allow_single_file = True,
            cfg = "exec",
            default = "//target/earlgrey/tooling:qemu_start.sh",
        ),
    },
)

def _verilator_environment_impl(ctx):
    verilator_bin = ctx.file.verilator_bin
    verilator_rom = ctx.file.verilator_rom
    verilator_otp = ctx.file.verilator_otp

    runfiles = depset([verilator_bin, verilator_rom, verilator_otp])

    test_args = [
        "--interface=verilator",
        "--verilator-bin={}".format(verilator_bin.short_path),
        "--verilator-rom={}".format(verilator_rom.short_path),
        "--verilator-otp={}".format(verilator_otp.short_path),
        "--verilator-flash={firmware}",
    ]

    return [
        TestEnvironmentInfo(
            interface = "verilator",
            runfiles = runfiles,
            setup_cmds = [],
            boot_cmd = "",
            opentitantool_args = [],
            rom_ext = None,
            use_custom_runner = False,
            runner = None,
            runner_args = [],
            test_args = test_args,
            need_flash = False,
            prepare = _verilator_prepare,
        ),
    ]

verilator_environment = rule(
    implementation = _verilator_environment_impl,
    attrs = {
        "verilator_bin": attr.label(
            allow_single_file = True,
            cfg = "exec",
            default = "@opentitan_devbundle//:earlgrey/verilator/Vchip_sim_tb",
        ),
        "verilator_otp": attr.label(
            allow_single_file = True,
            default = "@opentitan_devbundle//:earlgrey/otp/img_rma.24.vmem",
        ),
        "verilator_rom": attr.label(
            allow_single_file = True,
            default = "@opentitan_devbundle//:earlgrey/test_rom/test_rom_sim_verilator.39.scr.vmem",
        ),
    },
)

def _fpga_environment_impl(ctx):
    interface = ctx.attr.interface
    bitstream = ctx.file.bitstream
    rom_ext = ctx.file.rom_ext

    runfiles_list = []
    setup_cmds = []
    setup_cmds.append("transport init")

    if bitstream:
        runfiles_list.append(bitstream)
        setup_cmds.append("fpga load-bitstream {}".format(bitstream.short_path))

    runfiles = depset(runfiles_list)

    if rom_ext:
        boot_cmd = "bootstrap {boot_image}"
    else:
        boot_cmd = "rescue firmware {firmware}"

    return [
        TestEnvironmentInfo(
            interface = interface,
            runfiles = runfiles,
            setup_cmds = setup_cmds,
            boot_cmd = boot_cmd,
            opentitantool_args = ctx.attr.opentitantool_args,
            rom_ext = rom_ext,
            use_custom_runner = False,
            runner = None,
            runner_args = [],
            test_args = [],
            need_flash = False,
            prepare = _fpga_prepare,
        ),
    ]

fpga_environment = rule(
    implementation = _fpga_environment_impl,
    attrs = {
        "bitstream": attr.label(allow_single_file = True),
        "interface": attr.string(mandatory = True),
        "opentitantool_args": attr.string_list(),
        "rom_ext": attr.label(allow_single_file = True),
    },
)

def _silicon_environment_impl(ctx):
    interface = ctx.attr.interface

    return [
        TestEnvironmentInfo(
            interface = interface,
            runfiles = depset([]),
            setup_cmds = ["transport init"],
            boot_cmd = "rescue firmware {firmware}",
            opentitantool_args = ctx.attr.opentitantool_args,
            rom_ext = None,
            use_custom_runner = False,
            runner = None,
            runner_args = [],
            test_args = [],
            need_flash = False,
            prepare = _silicon_prepare,
        ),
    ]

silicon_environment = rule(
    implementation = _silicon_environment_impl,
    attrs = {
        "interface": attr.string(mandatory = True),
        "opentitantool_args": attr.string_list(),
    },
)
