# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")
load("//target/earlgrey/tooling:qemu.bzl", "gen_flash")
load("//target/earlgrey/tooling/signing:defs.bzl", "KeySetInfo", "sign_binary")

def _target_type_transition_impl(_, attr):
    if attr.interface == "hyper310" or attr.interface == "hyper340":
        return {"//target/earlgrey:target_type": "fpga"}
    if attr.interface == "verilator":
        return {"//target/earlgrey:target_type": "verilator"}
    if attr.interface == "qemu":
        return {"//target/earlgrey:target_type": "qemu"}

    return {"//target/earlgrey:target_type": "silicon"}

_target_type_transition = transition(
    implementation = _target_type_transition_impl,
    inputs = [],
    outputs = ["//target/earlgrey:target_type"],
)

def _opentitan_runner_impl(ctx):
    system_image_info = ctx.attr.target[0][SystemImageInfo]
    elf_file = system_image_info.elf
    bin_file = system_image_info.bin

    opentitantool = ctx.executable._opentitantool

    test_harness = opentitantool
    is_custom_harness = False
    if ctx.attr.test_harness:
        test_harness = ctx.executable.test_harness
        is_custom_harness = True

    if ctx.attr.ecdsa_key:
        result = sign_binary(
            ctx,
            opentitantool,
            bin = bin_file,
            basename = ctx.attr.name,
        )
        bin_file = result["signed"]

    run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")
    runfiles_list = [elf_file, bin_file, opentitantool]
    if is_custom_harness:
        runfiles_list.append(test_harness)

    exit_success = ctx.attr.exit_success if hasattr(ctx.attr, "exit_success") else "PASS\\n"
    exit_failure = ctx.attr.exit_failure if hasattr(ctx.attr, "exit_failure") else "FAIL: .+\\n"

    if ctx.attr.interface == "qemu":
        flash_file = gen_flash(
            ctx,
            flashgen = ctx.attr._flashgen,
            firmware_bin = bin_file,
            firmware_elf = elf_file,
        )

        cfg_file = ctx.attr._qemu_cfg[DefaultInfo].files.to_list()[0]
        otp_file = ctx.attr._qemu_otp[DefaultInfo].files.to_list()[0]
        qemu_bin = ctx.file._qemu_bin
        qemu_rom = ctx.file._qemu_rom
        qemu_start = ctx.file._qemu_start
        qemu_runner_exe = ctx.executable._qemu_runner

        ctx.actions.write(
            output = run_script,
            is_executable = True,
            content = """#!/bin/bash
exec {runner} \
  --qemu-start {qemu_start} \
  --qemu-bin {qemu_bin} \
  --qemu-config {cfg} \
  --qemu-rom {rom} \
  --qemu-otp {otp} \
  --qemu-flash {flash} \
  --firmware-elf {elf} \
  --icount 6 \
  --timeout-seconds 120 \
  --exit-success='{exit_success}' \
  --exit-failure='{exit_failure}'
""".format(
                runner = qemu_runner_exe.short_path,
                qemu_start = qemu_start.short_path,
                qemu_bin = qemu_bin.short_path,
                cfg = cfg_file.short_path,
                rom = qemu_rom.short_path,
                otp = otp_file.short_path,
                flash = flash_file.short_path,
                elf = elf_file.short_path,
                exit_success = exit_success,
                exit_failure = exit_failure,
            ),
        )

        qemu_runner_runfiles = ctx.attr._qemu_runner[DefaultInfo].default_runfiles.files

        return [DefaultInfo(
            runfiles = ctx.runfiles(
                files = runfiles_list + [qemu_bin, qemu_rom, qemu_start, cfg_file, otp_file, flash_file],
                transitive_files = qemu_runner_runfiles,
            ),
            executable = run_script,
        )]

    elif ctx.attr.interface == "verilator":
        verilator_bin = ctx.file._verilator_bin
        verilator_rom = ctx.file._verilator_rom
        verilator_otp = ctx.file._verilator_otp

        runfiles_list.extend([verilator_bin, verilator_rom, verilator_otp])

        if ctx.attr.test_cmd:
            test_cmd_part = ctx.attr.test_cmd
        else:
            test_cmd_part = "console --non-interactive --exit-success='{}' --exit-failure='{}'".format(exit_success, exit_failure)

        ctx.actions.write(
            output = run_script,
            is_executable = True,
            content = """#!/bin/bash
echo {opentitantool} \
  --rcfile= \
  --interface=verilator \
  --verilator-bin={verilator_bin} \
  --verilator-rom={rom} \
  --verilator-otp={otp} \
  --verilator-flash={firmware} \
  {test_cmd} "$@"

exec {opentitantool} \
  --rcfile= \
  --interface=verilator \
  --verilator-bin={verilator_bin} \
  --verilator-rom={rom} \
  --verilator-otp={otp} \
  --verilator-flash={firmware} \
  {test_cmd} "$@"
""".format(
                opentitantool = opentitantool.short_path,
                verilator_bin = verilator_bin.short_path,
                rom = verilator_rom.short_path,
                otp = verilator_otp.short_path,
                firmware = bin_file.short_path,
                test_cmd = test_cmd_part,
            ),
        )
        return [DefaultInfo(
            runfiles = ctx.runfiles(files = runfiles_list),
            executable = run_script,
        )]

    else:
        bitstream = None
        rom_ext = None

        if ctx.attr.interface == "hyper310":
            bitstream = ctx.file._bitstream_hyper310
            rom_ext = ctx.file._rom_ext_cw310
        elif ctx.attr.interface == "hyper340":
            bitstream = ctx.file._bitstream_hyper340
            rom_ext = ctx.file._rom_ext_cw340

        setup_cmds = []
        setup_cmds.append('exec="transport init"')

        if bitstream:
            runfiles_list.append(bitstream)
            setup_cmds.append('exec="fpga load-bitstream {}"'.format(bitstream.short_path))

        if rom_ext:
            runfiles_list.append(rom_ext)
            boot_img = "boot_image.img"
            setup_cmds.append('exec="image assemble --mirror=false --output={boot_img} {rom_ext}@0 {firmware}@0x10000"'.format(
                boot_img = boot_img,
                rom_ext = rom_ext.short_path,
                firmware = bin_file.short_path,
            ))
            setup_cmds.append("bootstrap {}".format(boot_img))
        else:
            setup_cmds.append("rescue firmware {}".format(bin_file.short_path))

        setup_args_list = []
        for c in setup_cmds:
            if c.startswith("exec="):
                setup_args_list.append("--" + c)
            else:
                setup_args_list.append(c)
        setup_args_str = " ".join(setup_args_list)

        if is_custom_harness:
            test_exec = test_harness.short_path
            test_args = "--interface={interface}".format(
                interface = ctx.attr.interface,
            )
            if ctx.attr.test_cmd:
                test_args += " " + ctx.attr.test_cmd
        else:
            test_exec = opentitantool.short_path
            test_args = "--rcfile= --interface={interface}".format(interface = ctx.attr.interface)
            if ctx.attr.test_cmd:
                test_args += " " + ctx.attr.test_cmd
            else:
                test_args += " console --non-interactive --exit-success='{}' --exit-failure='{}'".format(exit_success, exit_failure)

        ctx.actions.write(
            output = run_script,
            is_executable = True,
            content = """#!/bin/bash
set -e

echo "=== Performing Board Setup ==="
echo {opentitantool} --rcfile= --interface={interface} {setup_args}
{opentitantool} --rcfile= --interface={interface} {setup_args}

echo "=== Running Test ==="
echo {test_exec} {test_args} "$@"
exec {test_exec} {test_args} "$@"
""".format(
                opentitantool = opentitantool.short_path,
                interface = ctx.attr.interface,
                setup_args = setup_args_str,
                test_exec = test_exec,
                test_args = test_args,
            ),
        )
        return [DefaultInfo(
            runfiles = ctx.runfiles(files = runfiles_list),
            executable = run_script,
        )]

_BASE_ATTRS = {
    "ecdsa_key": attr.label_keyed_string_dict(
        allow_files = True,
        providers = [KeySetInfo],
        doc = "ECDSA public key to validate this image",
    ),
    "interface": attr.string(
        values = ["hyper310", "hyper340", "qemu", "teacup", "verilator"],
        mandatory = True,
    ),
    "manifest": attr.label(
        allow_single_file = True,
        doc = "A json manifest to apply to the image being signed",
    ),
    "spx_key": attr.label_keyed_string_dict(
        allow_files = True,
        providers = [KeySetInfo],
        doc = "SPX public key to validate this image",
    ),
    "target": attr.label(
        doc = "The system_image target to run.",
        mandatory = True,
        providers = [SystemImageInfo],
        cfg = _target_type_transition,
    ),
    "test_cmd": attr.string(
        default = "",
        doc = "Custom command to run after setup (for FPGA/Silicon) or instead of console (for Verilator)",
    ),
    "test_harness": attr.label(
        executable = True,
        cfg = "exec",
        doc = "Alternative test harness binary",
    ),
    "_bitstream_hyper310": attr.label(
        allow_single_file = True,
        default = "@opentitan_devbundle//:earlgrey/bitstreams/bitstream_fpga_hyper310.bit",
    ),
    "_bitstream_hyper340": attr.label(
        allow_single_file = True,
        default = "@opentitan_devbundle//:earlgrey/bitstreams/bitstream_fpga_hyper340.bit",
    ),
    "_opentitantool": attr.label(
        executable = True,
        allow_single_file = True,
        cfg = "exec",
        default = "//third_party/lowrisc_opentitan:opentitantool",
        doc = "opentitantool",
    ),
    "_rom_ext_cw310": attr.label(
        allow_single_file = True,
        default = "@opentitan_devbundle//:rom_ext/rom_ext_dice_x509_slot_virtual_fpga_cw310.prod_key_0.prod_key_0.signed.bin",
    ),
    "_rom_ext_cw340": attr.label(
        allow_single_file = True,
        default = "@opentitan_devbundle//:rom_ext/rom_ext_dice_x509_slot_virtual_fpga_cw340.prod_key_0.prod_key_0.signed.bin",
    ),
    "_verilator_bin": attr.label(
        allow_single_file = True,
        cfg = "exec",
        default = "@opentitan_devbundle//:earlgrey/verilator/Vchip_sim_tb",
    ),
    "_verilator_otp": attr.label(
        allow_single_file = True,
        default = "@opentitan_devbundle//:earlgrey/otp/img_rma.24.vmem",
    ),
    "_verilator_rom": attr.label(
        allow_single_file = True,
        default = "@opentitan_devbundle//:earlgrey/test_rom/test_rom_sim_verilator.39.scr.vmem",
    ),
}

_QEMU_ATTRS = {
    "_flashgen": attr.label(
        executable = True,
        cfg = "exec",
        default = "//third_party/qemu:flashgen",
    ),
    "_qemu_bin": attr.label(
        allow_single_file = True,
        cfg = "exec",
        default = "//third_party/qemu:qemu-system-riscv32",
    ),
    "_qemu_cfg": attr.label(
        default = "//target/earlgrey/tooling:qemu_earlgrey_cfg",
    ),
    "_qemu_otp": attr.label(
        default = "//target/earlgrey/tooling:qemu_rma_otp",
    ),
    "_qemu_rom": attr.label(
        allow_single_file = True,
        cfg = "exec",
        default = "@opentitan_devbundle//:earlgrey/test_rom/test_rom_sim_verilator.elf",
    ),
    "_qemu_runner": attr.label(
        executable = True,
        cfg = "exec",
        default = "//target/earlgrey/tooling:qemu_runner",
    ),
    "_qemu_start": attr.label(
        allow_single_file = True,
        cfg = "exec",
        default = "//target/earlgrey/tooling:qemu_start.sh",
    ),
}

opentitan_runner = rule(
    implementation = _opentitan_runner_impl,
    executable = True,
    attrs = _BASE_ATTRS,
)

opentitan_test = rule(
    implementation = _opentitan_runner_impl,
    test = True,
    attrs = _BASE_ATTRS | _QEMU_ATTRS | {
        "exit_failure": attr.string(
            default = "FAIL: .+\\n",
            doc = "The regex to look for in the output to determine failure.",
        ),
        "exit_success": attr.string(
            default = "PASS\\n",
            doc = "The regex to look for in the output to determine success.",
        ),
    },
)
