# Copyright 2025 The Pigweed Authors
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

load("@bazel_skylib//lib:paths.bzl", "paths")
load("//target/earlgrey/tooling/signing:util.bzl", "key_ext")

def presigning_artifacts(ctx, opentitantool, src, manifest, ecdsa_key, rsa_key, spx_key, basename = None, keyname_in_filenames = False):
    """Create the pre-signing artifacts for a given input binary.

    Applies the manifest and public components of the keys.  Creates the
    digests/messages required for signing.

    Args:
        ctx: The rule context.
        opentitantool: file; The opentitantool binary.
        src: file; The source binary
        manifest: file; The manifest file.
        ecdsa_key: struct; The ECDSA public key.
        rsa_key: struct; The RSA public key.
        spx_key: struct; The SPX+ public key.
        basename: str; Optional basename of the outputs.  Defaults to src.basename.
        keyname_in_filenames: bool; Whether or not to use the key names to construct filenames.
                              Used in test-signing flows to maintain compatibility with existing
                              naming conventions for DV tests.
    Returns:
        struct: A struct containing the pre-signing binary, the digest and spx message files.
    """
    if ecdsa_key and rsa_key:
        fail("Only one of ECDSA or RSA key should be provided")

    kext = key_ext(ecdsa_key, rsa_key, spx_key)
    if not basename:
        basename = src.basename
    if keyname_in_filenames:
        basename = paths.replace_extension(basename, kext)
    else:
        basename = paths.replace_extension(basename, "")

    signing_directives = []
    pre = ctx.actions.declare_file("{}.pre-signing".format(basename))
    inputs = [
        src,
    ]
    manifest_args = []
    if manifest:
        inputs.append(manifest)
        manifest_args.append("--manifest={}".format(manifest.path))

    ecdsa_or_rsa_args = []
    if ecdsa_key:
        selected_ecdsa_key = getattr(ecdsa_key, "file", None)
        ecdsa_or_rsa_args.append("--ecdsa-key={}".format(selected_ecdsa_key.path))
        inputs.append(selected_ecdsa_key)
    elif rsa_key:
        ecdsa_or_rsa_args.append("--rsa-key={}".format(rsa_key.file.path))
        inputs.append(rsa_key.file)

    spx_args = []
    spx_domain = None
    if spx_key:
        spx_domain = spx_key.config.get("domain", "Pure")
        selected_spx_key = getattr(spx_key, "file", None)
        spx_args.append("--spx-key={}".format(selected_spx_key.path))
        inputs.append(selected_spx_key)
    ctx.actions.run(
        outputs = [pre],
        inputs = inputs,
        arguments = [
            "--rcfile=",
            "--quiet",
            "image",
            "manifest",
            "update",
            "--domain={}".format(spx_domain),
            "--output={}".format(pre.path),
            src.path,
        ] + manifest_args + ecdsa_or_rsa_args + spx_args,
        executable = opentitantool,
        mnemonic = "PreSigningArtifacts",
    )

    # Compute digest to be signed with RSA or ECDSA.
    digest = ctx.actions.declare_file("{}.digest".format(basename))
    ctx.actions.run(
        outputs = [digest],
        inputs = [pre],
        arguments = [
            "--rcfile=",
            "--quiet",
            "image",
            "digest",
            "--bin={}".format(digest.path),
            pre.path,
        ],
        executable = opentitantool,
        mnemonic = "PreSigningDigest",
    )

    if rsa_key:
        signing_directives.append(struct(
            command = "rsa-sign",
            id = None,
            label = rsa_key.name,
            format = "Sha256Hash",
            little_endian = True,
            output = "{}.rsa_sig".format(basename),
            input = "{}.digest".format(basename),
        ))
    elif ecdsa_key:
        signing_directives.append(struct(
            command = "ecdsa-sign",
            id = None,
            label = ecdsa_key.name,
            format = "Sha256Hash",
            little_endian = True,
            output = "{}.ecdsa_sig".format(basename),
            input = "{}.digest".format(basename),
        ))

    # Compute message to be signed with SPX+.
    spxmsg = None
    if spx_key:
        if spx_domain.lower() == "prehashedsha256":
            spxmsg = digest
            rev = spx_key.config.get("byte-reversal-bug", "false")
            fmt = "Sha256HashReversed" if rev == "true" else "Sha256Hash"
            signing_directives.append(struct(
                command = "spx-sign",
                id = None,
                label = spx_key.name,
                format = fmt,
                domain = spx_domain,
                output = "{}.spx_sig".format(basename),
                input = "{}.digest".format(basename),
            ))
        else:
            spxmsg = ctx.actions.declare_file("{}.spx-message".format(basename))
            ctx.actions.run(
                outputs = [spxmsg],
                inputs = [pre],
                arguments = [
                    "--rcfile=",
                    "--quiet",
                    "image",
                    "spx-message",
                    "--output={}".format(spxmsg.path),
                    pre.path,
                ],
                executable = opentitantool,
                mnemonic = "PreSigningSpxMessage",
            )
            signing_directives.append(struct(
                command = "spx-sign",
                id = None,
                label = spx_key.name,
                format = "PlainText",
                domain = spx_domain,
                output = "{}.spx_sig".format(basename),
                input = "{}.spx-message".format(basename),
            ))

    return struct(pre = pre, digest = digest, spxmsg = spxmsg, script = signing_directives)

def post_signing_attach(ctx, opentitantool, pre, ecdsa_sig, rsa_sig, spx_sig):
    """Attach signatures to an unsigned binary.

    Args:
        ctx: The rule context.
        opentitantool: file; The opentitantool binary.
        pre: file; The pre-signed input binary.
        ecdsa_sig: file; The ECDSA-signed digest of the binary.
        rsa_sig: file; The RSA-signed digest of the binary.
        spx_sig: file; The SPX-signed message of the binary.
    Returns:
        file: The signed binary.
    """
    if ecdsa_sig and rsa_sig:
        fail("Only one of ECDSA or RSA signature should be provided")

    signed = ctx.actions.declare_file(paths.replace_extension(pre.basename, ".signed.bin"))
    inputs = [pre]

    args = [
        "--rcfile=",
        "--quiet",
        "image",
        "manifest",
        "update",
        "--update-length=false",
        "--output={}".format(signed.path),
        pre.path,
    ]

    if rsa_sig:
        inputs.append(rsa_sig)
        args.append("--rsa-signature={}".format(rsa_sig.path))

    if ecdsa_sig:
        inputs.append(ecdsa_sig)
        args.append("--ecdsa-signature={}".format(ecdsa_sig.path))

    if spx_sig:
        inputs.append(spx_sig)
        args.append("--spx-signature={}".format(spx_sig.path))

    ctx.actions.run(
        outputs = [signed],
        inputs = inputs,
        arguments = args,
        executable = opentitantool,
        mnemonic = "PostSigningAttach",
    )
    return signed
