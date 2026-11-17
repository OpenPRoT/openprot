# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Rules for checking the hardware each process claims, straight from the system manifest.

A pin conflict is read from two `pin_claims_<process>` manifest constants naming the same pin;
an MMIO conflict is read straight from each process's own `memory_mappings` (`type: "device"`) —
a granted device range *is* the claim for MMIO, unlike a pin, where holding a register-block
grant doesn't say which bit within it is actually driven. See `manifest_check.rs`.
"""

def _manifest_hw_claim_test_impl(ctx):
    manifest = ctx.file.system_config
    launcher = ctx.actions.declare_file(ctx.attr.name + ".sh")

    ctx.actions.write(
        output = launcher,
        content = "exec ./{checker} ./{manifest} {flags}\n".format(
            checker = ctx.executable._checker.short_path,
            manifest = manifest.short_path,
            flags = "--expect-conflict" if ctx.attr.expect_conflict else "",
        ),
        is_executable = True,
    )

    return [DefaultInfo(
        executable = launcher,
        runfiles = ctx.runfiles(files = [manifest, ctx.executable._checker]),
    )]

manifest_hw_claim_test = rule(
    implementation = _manifest_hw_claim_test_impl,
    test = True,
    attrs = {
        "expect_conflict": attr.bool(
            doc = "Invert the check: pass only if the manifest does conflict.",
            default = False,
        ),
        "system_config": attr.label(
            doc = "system.json5 manifest to check.",
            mandatory = True,
            allow_single_file = True,
        ),
        "_checker": attr.label(
            executable = True,
            cfg = "exec",
            default = "//util/hw_claim:manifest_check",
        ),
    },
    doc = """Fails if one system manifest has a pin or MMIO hardware conflict.

    Reads system.json5 directly — no build, no assembled image needed. A plain host binary, not
    a QEMU/on-device image: every call site must pass
    `target_compatible_with = ["@platforms//cpu:x86_64", "@platforms//os:linux"]` (`target_compatible_with`
    is a built-in attribute, so this rule can't default it itself) — otherwise a
    `--platforms=//target/ast10x0` run tries to execute it through the embedded test runner
    anyway, which fails trying to detokenize this test's own launcher script as a device image.
    """,
)
