# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Builds a flash image from a slot layout, for QEMU tests to hand the device."""

def _flash_image_impl(ctx):
    image = ctx.actions.declare_file(ctx.label.name + ".img")

    args = ctx.actions.args()
    args.add("--flash-size", ctx.attr.flash_size)
    args.add("--output", image)

    payloads = []
    for spec, target in ctx.attr.slots.items():
        files = target.files.to_list()
        if len(files) != 1:
            fail("slot {}: {} is not a single file".format(spec, target.label))
        payloads.append(files[0])
        args.add("--slot", "{}={}".format(spec, files[0].path))

    if ctx.attr.golden:
        files = ctx.attr.golden.files.to_list()
        if len(files) != 1:
            fail("golden: {} is not a single file".format(ctx.attr.golden.label))
        payloads.append(files[0])
        args.add("--golden", "{}={}".format(ctx.attr.golden_region, files[0].path))

    ctx.actions.run(
        mnemonic = "FlashImage",
        executable = ctx.executable._builder,
        arguments = [args],
        inputs = payloads,
        outputs = [image],
    )

    return [DefaultInfo(files = depset([image]))]

flash_image = rule(
    implementation = _flash_image_impl,
    doc = "A flash image with each payload written at its slot's base, the " +
          "rest left erased. The layout goes through the same constructors " +
          "the board tables use, so a layout the orchestrator would refuse " +
          "fails the build here.",
    attrs = {
        "flash_size": attr.int(
            doc = "Size in bytes of the image, matching the test's flash_size.",
            mandatory = True,
        ),
        "golden": attr.label(
            doc = "Payload for the golden image, if the layout has one.",
            allow_single_file = True,
        ),
        "golden_region": attr.string(
            doc = "Where the golden image lives, as base:len.",
            default = "",
        ),
        "slots": attr.string_keyed_label_dict(
            doc = "Maps id:base:len to the file that fills that slot. Base " +
                  "and len take decimal or 0x hex.",
            allow_files = True,
        ),
        "_builder": attr.label(
            default = "//tools/flash_image:flash_image",
            executable = True,
            cfg = "exec",
        ),
    },
)
