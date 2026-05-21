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
    runner = ctx.executable._opentitan_runner

    if ctx.attr.ecdsa_key:
        result = sign_binary(
            ctx,
            ctx.executable._opentitantool,
            bin = bin_file,
            basename = ctx.attr.name,
        )
        bin_file = result["signed"]

    optional_args = ""
    if hasattr(ctx.attr, "exit_success") and ctx.attr.exit_success:
        optional_args += " --exit-success='{}'".format(ctx.attr.exit_success)
    if hasattr(ctx.attr, "exit_failure") and ctx.attr.exit_failure:
        optional_args += " --exit-failure='{}'".format(ctx.attr.exit_failure)

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

        run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")
        ctx.actions.write(
            output = run_script,
            is_executable = True,
            content = """#!/bin/bash
{runner} \
  --qemu-start {qemu_start} \
  --qemu-bin {qemu_bin} \
  --qemu-config {cfg} \
  --qemu-rom {rom} \
  --qemu-otp {otp} \
  --qemu-flash {flash} \
  --firmware-elf {elf} \
  --icount 6 \
  --timeout-seconds 120 \
  {optional_args}
""".format(
                runner = qemu_runner_exe.short_path,
                qemu_start = qemu_start.short_path,
                qemu_bin = qemu_bin.short_path,
                cfg = cfg_file.short_path,
                rom = qemu_rom.short_path,
                otp = otp_file.short_path,
                flash = flash_file.short_path,
                elf = elf_file.short_path,
                optional_args = optional_args,
            ),
        )

        qemu_runner_runfiles = ctx.attr._qemu_runner[DefaultInfo].default_runfiles.files

        return [DefaultInfo(
            runfiles = ctx.runfiles(
                files = [elf_file, bin_file, qemu_bin, qemu_rom, qemu_start, cfg_file, otp_file, flash_file],
                transitive_files = qemu_runner_runfiles,
            ),
            executable = run_script,
        )]

    if ctx.attr.interface == "hyper310" or ctx.attr.interface == "hyper340":
        load_bitstream = "--load-bitstream"
        mechanism = "--mechanism=bootstrap"
    elif ctx.attr.interface == "teacup":
        load_bitstream = ""
        mechanism = "--mechanism=rescue"
    else:
        load_bitstream = ""
        mechanism = ""

    run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")
    ctx.actions.write(
        output = run_script,
        is_executable = True,
        content = """#!/bin/bash
{runner} --interface {interface} {load_bitstream} {mechanism} --elf {elf} --bin {bin} {optional_args}
""".format(
            runner = runner.short_path,
            interface = ctx.attr.interface,
            load_bitstream = load_bitstream,
            mechanism = mechanism,
            elf = elf_file.short_path,
            bin = bin_file.short_path,
            optional_args = optional_args,
        ),
    )

    runner_files_depset = ctx.attr._opentitan_runner[DefaultInfo].default_runfiles.files

    return [DefaultInfo(
        runfiles = ctx.runfiles(
            files = [elf_file, bin_file, runner],
            transitive_files = runner_files_depset,
        ),
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
    "_opentitan_runner": attr.label(
        executable = True,
        cfg = "exec",
        default = "//target/earlgrey/tooling:opentitan_runner",
    ),
    "_opentitantool": attr.label(
        executable = True,
        allow_single_file = True,
        cfg = "exec",
        default = "@opentitan_devbundle//:opentitantool/opentitantool",
        doc = "opentitantool",
    ),
}

# Hidden attrs for the qemu interface.  Kept separate from _BASE_ATTRS because
# several of these deps are testonly=True; merging them into _BASE_ATTRS would
# prevent non-test opentitan_runner targets from loading without errors.
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
