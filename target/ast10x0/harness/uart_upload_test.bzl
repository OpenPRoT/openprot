# Licensed under the Apache-2.0 license

"""Bazel rules for UART test execution on AST1060 hardware.

Provides test rules that upload firmware via UART and monitor test execution.
"""

load(
    "@bazel_skylib//rules:common_settings.bzl",
    "BuildSettingInfo",
)

load(
    "@pigweed//pw_kernel/tooling:system_image.bzl",
    "SystemImageInfo",
)

def _firmware_bin(ctx):
    if SystemImageInfo in ctx.attr.image:
        return ctx.attr.image[SystemImageInfo].bin
    return ctx.file.image

def _declare_uart_boot_image(ctx, firmware_bin, name):
    output = ctx.actions.declare_file(name + ".bin")
    ctx.actions.run_shell(
        inputs = [firmware_bin],
        outputs = [output],
        arguments = [firmware_bin.path, output.path],
        mnemonic = "Ast10x0UartBootImage",
        command = """set -eu
input=\"$1\"
output=\"$2\"
size=$(wc -c < \"$input\")
aligned=$(( (size + 3) & ~3 ))

emit_byte() {
    printf '%b' "$(printf '\\%03o' \"$1\")"
}

{
    emit_byte $((aligned & 255))
    emit_byte $(((aligned >> 8) & 255))
    emit_byte $(((aligned >> 16) & 255))
    emit_byte $(((aligned >> 24) & 255))
    cat \"$input\"
    padding=$((aligned - size))
    if [ \"$padding\" -gt 0 ]; then
        dd if=/dev/zero bs=1 count=\"$padding\" status=none
    fi
} > \"$output\"
""",
    )
    return output

def _uart_boot_image_impl(ctx):
    firmware_bin = _firmware_bin(ctx)
    if ctx.attr._uart_boot_header[BuildSettingInfo].value:
        output = _declare_uart_boot_image(ctx, firmware_bin, ctx.label.name)
    else:
        output = firmware_bin
    return [
        DefaultInfo(files = depset([output])),
    ]

uart_boot_image = rule(
    implementation = _uart_boot_image_impl,
    attrs = {
        "image": attr.label(
            mandatory = True,
            doc = "system_image or binary target to wrap with the AST10x0 UART boot header",
        ),
        "_uart_boot_header": attr.label(
            default = "//target/ast10x0:uart_boot_header",
            providers = [BuildSettingInfo],
        ),
    },
    doc = "Generate an AST10x0 UART boot image by prepending the 4-byte size header when enabled by config.",
)

def _uart_upload_test_impl(ctx):
    """Implementation of uart_upload_test rule."""

    # Get the firmware binary
    firmware_bin = _firmware_bin(ctx)

    # Create test script
    test_script = ctx.actions.declare_file(ctx.label.name + "_test.sh")

    # Build the command line arguments
    args = []
    
    if ctx.attr.baudrate:
        args.extend(["--baudrate", str(ctx.attr.baudrate)])
    
    if ctx.attr.test_timeout:
        args.extend(["--test-timeout", str(ctx.attr.test_timeout)])
    
    if ctx.attr.srst_pin:
        args.extend(["--srst-pin", str(ctx.attr.srst_pin)])
    
    if ctx.attr.fwspick_pin:
        args.extend(["--fwspick-pin", str(ctx.attr.fwspick_pin)])
    
    if ctx.attr.skip_gpio:
        args.append("--skip-gpio")
    
    if ctx.attr.upload_only:
        args.append("--upload-only")

    # Get path to the Python script 
    python_script = ctx.file._uart_test_exec

    script_content = """#!/bin/bash
set -e

# Allow overriding UART device via environment
UART_DEVICE="${{UART_DEVICE:-{default_device}}}"

# Check if device exists (unless skipping device check)
if [[ ! -e "$UART_DEVICE" && -z "$SKIP_DEVICE_CHECK" ]]; then
    echo "ERROR: UART device not found: $UART_DEVICE"
    echo "Set UART_DEVICE environment variable or connect hardware"
    exit 1
fi

# Find the Python script in runfiles
SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
RUNFILES="${{SCRIPT_DIR}}/{test_name}_test.sh.runfiles/_main"

if [[ -f "$RUNFILES/{script_path}" ]]; then
    PYTHON_SCRIPT="$RUNFILES/{script_path}"
elif [[ -f "{script_path}" ]]; then
    PYTHON_SCRIPT="{script_path}"
else
    echo "ERROR: Could not find uart_test_exec.py" >&2
    exit 1
fi

# Find firmware in runfiles
if [[ -f "$RUNFILES/{firmware_path}" ]]; then
    FIRMWARE="$RUNFILES/{firmware_path}"
elif [[ -f "{firmware_path}" ]]; then
    FIRMWARE="{firmware_path}"
else
    echo "ERROR: Could not find firmware: {firmware_path}" >&2
    exit 1
fi

# Run the UART test executor
exec python3 "$PYTHON_SCRIPT" "$UART_DEVICE" "$FIRMWARE" {args}
""".format(
        default_device = ctx.attr.uart_device or "/dev/ttyUSB0",
        test_name = ctx.label.name,
        script_path = python_script.short_path,
        firmware_path = firmware_bin.short_path,
        args = " ".join(args),
    )

    ctx.actions.write(
        output = test_script,
        content = script_content,
        is_executable = True,
    )

    runfiles = ctx.runfiles(
        files = [firmware_bin, python_script],
    )

    return [
        DefaultInfo(
            executable = test_script,
            runfiles = runfiles,
        ),
    ]

uart_upload_test = rule(
    implementation = _uart_upload_test_impl,
    test = True,
    attrs = {
        "image": attr.label(
            mandatory = True,
            allow_single_file = True,
            doc = "system_image or uart_boot_image target to upload",
        ),
        "uart_device": attr.string(
            default = "",
            doc = "UART device path (default: /dev/ttyUSB0 or UART_DEVICE env var)",
        ),
        "baudrate": attr.int(
            default = 115200,
            doc = "UART baud rate",
        ),
        "test_timeout": attr.int(
            default = 600,
            doc = "Test execution timeout in seconds",
        ),
        "srst_pin": attr.int(
            default = 23,
            doc = "SRST GPIO pin number",
        ),
        "fwspick_pin": attr.int(
            default = 18,
            doc = "FWSPICK GPIO pin number",
        ),
        "skip_gpio": attr.bool(
            default = False,
            doc = "Skip GPIO operations (for pre-configured boards)",
        ),
        "upload_only": attr.bool(
            default = False,
            doc = "Upload firmware only, skip test monitoring",
        ),
        "_uart_test_exec": attr.label(
            default = "//target/ast10x0/harness:uart_test_exec.py",
            allow_single_file = [".py"],
        ),
    },
    doc = """Run a firmware test on AST1060 hardware via UART.

This test rule:
1. Optionally enters FWSPICK mode via GPIO
2. Uploads firmware via UART bootloader
3. Monitors serial output for test pass/fail

Environment variables:
- UART_DEVICE: Override the UART device path
- SKIP_DEVICE_CHECK: Skip device existence check (for CI)

Usage:
    load(
        "//target/ast10x0/harness:uart_upload_test.bzl",
        "uart_boot_image",
        "uart_upload_test",
    )

    # AST1030-EVB UART boot expects the 4-byte size header.
    uart_boot_image(
        name = "threads_uart",
        image = ":threads",
    )

    uart_upload_test(
        name = "threads_uart_test",
        image = ":threads_uart",
        test_timeout = 300,
    )

Run with:
    bazel test //target/ast1060-evb/threads/kernel:threads_uart_test \\
        --test_env=UART_DEVICE=/dev/ttyUSB0
""",
)

def _uart_upload_impl(ctx):
    """Implementation of uart_upload rule (non-test, just upload)."""

    # Get the firmware binary
    firmware_bin = _firmware_bin(ctx)

    # Create upload script
    upload_script = ctx.actions.declare_file(ctx.label.name + "_upload.sh")

    args = ["--upload-only"]
    
    if ctx.attr.baudrate:
        args.extend(["--baudrate", str(ctx.attr.baudrate)])
    
    if ctx.attr.skip_gpio:
        args.append("--skip-gpio")

    # Get path to the Python script
    python_script = ctx.file._uart_test_exec

    script_content = """#!/bin/bash
set -e

UART_DEVICE="${{UART_DEVICE:-{default_device}}}"

if [[ ! -e "$UART_DEVICE" ]]; then
    echo "ERROR: UART device not found: $UART_DEVICE"
    exit 1
fi

# Find the Python script in runfiles
SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
RUNFILES="${{SCRIPT_DIR}}/{script_name}_upload.sh.runfiles/_main"

if [[ -f "$RUNFILES/{script_path}" ]]; then
    PYTHON_SCRIPT="$RUNFILES/{script_path}"
elif [[ -f "{script_path}" ]]; then
    PYTHON_SCRIPT="{script_path}"
else
    echo "ERROR: Could not find uart_test_exec.py" >&2
    exit 1
fi

# Find firmware in runfiles
if [[ -f "$RUNFILES/{firmware_path}" ]]; then
    FIRMWARE="$RUNFILES/{firmware_path}"
elif [[ -f "{firmware_path}" ]]; then
    FIRMWARE="{firmware_path}"
else
    echo "ERROR: Could not find firmware: {firmware_path}" >&2
    exit 1
fi

echo "Uploading firmware to $UART_DEVICE..."
exec python3 "$PYTHON_SCRIPT" "$UART_DEVICE" "$FIRMWARE" {args}
""".format(
        default_device = ctx.attr.uart_device or "/dev/ttyUSB0",
        script_name = ctx.label.name,
        script_path = python_script.short_path,
        firmware_path = firmware_bin.short_path,
        args = " ".join(args),
    )

    ctx.actions.write(
        output = upload_script,
        content = script_content,
        is_executable = True,
    )

    runfiles = ctx.runfiles(
        files = [firmware_bin, python_script],
    )

    return [
        DefaultInfo(
            executable = upload_script,
            runfiles = runfiles,
        ),
    ]

uart_upload = rule(
    implementation = _uart_upload_impl,
    executable = True,
    attrs = {
        "image": attr.label(
            mandatory = True,
            allow_single_file = True,
            doc = "system_image or uart_boot_image target to upload",
        ),
        "uart_device": attr.string(
            default = "",
            doc = "UART device path (default: /dev/ttyUSB0 or UART_DEVICE env var)",
        ),
        "baudrate": attr.int(
            default = 115200,
            doc = "UART baud rate",
        ),
        "skip_gpio": attr.bool(
            default = False,
            doc = "Skip GPIO operations",
        ),
        "_uart_test_exec": attr.label(
            default = "//target/ast10x0/harness:uart_test_exec.py",
            allow_single_file = [".py"],
        ),
    },
    doc = """Upload firmware to AST10x0 hardware via UART.

This is a non-test rule that just uploads firmware without monitoring.

Usage:
    bazel run //target/ast1060-evb/threads/kernel:upload_threads -- \\
        UART_DEVICE=/dev/ttyUSB0
""",
)
