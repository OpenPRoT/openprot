# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""A rule for running a pigweed system_image in a calptra-mcu development environment"""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

def _target_type_transition_impl(_, attr):
    if attr.interface == "fpga":
        return {"//target/veer:target_type": "fpga"}
    if attr.interface == "emulator":
        return {"//target/veer:target_type": "emulator"}

    return {"//target/veer:target_type": "silicon"}

_target_type_transition = transition(
    implementation = _target_type_transition_impl,
    inputs = [],
    outputs = ["//target/veer:target_type"],
)

def _caliptra_runner_impl(ctx):
    system_image_info = ctx.attr.target[0][SystemImageInfo]
    bin_file = system_image_info.bin
    elf_file = system_image_info.elf
    runner = ctx.executable._caliptra_runner

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
            "--mcu_image={},0x4000000,0,2,2".format(bin_file.path),
            "--caliptra_rom={}".format(ctx.file.caliptra_rom.path),
            "--caliptra_firmware={}".format(ctx.file.caliptra_firmware.path),
            "--vendor_pk_hash={}".format(ctx.attr.vendor_pk_hash),
            "--output={}".format(manifest.path),
        ],
    )

    run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")

    # TODO: currently, the caliptra rom, firmware and mcu-rom are hardcoded in the runner python script.
    # Perhaps these shoulc be arguments to the script instead.
    ctx.actions.write(
        output = run_script,
        is_executable = True,
        content = """#!/bin/bash
{runner} --interface {interface} --elf {elf} --bin {bin} --manifest {manifest} --vendor-pk-hash {vendor_pk_hash} {optional_args}
""".format(
            runner = runner.short_path,
            interface = ctx.attr.interface,
            elf = elf_file.short_path,
            bin = bin_file.short_path,
            manifest = manifest.short_path,
            vendor_pk_hash = ctx.attr.vendor_pk_hash,
            optional_args = "",
        ),
    )

    runner_files_depset = ctx.attr._caliptra_runner[DefaultInfo].default_runfiles.files

    return [DefaultInfo(
        runfiles = ctx.runfiles(
            files = [manifest, bin_file, runner],
            transitive_files = runner_files_depset,
        ),
        executable = run_script,
    )]

_BASE_ATTRS = {
    "interface": attr.string(
        doc = "The interface to use.",
        mandatory = True,
    ),
    "target": attr.label(
        doc = "The system_image target to run.",
        mandatory = True,
        providers = [SystemImageInfo],
        cfg = _target_type_transition,
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
        default = "@caliptra_devbundle//:tools/signer",
        doc = "caliptra signer",
    ),
    "caliptra_rom": attr.label(
        allow_single_file = True,
        default = "@caliptra_devbundle//:emulator/cptra-rom.bin",
        doc = "caliptra ROM",
    ),
    "caliptra_firmware": attr.label(
        allow_single_file = True,
        default = "@caliptra_devbundle//:emulator/cptra-firmware.bin",
        doc = "caliptra firmware",
    ),
    "vendor_pk_hash": attr.string(
        default = "b17ca877666657ccd100e6926c7206b60c995cb68992c6c9baefce728af05441dee1ff415adfc187e1e4edb4d3b2d909",
        doc = "SHA384 of vendor public key",
    ),
}

caliptra_runner = rule(
    implementation = _caliptra_runner_impl,
    executable = True,
    attrs = _BASE_ATTRS,
)

caliptra_test = rule(
    implementation = _caliptra_runner_impl,
    test = True,
    attrs = _BASE_ATTRS,
)
