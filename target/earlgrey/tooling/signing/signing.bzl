# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")
load(
    "//target/earlgrey/tooling/signing:pre_post.bzl",
    "post_signing_attach",
    "presigning_artifacts",
)
load(
    "//target/earlgrey/tooling/signing:util.bzl",
    "clear_if_none_key",
    "get_override",
    "key_from_dict",
    "signing_tool_info",
)

def sign_binary(ctx, opentitantool, **kwargs):
    """Sign a binary.

    Args:
      ctx: The rule context.
      opentitantool: An opentitantool FilesToRun provider.
      **kwargs: Overrides of values normally retrived from the context object.
        ecdsa_key: The ECDSA signing key.
        rsa_key: The RSA signing key.
        spx_key: The SPHINCS+ signing key.
        bin: The input binary.
        manifest: The manifest header.
        _tool: The signing tool (opentitantool).
    Returns:
        A dict of all of the signing artifacts:
          pre: The pre-signing binary (input binary with manifest changes applied).
          digest: The SHA256 hash over the pre-signing binary.
          spxmsg: The SPHINCS+ message to be signed.
          ecdsa_sig: The ECDSA signature of the digest.
          rsa_sig: The RSA signature of the digest.
          spx_sig: The SPHINCS+ signature over the message.
          signed: The final signed binary.
    """
    key_attr = get_override(ctx, "attr.ecdsa_key", kwargs)
    key_attr = clear_if_none_key(key_attr)
    ecdsa_key = key_from_dict(key_attr, "ecdsa_key")

    # TODO(cfrantz): Refactor to remove RSA, as we no longer support
    # signing with RSA keys.
    rsa_attr = {}

    if rsa_attr and key_attr:
        fail("Only one of ECDSA or RSA key should be provided")

    if rsa_attr:
        # Select RSA as the key attribute since at this point we have already
        # determined that only one of ECDSA or RSA key should be provided.
        key_attr = rsa_attr

    rsa_key = key_from_dict(rsa_attr, "rsa_key")
    spx_key = key_from_dict(get_override(ctx, "attr.spx_key", kwargs), "spx_key")

    artifacts = presigning_artifacts(
        ctx,
        opentitantool,
        get_override(ctx, "file.bin", kwargs),
        get_override(ctx, "file.manifest", kwargs),
        ecdsa_key,
        rsa_key,
        spx_key,
        basename = kwargs.get("basename"),
        keyname_in_filenames = True,
    )
    tool, signing_func, profile = signing_tool_info(ctx, key_attr, opentitantool)
    ecdsa_sig, rsa_sig, spx_sig = signing_func(
        ctx,
        tool,
        artifacts.digest,
        ecdsa_key,
        rsa_key,
        artifacts.spxmsg,
        spx_key,
        profile,
    )
    signed = post_signing_attach(
        ctx,
        opentitantool,
        artifacts.pre,
        ecdsa_sig,
        rsa_sig,
        spx_sig,
    )
    return {
        "digest": artifacts.digest,
        "ecdsa_sig": ecdsa_sig,
        "pre": artifacts.pre,
        "rsa_sig": rsa_sig,
        "signed": signed,
        "spx_sig": spx_sig,
        "spxmsg": artifacts.spxmsg,
    }

def _sign_bin_impl(ctx):
    system_image_info = ctx.attr.bin[SystemImageInfo]
    result = sign_binary(ctx, ctx.executable._opentitantool, bin = system_image_info.bin)
    return [
        DefaultInfo(files = depset([result["signed"]]), data_runfiles = ctx.runfiles(files = [result["signed"]])),
    ]

sign_bin = rule(
    implementation = _sign_bin_impl,
    attrs = {
        "bin": attr.label(providers = [SystemImageInfo]),
        "ecdsa_key": attr.label_keyed_string_dict(
            allow_files = True,
            doc = "ECDSA public key to validate this image",
        ),
        "manifest": attr.label(allow_single_file = True),
        "rsa_key": attr.label_keyed_string_dict(
            allow_files = True,
            doc = "RSA public key to validate this image",
        ),
        "spx_key": attr.label_keyed_string_dict(
            allow_files = True,
            doc = "SPX public key to validate this image",
        ),
        "_opentitantool": attr.label(
            executable = True,
            allow_single_file = True,
            cfg = "exec",
            default = "@opentitan_devbundle//:opentitantool/opentitantool",
            doc = "opentitantool",
        ),
    },
)
