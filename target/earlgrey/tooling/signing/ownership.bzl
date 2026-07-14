# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

def _owner_block_binary_impl(ctx):
    output = ctx.attr.output
    if not output:
        output = ctx.attr.name + ".bin"
    out_bin = ctx.actions.declare_file(output)
    inputs = [ctx.file.src] + ctx.files.keys

    command = "set -euo pipefail"

    if ctx.attr.relative_to:
        relative_to_files = ctx.attr.relative_to.files.to_list()
        if not relative_to_files:
            fail("relative_to target has no files")
        ref_file = relative_to_files[0]
        workspace_root = ref_file.owner.workspace_root

        if workspace_root:
            command += """
            WORKSPACE_ROOT="{workspace_root}"
            if [ -n "$WORKSPACE_ROOT" ]; then
              for f in $WORKSPACE_ROOT/*; do
                name=$(basename "$f")
                if [ ! -e "$name" ]; then
                  ln -s "$f" "$name"
                fi
              done
            fi
            """.format(workspace_root = workspace_root)
            inputs.append(ref_file)

    command += """
    {opentitantool} --rcfile= \
        ownership config --input {input} {output}
    """.format(
        opentitantool = ctx.executable.opentitantool.path,
        input = ctx.file.src.path,
        output = out_bin.path,
    )

    ctx.actions.run_shell(
        inputs = inputs,
        outputs = [out_bin],
        command = command,
        tools = [ctx.executable.opentitantool],
        use_default_shell_env = True,
        mnemonic = "OwnerBlockGen",
        progress_message = "Generating owner block binary %s" % ctx.attr.name,
    )

    return [DefaultInfo(files = depset([out_bin]))]

owner_block_binary = rule(
    implementation = _owner_block_binary_impl,
    attrs = {
        "keys": attr.label_list(allow_files = True),
        "opentitantool": attr.label(
            default = Label("@opentitan_devbundle//:opentitantool/opentitantool"),
            allow_single_file = True,
            executable = True,
            cfg = "exec",
        ),
        "output": attr.string(doc = "Optional output filename"),
        "relative_to": attr.label(allow_single_file = True),
        "src": attr.label(allow_single_file = True, mandatory = True),
    },
)

def _ownership_detached_signature_impl(ctx):
    out_sig = ctx.actions.declare_file(ctx.attr.output or (ctx.attr.name + ".bin"))

    inputs = []
    args = [
        "--rcfile=",
        "--quiet",
        "ownership",
        "detached-signature",
        "--command={}".format(ctx.attr.command),
        "--key-alg={}".format(ctx.attr.key_alg),
        "--nonce={}".format(ctx.attr.nonce),
    ]

    if ctx.file.src:
        inputs.append(ctx.file.src)
        args.append("--input={}".format(ctx.file.src.path))
    if ctx.file.ecdsa_key:
        inputs.append(ctx.file.ecdsa_key)
        args.append("--ecdsa-key={}".format(ctx.file.ecdsa_key.path))
    if ctx.file.spx_key:
        inputs.append(ctx.file.spx_key)
        args.append("--spx-key={}".format(ctx.file.spx_key.path))
    if ctx.file.ecdsa_sig:
        inputs.append(ctx.file.ecdsa_sig)
        args.append("--ecdsa-sig={}".format(ctx.file.ecdsa_sig.path))
    if ctx.file.spx_sig:
        inputs.append(ctx.file.spx_sig)
        args.append("--spx-sig={}".format(ctx.file.spx_sig.path))

    args.append(out_sig.path)

    ctx.actions.run(
        outputs = [out_sig],
        inputs = inputs,
        arguments = args,
        executable = ctx.executable.opentitantool,
        mnemonic = "OwnershipDetachedSignatureGen",
        progress_message = "Generating ownership detached signature %s" % ctx.attr.name,
    )

    return [DefaultInfo(files = depset([out_sig]))]

ownership_detached_signature = rule(
    implementation = _ownership_detached_signature_impl,
    attrs = {
        "command": attr.string(mandatory = True, doc = "Command: Owner, Unlock, Activate"),
        "ecdsa_key": attr.label(allow_single_file = True, doc = "ECDSA private key file in DER format"),
        "ecdsa_sig": attr.label(allow_single_file = True, doc = "Raw ECDSA signature file"),
        "key_alg": attr.string(mandatory = True, doc = "Key algorithm: EcdsaP256, SpxPure, SpxPrehash, HybridSpxPure, HybridSpxPrehash"),
        "nonce": attr.string(default = "0", doc = "Nonce value as string (due to int limits)"),
        "opentitantool": attr.label(
            default = Label("@opentitan_devbundle//:opentitantool/opentitantool"),
            allow_single_file = True,
            executable = True,
            cfg = "exec",
        ),
        "output": attr.string(doc = "Optional output signature filename"),
        "spx_key": attr.label(allow_single_file = True, doc = "SPHINCS+ private key file in PEM format"),
        "spx_sig": attr.label(allow_single_file = True, doc = "Raw SPX signature file"),
        "src": attr.label(allow_single_file = True, doc = "Raw data block to sign (e.g. owner_block_binary)"),
    },
)
