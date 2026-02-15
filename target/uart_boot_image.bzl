# Licensed under the Apache-2.0 license

"""Bazel rule for generating UART boot images with size header.

The AST1060 UART bootloader expects a 4-byte little-endian size header
followed by the binary image, padded to 4-byte alignment.
"""

load(
    "@pigweed//pw_kernel/tooling:system_image.bzl",
    "SystemImageInfo",
)

def _uart_boot_image_impl(ctx):
    output = ctx.outputs.out
    
    # Get the bin file from SystemImageInfo if available, otherwise use src
    if SystemImageInfo in ctx.attr.src:
        input_bin = ctx.attr.src[SystemImageInfo].bin
    else:
        input_bin = ctx.file.src

    # Shell script to create UART boot header
    ctx.actions.run_shell(
        inputs = [input_bin],
        outputs = [output],
        command = """
            set -e
            src="$1"
            dst="$2"
            src_sz=$(wc -c < "$src")
            src_sz_align=$(( (($src_sz + 3) / 4) * 4 ))
            # Write 4-byte little-endian size header
            printf "0: %.8x" $src_sz_align | sed -E 's/0: (..)(..)(..)(..)/0: \\4\\3\\2\\1/' | xxd -r -g0 > "$dst"
            # Append binary
            dd if="$src" of="$dst" bs=1 seek=4 2>/dev/null
            # Pad to alignment
            padding=$(( $src_sz_align - $src_sz ))
            if [ $padding -gt 0 ]; then
                dd if=/dev/zero bs=1 count=$padding >> "$dst" 2>/dev/null
            fi
        """,
        arguments = [input_bin.path, output.path],
        mnemonic = "UartBootImage",
        progress_message = "Generating UART boot image %{output}",
    )

    return [DefaultInfo(files = depset([output]))]

uart_boot_image = rule(
    implementation = _uart_boot_image_impl,
    attrs = {
        "src": attr.label(
            mandatory = True,
            doc = "Input system_image target or binary file (.bin)",
        ),
        "out": attr.output(
            mandatory = True,
            doc = "Output UART boot image file",
        ),
    },
    doc = """Prepends a 4-byte little-endian size header to a binary image.

The AST1060 UART bootloader protocol expects:
  - 4 bytes: image size (little-endian, 4-byte aligned)
  - N bytes: binary image data
  - 0-3 bytes: zero padding to 4-byte alignment

Usage:
    load("//target:uart_boot_image.bzl", "uart_boot_image")

    uart_boot_image(
        name = "threads_uart",
        src = ":threads",  # system_image target
        out = "threads_uart.bin",
    )
""",
)
