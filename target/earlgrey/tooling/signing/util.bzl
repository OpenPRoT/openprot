# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@bazel_skylib//lib:paths.bzl", "paths")

SigningToolInfo = provider(fields = ["tool", "data", "env", "location"])
KeySetInfo = provider(fields = ["keys", "config", "selected_key", "profile", "sign", "tool"])

def get_override(obj, item, overrides):
    """Get an item from obj unless it exists in overrides.

    Args:
      obj: The object holding the item.
      item: An object path to the desired item (ie: `attr.srcs`).
      overrides: A dict that may contain an override named by the last
                 component of the item path (ie: `srcs`).
    """
    items = item.split(".")
    item = items[-1]
    if item in overrides:
        return overrides.get(item)
    for i in items:
        obj = getattr(obj, i)
    return obj

def label_str(label):
    return "@{}//{}:{}".format(label.workspace_name, label.package, label.name)

def key_from_dict(key, attr_name):
    """Extract the key information from the `key` dict.

    Args:
        key: dict; A signing key and nickname or a keyset and key nickname.
        attr_name: The attribute name (used for error reporting).
    Returns:
        A struct with the key label, the key file and key nickname.
    """
    if not key:
        return None
    if len(key) == 0:
        return None
    if len(key) != 1:
        fail("Expected exactly one key/value pair for attribute", attr_name)
    key, name = key.items()[0]
    if "/" in name or "." in name or ":" in name:
        fail("Invalid key nickname for ", str(key), ".  Nickname ", name, " is invalid.")
    if KeySetInfo in key:
        ksi = key[KeySetInfo]
        if ksi.selected_key:
            name = ksi.selected_key
        elif name.isdigit():
            # If the nickname is a number, we assume it is the index of the key in
            # the dictionary.
            name = int(name)
            name = ksi.keys.keys()[name]
        return struct(
            label = key,
            file = ksi.keys[name],
            name = name,
            info = None,
            config = ksi.config[name],
        )
    elif DefaultInfo in key:
        key_file = key[DefaultInfo].files.to_list()
        if len(key_file) != 1:
            fail("Expected label to refer to exactly one file:", key)
        return struct(
            label = key,
            file = key_file[0],
            name = name,
            config = {},
        )
    return None

def signing_tool_info(ctx, key, opentitantool):
    """Returns the signing tool information for a given key.

    Args:
        ctx: The rule context object.
        key: The key dict attribute.
        opentitantool: A reference to opentitantool.
    Returns:
        (SigningToolInfo, signing function, profile)
    """
    key, _ = key.items()[0]
    if KeySetInfo in key:
        ksi = key[KeySetInfo]
        return ksi.tool, ksi.sign, ksi.profile
    elif DefaultInfo in key:
        toolinfo = SigningToolInfo(
            tool = opentitantool,
            data = [],
            env = {},
            location = "local",
        )
        return toolinfo, local_sign, None
    fail("Expected a KeySetInfo or DefaultInfo provider")

def key_ext(ecdsa, rsa, spx):
    """Returns the key extension for a given key.

    Args:
        ecdsa: struct; The ECDSA key.
        rsa: struct; The RSA key.
        spx: struct; The SPX+ key.
    Returns:
        str: The key extension.
    """
    if ecdsa:
        name = ecdsa.name
    elif rsa:
        name = rsa.name
    else:
        fail("Expected an ECDSA or RSA key")

    if spx:
        return ".{}.{}".format(name, spx.name)
    else:
        return ".{}".format(name)

def local_sign(ctx, tool, digest, ecdsa_key, rsa_key, spxmsg = None, spx_key = None, profile = None):
    """Sign a digest with a local on-disk RSA private key.

    Args:
        ctx: The rule context.
        tool: SigningToolInfo; A provider refering to the opentitantool binary.
        digest: file; The digest of the binary to be signed.
        ecdsa_key: struct; The ECDSA private key.
        rsa_key: struct; The RSA private key.
        spxmsg: file; The SPX+ message to be signed.
        spx_key: struct; The SPX+ private key.
        profile: str; The token profile.  Not used by this function.
    Returns:
        file, file, file: The ECDSA, RSA and SPX signature files.
    """
    if rsa_key and ecdsa_key:
        fail("Only one of ECDSA or RSA key should be provided")

    inputs = [digest]
    if rsa_key:
        output_sig = ctx.actions.declare_file(paths.replace_extension(digest.basename, ".rsa_sig"))
        inputs.append(rsa_key.file)
        key_path = rsa_key.file.path
        key_command = "rsa"
    elif ecdsa_key:
        output_sig = ctx.actions.declare_file(paths.replace_extension(digest.basename, ".ecdsa_sig"))
        inputs.append(ecdsa_key.file)
        key_path = ecdsa_key.file.path
        key_command = "ecdsa"
    else:
        fail("Expected an ECDSA or RSA key")

    ctx.actions.run(
        outputs = [output_sig],
        inputs = inputs,
        arguments = [
            "--rcfile=",
            "--quiet",
            key_command,
            "sign",
            "--input={}".format(digest.path),
            "--output={}".format(output_sig.path),
            key_path,
        ],
        executable = tool.tool,
        mnemonic = "LocalRsaOrEcdsaSign",
    )

    spx_sig = None
    if spxmsg and spx_key:
        private_key = spx_key.file
        spx_sig = ctx.actions.declare_file(paths.replace_extension(spxmsg.basename, ".spx_sig"))
        domain = spx_key.config.get("domain", "Pure")
        rev = spx_key.config.get("byte-reversal-bug", "false")
        ctx.actions.run(
            outputs = [spx_sig],
            inputs = [spxmsg, private_key],
            arguments = [
                "--rcfile=",
                "--quiet",
                "spx",
                "sign",
                "--spx-hash-reversal-bug={}".format(rev),
                "--domain={}".format(domain),
                "--output={}".format(spx_sig.path),
                spxmsg.path,
                private_key.path,
            ],
            executable = tool.tool,
            mnemonic = "LocalSpxSign",
        )

    if rsa_key:
        return None, output_sig, spx_sig
    elif ecdsa_key:
        return output_sig, None, spx_sig
    else:
        fail("Expected an ECDSA or RSA key")

def hsmtool_sign(ctx, tool, digest, ecdsa_key, rsa_key, spxmsg = None, spx_key = None, profile = None):
    """Sign a digest with a token-provided private key.

    Args:
        ctx: The rule context.
        tool: file; A SigningToolInfo provider referring to the hsmtool binary.
        digest: file; The digest of the binary to be signed.
        ecdsa_key: struct; The ECDSA private key.
        rsa_key: struct; The RSA private key.
        spxmsg: file; The SPX+ message to be signed.
        spx_key: struct; The SPX+ private key.
        profile: str; The hsmtool profile.
    Returns:
        file, file, file: The RSA and SPX signature files.
    """
    if not profile:
        fail("Missing the `hsmtool` profile")

    if rsa_key:
        cmd = "rsa"
        sig = ctx.actions.declare_file(paths.replace_extension(digest.basename, ".rsa-sig"))
        label = rsa_key.name
        mnemonic = "HsmtoolRsaSign"
        retval = (None, sig, None)
    elif ecdsa_key:
        cmd = "ecdsa"
        sig = ctx.actions.declare_file(paths.replace_extension(digest.basename, ".ecdsa-sig"))
        label = ecdsa_key.name
        mnemonic = "HsmtoolEcdsaSign"
        retval = (sig, None, None)
    else:
        fail("Expected either rsa_key or ecdsa_key; got neither")

    ctx.actions.run(
        outputs = [sig],
        inputs = [digest, tool.tool] + tool.data,
        arguments = [
            "--quiet",
            "--lockfile=/tmp/hsmtool.lock",
            "--profile={}".format(profile),
            cmd,
            "sign",
            "--little-endian",
            "--format=sha256-hash",
            "--label={}".format(label),
            "--output={}".format(sig.path),
            digest.path,
        ],
        executable = tool.tool,
        execution_requirements = {
            "no-sandbox": "",
        },
        env = tool.env,
        mnemonic = mnemonic,
    )

    spx_sig = None
    if spxmsg and spx_key:
        domain = spx_key.config.get("domain", "Pure")
        if domain.lower() == "prehashedsha256":
            rev = spx_key.config.get("byte-reversal-bug", "false")
            fmt = "sha256-hash-reversed" if rev == "true" else "sha256-hash"
            args = [
                "--little-endian",
                "--format={}".format(fmt),
                "--domain={}".format(domain),
            ]
        else:
            args = [
                "--format=plain-text",
                "--domain={}".format(domain),
            ]
        spx_sig = ctx.actions.declare_file(paths.replace_extension(spxmsg.basename, ".spx-sig"))
        ctx.actions.run(
            outputs = [spx_sig],
            inputs = [spxmsg, tool.tool] + tool.data,
            arguments = [
                "--quiet",
                "--lockfile=/tmp/hsmtool.lock",
                "--profile={}".format(profile),
                "spx",
                "sign",
                "--label={}".format(spx_key.name),
                "--output={}".format(spx_sig.path),
                spxmsg.path,
            ] + args,
            executable = tool.tool,
            execution_requirements = {
                "no-sandbox": "",
            },
            env = tool.env,
            mnemonic = "HsmtoolSpxSign",
        )

    if rsa_key:
        return None, sig, spx_sig
    elif ecdsa_key:
        return sig, None, spx_sig
    else:
        fail("Expected an ECDSA or RSA key")

def clear_if_none_key(key_attr):
    """Clear the key attribute if it is set to "//hw/signing:none_key.

    Args:
        key_attr: The key attribute.
    Returns:
        The key attribute if it is not set to "//hw/signing:none_key" or {}.
    """
    if not key_attr:
        return None
    key, _ = key_attr.items()[0]
    if key.label.name == "none_key":
        return None
    return key_attr
