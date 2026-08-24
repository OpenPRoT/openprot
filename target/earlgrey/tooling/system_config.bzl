# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Rule to generate a modified system.json5 configuration variant."""

def _system_config_variant_impl(ctx):
    out = ctx.actions.declare_file(ctx.attr.name + ".json5")
    args = [
        "--input",
        ctx.file.base.path,
        "--output",
        out.path,
    ]
    if ctx.attr.flash_start_address:
        args.extend(["--flash-start-address", ctx.attr.flash_start_address])

    ctx.actions.run(
        inputs = [ctx.file.base],
        outputs = [out],
        executable = ctx.executable._tool,
        arguments = args,
        mnemonic = "SystemConfigVariant",
    )
    return [DefaultInfo(files = depset([out]))]

system_config_variant = rule(
    implementation = _system_config_variant_impl,
    attrs = {
        "base": attr.label(
            doc = "Base system.json5 file",
            allow_single_file = True,
            mandatory = True,
        ),
        "flash_start_address": attr.string(
            doc = "Override for kernel.flash_start_address (e.g. '0xA0016000')",
        ),
        "_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = "//target/earlgrey/tooling:system_config_override",
        ),
    },
    doc = "Generates a modified system.json5 configuration variant",
)
