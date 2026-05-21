# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
#
# Three pure-function Bazel rules for generating QEMU backing files:
#   qemu_cfg   — QEMU readconfig INI (from RTL secrets via cfggen.py)
#   qemu_otp   — QEMU OTP raw image  (from a .vmem via otptool.py)
#   qemu_flash — QEMU flash image    (from a firmware .bin via flashgen.py)
#
# Ported from opentitan/rules/opentitan/qemu.bzl.  Stripped: ExecEnvInfo,
# sim_qemu, qemu_params, _test_dispatch, _transform.  Those belong to
# opentitan's exec-env system which openprot does not use (see plan §"exec_env
# is unused").
#
# Security note (build-side): all file paths flow through ctx.actions.run()
# argument lists, never through shell interpolation.  Attack surface here is
# Bazel rule invocation by BUILD authors, not external input.  No runtime
# user-controlled data crosses this boundary.

# ---------------------------------------------------------------------------
# Helper functions — call these from within a rule's own ctx to generate a
# single backing file.  Each takes explicit arguments rather than ctx
# attribute lookup, so callers (including opentitan_runner.bzl) can supply
# values from their own ctx without attribute-name collisions.
# ---------------------------------------------------------------------------

def gen_cfg(ctx, cfggen, qemu_constants):
    """Generate a QEMU readconfig INI file containing OpenTitan RTL secrets from JSON.

    Args:
        ctx:            Rule context.
        cfggen:         Target providing qemu_cfg_gen.py executable (DefaultInfo).
        qemu_constants: File — qemu_constants.json.

    Returns:
        The declared output File (.ini).
    """
    out = ctx.actions.declare_file(ctx.label.name + ".ini")
    ctx.actions.run(
        inputs = [qemu_constants],
        outputs = [out],
        executable = cfggen[DefaultInfo].files_to_run,
        arguments = [
            "--json",
            qemu_constants.path,
            "--out",
            out.path,
        ],
        mnemonic = "QemuCfgGen",
    )
    return out

def gen_otp(ctx, otptool, vmem):
    """Generate a QEMU-compatible raw OTP image from a .vmem file.

    Args:
        ctx:     Rule context.
        otptool: Target providing otptool.py executable (DefaultInfo).
        vmem:    File — OTP .vmem image (e.g. img_rma.vmem).

    Returns:
        The declared output File (.raw).
    """
    out = ctx.actions.declare_file(ctx.label.name + ".raw")
    ctx.actions.run(
        inputs = [vmem],
        outputs = [out],
        executable = otptool[DefaultInfo].files_to_run,
        arguments = [
            "-m",
            vmem.path,
            "-r",
            out.path,
            "-k",
            "otp",
        ],
        mnemonic = "QemuOtpGen",
    )
    return out

def gen_flash(ctx, flashgen, firmware_bin, firmware_elf = None):
    """Generate a QEMU-compatible flash backing image from a firmware binary.

    NOTE: only single-bank flash images are supported (mirrors opentitan).
    The firmware binary is placed at offset 0x0 of the flash image.

    Args:
        ctx:          Rule context.
        flashgen:     Target providing flashgen.py executable (DefaultInfo).
        firmware_bin: File — flat binary to splice into the flash image.
        firmware_elf: File or None — ELF for mtime sanity checks (unused by
                      QEMU itself; pass None to skip checks via --unsafe-elf).

    Returns:
        The declared output File (.qemu_bin).
    """
    out = ctx.actions.declare_file(ctx.label.name + ".qemu_bin")
    args = [
        "-t",
        "{}@0x0".format(firmware_bin.path),
    ]

    # When an ELF is present, pass --ignore-time so Bazel's mtime rewriting
    # does not cause spurious "binary older than ELF" failures.  When absent,
    # pass --unsafe-elf to skip ELF size validation entirely.
    if firmware_elf:
        args += ["--ignore-time"]
    else:
        args += ["--unsafe-elf"]

    args += [out.path]

    inputs = [firmware_bin]
    if firmware_elf:
        inputs.append(firmware_elf)

    ctx.actions.run(
        inputs = inputs,
        outputs = [out],
        executable = flashgen[DefaultInfo].files_to_run,
        arguments = args,
        mnemonic = "QemuFlashGen",
    )
    return out

# ---------------------------------------------------------------------------
# Standalone rules — invoke these directly from a BUILD file to materialize
# the backing files as named Bazel targets.
# ---------------------------------------------------------------------------

def _qemu_cfg_impl(ctx):
    out = gen_cfg(
        ctx,
        cfggen = ctx.attr.cfggen,
        qemu_constants = ctx.file.qemu_constants,
    )
    return [DefaultInfo(files = depset([out]))]

qemu_cfg = rule(
    implementation = _qemu_cfg_impl,
    attrs = {
        "cfggen": attr.label(
            executable = True,
            cfg = "exec",
            allow_files = True,
            default = Label("//target/earlgrey/tooling:qemu_cfg_gen"),
            doc = "qemu_cfg_gen.py py_binary target.",
        ),
        "qemu_constants": attr.label(
            allow_single_file = True,
            default = Label("//third_party/opentitan_rtl:qemu_constants"),
            doc = "qemu_constants.json — QEMU constants in JSON format.",
        ),
    },
)

def _qemu_otp_impl(ctx):
    out = gen_otp(
        ctx,
        otptool = ctx.attr.otptool,
        vmem = ctx.file.vmem,
    )
    return [DefaultInfo(files = depset([out]))]

qemu_otp = rule(
    implementation = _qemu_otp_impl,
    attrs = {
        "otptool": attr.label(
            executable = True,
            cfg = "exec",
            allow_files = True,
            default = Label("//third_party/qemu:otptool"),
            doc = "otptool.py py_binary target.",
        ),
        "vmem": attr.label(
            allow_single_file = True,
            default = Label("@opentitan_devbundle//:earlgrey/otp/img_rma.24.vmem"),
            doc = "OTP .vmem image to convert (defaults to RMA image).",
        ),
    },
)

def _qemu_flash_impl(ctx):
    out = gen_flash(
        ctx,
        flashgen = ctx.attr.flashgen,
        firmware_bin = ctx.file.firmware_bin,
        firmware_elf = ctx.file.firmware_elf,
    )
    return [DefaultInfo(files = depset([out]))]

qemu_flash = rule(
    implementation = _qemu_flash_impl,
    attrs = {
        "firmware_bin": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "Flat firmware binary to splice into the flash image at offset 0x0.",
        ),
        "firmware_elf": attr.label(
            allow_single_file = True,
            doc = "ELF counterpart for mtime sanity checks (optional; omit for unsigned builds).",
        ),
        "flashgen": attr.label(
            executable = True,
            cfg = "exec",
            allow_files = True,
            default = Label("//third_party/qemu:flashgen"),
            doc = "flashgen.py py_binary target.",
        ),
    },
)
