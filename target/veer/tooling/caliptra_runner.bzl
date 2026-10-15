# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""A rule for running a pigweed system_image in a calptra-mcu development environment"""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

def _target_type_transition_impl(_, attr):
    # target_type selects the image's memory map; //third_party/caliptra:platform
    # selects the matching Caliptra ROM/FW and MCU ROM variants.
    if attr.interface == "fpga":
        return {
            "//target/veer:target_type": "fpga",
            "//third_party/caliptra:platform": "fpga",
        }
    if attr.interface == "emulator":
        return {
            "//target/veer:target_type": "emulator",
            "//third_party/caliptra:platform": "emulator",
        }

    return {
        "//target/veer:target_type": "emulator",
        "//third_party/caliptra:platform": "emulator",
    }

_target_type_transition = transition(
    implementation = _target_type_transition_impl,
    inputs = [],
    outputs = [
        "//target/veer:target_type",
        "//third_party/caliptra:platform",
    ],
)

def _mcu_rom_transition_impl(settings, attr):
    return {"//command_line_option:platforms": ["//third_party/caliptra/platforms:caliptra"]}

_mcu_rom_transition = transition(
    implementation = _mcu_rom_transition_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

def _caliptra_runner_impl(ctx):
    system_image_info = ctx.attr.target[SystemImageInfo]
    bin_file = system_image_info.bin
    elf_file = system_image_info.elf
    runner = ctx.executable._caliptra_runner

    platform = "fpga" if ctx.attr.interface == "fpga" else "emulator"
    manifest = ctx.actions.declare_file("{}.manifest".format(ctx.attr.name))
    ctx.actions.run(
        inputs = [
            bin_file,
            ctx.file.caliptra_rom,
            ctx.file.caliptra_firmware,
        ],
        outputs = [manifest],
        executable = ctx.executable._signer,
        env = {"RUST_BACKTRACE": "1"},
        arguments = [
            "auth-manifest",
            "create",
            "--platform={}".format(platform),
            "--mcu_image={}".format(bin_file.path),
            "--caliptra_rom={}".format(ctx.file.caliptra_rom.path),
            "--caliptra_firmware={}".format(ctx.file.caliptra_firmware.path),
            "--vendor_pk_hash={}".format(ctx.attr.vendor_pk_hash),
            "--output={}".format(manifest.path),
        ],
    )

    run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")

    ctx.actions.write(
        output = run_script,
        is_executable = True,
        content = """#!/bin/bash
{runner} --interface {interface} --elf {elf} --bin {bin} --manifest {manifest} --caliptra-rom {caliptra_rom} --caliptra-firmware {caliptra_firmware} --mcu-rom {mcu_rom} --vendor-pk-hash {vendor_pk_hash} {optional_args}
""".format(
            runner = runner.short_path,
            interface = ctx.attr.interface,
            elf = elf_file.short_path,
            bin = bin_file.short_path,
            manifest = manifest.short_path,
            caliptra_rom = ctx.file.caliptra_rom.short_path,
            caliptra_firmware = ctx.file.caliptra_firmware.short_path,
            mcu_rom = ctx.file.mcu_rom.short_path,
            vendor_pk_hash = ctx.attr.vendor_pk_hash,
            optional_args = "",
        ),
    )

    runner_files_depset = ctx.attr._caliptra_runner[DefaultInfo].default_runfiles.files

    return [DefaultInfo(
        runfiles = ctx.runfiles(
            files = [
                manifest,
                bin_file,
                runner,
                ctx.file.caliptra_rom,
                ctx.file.caliptra_firmware,
                ctx.file.mcu_rom,
            ],
            transitive_files = runner_files_depset,
        ),
        executable = run_script,
    )]

_BASE_ATTRS = {
    "caliptra_firmware": attr.label(
        allow_single_file = True,
        default = "//target/veer/tooling:caliptra_firmware_bin_transitioned",
        doc = "caliptra firmware",
    ),
    "caliptra_rom": attr.label(
        allow_single_file = True,
        default = "//target/veer/tooling:caliptra_rom_transitioned",
        doc = "caliptra ROM",
    ),
    "interface": attr.string(
        doc = "The interface to use.",
        mandatory = True,
    ),
    "mcu_rom": attr.label(
        allow_single_file = True,
        default = "//target/veer/tooling:mcu_rom_bin_transitioned",
        doc = "MCU ROM",
    ),
    "target": attr.label(
        doc = "The system_image target to run.",
        mandatory = True,
        providers = [SystemImageInfo],
    ),
    "vendor_pk_hash": attr.string(
        default = "b17ca877666657ccd100e6926c7206b60c995cb68992c6c9baefce728af05441dee1ff415adfc187e1e4edb4d3b2d909",
        doc = "SHA384 of vendor public key",
    ),
    "_caliptra_runner": attr.label(
        executable = True,
        cfg = "exec",
        default = "//target/veer/tooling:caliptra_runner",
    ),
    "_signer": attr.label(
        executable = True,
        allow_single_file = True,
        cfg = "exec",
        default = "//third_party/caliptra/caliptra-mcu-sw:signer",
        doc = "caliptra signer",
    ),
}

caliptra_runner = rule(
    implementation = _caliptra_runner_impl,
    executable = True,
    attrs = _BASE_ATTRS,
    cfg = _target_type_transition,
)

caliptra_test = rule(
    implementation = _caliptra_runner_impl,
    test = True,
    attrs = _BASE_ATTRS,
    cfg = _target_type_transition,
)

def _mcu_rom_wrapper_impl(ctx):
    files = ctx.attr.target[0][DefaultInfo].files.to_list()
    return [DefaultInfo(files = depset(files))]

mcu_rom_wrapper = rule(
    implementation = _mcu_rom_wrapper_impl,
    attrs = {
        "target": attr.label(cfg = _mcu_rom_transition),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
