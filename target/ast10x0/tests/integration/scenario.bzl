# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""One QEMU scenario: a kernel, its apps, the image and the test.

Every scenario needs the same seven rules in the same order, and each
negative case needs them again with one `--cfg` added. Spelling that out
per package is where a copy drifts from its original, so it lives here
once.

A negative scenario is a package of its own because an app's target name
is its name in the system config, and two targets in one package cannot
share a name. It reuses the positive scenario's sources and config
through labels, so the only thing a negative package states is what
differs: the flag.
"""

load("@pigweed//pw_kernel/tooling:rust_app.bzl", "rust_app")
load("@pigweed//pw_kernel/tooling:system_image.bzl", "system_image", "system_image_test")
load("@pigweed//pw_kernel/tooling:target_codegen.bzl", "target_codegen")
load("@pigweed//pw_kernel/tooling:target_linker_script.bzl", "target_linker_script")
load("@pigweed//pw_kernel/tooling/panic_detector:rust_binary_no_panics_test.bzl", "rust_binary_no_panics_test")
load("@rules_rust//rust:defs.bzl", "rust_binary")
load("//target/ast10x0:defs.bzl", "TARGET_COMPATIBLE_WITH")

# Every `--cfg` name any scenario in this tree uses. Declared on every
# build, not just the one that sets it, because an undeclared cfg name is
# a warning and warnings are errors here. A name goes in once and the
# scenario that sets it says so in its own BUILD file.
SCENARIO_CFGS = [
    "device_hangs",
    "corrupt_image",
    "refused_update",
]

KERNEL_DEPS = [
    "//target/ast10x0:entry",
    "@pigweed//pw_kernel/arch/arm_cortex_m:arch_arm_cortex_m",
    "@pigweed//pw_kernel/kernel",
    "@pigweed//pw_kernel/subsys/console:console_backend",
    "@pigweed//pw_kernel/target:target_common",
    "@pigweed//pw_kernel/userspace",
    "@pigweed//pw_log/rust:pw_log",
]

def qemu_scenario(
        name,
        system_config,
        target_src,
        apps,
        cfgs = None,
        test_tags = None):
    """Builds one scenario image and the QEMU test that runs it.

    One scenario per package: the kernel's `target.rs` names the codegen
    crate `codegen`, so that target's name is fixed rather than derived.

    Args:
        name: Image name. The test is `<name>_test`.
        system_config: Label of the system.json5 filegroup.
        target_src: Label of the kernel's `target.rs`.
        apps: One dict per app, with `name` (which must match the app in
            the system config), `src`, and `deps`.
        cfgs: `--cfg` names set on every app, from `SCENARIO_CFGS`. A
            scenario that inverts its own verdict reads these, so the
            negative case passes when the failure is caught rather than
            failing like a broken build.
        test_tags: Extra tags for the QEMU test.
    """
    cfgs = cfgs or []
    for cfg in cfgs:
        if cfg not in SCENARIO_CFGS:
            fail("unknown scenario cfg {}; add it to SCENARIO_CFGS".format(cfg))

    rustc_flags = ["--check-cfg=cfg({})".format(c) for c in SCENARIO_CFGS]
    rustc_flags += ["--cfg={}".format(c) for c in cfgs]

    target_codegen(
        name = "codegen",
        arch = "@pigweed//pw_kernel/arch/arm_cortex_m:arch_arm_cortex_m",
        system_config = system_config,
        target_compatible_with = TARGET_COMPATIBLE_WITH,
    )

    target_linker_script(
        name = "linker_script",
        system_config = system_config,
        tags = ["kernel"],
        target_compatible_with = TARGET_COMPATIBLE_WITH,
        template = "//target/ast10x0:linker_script_template",
    )

    rust_binary(
        name = "target",
        srcs = [target_src],
        edition = "2024",
        tags = ["kernel"],
        target_compatible_with = TARGET_COMPATIBLE_WITH,
        deps = [
            ":codegen",
            ":linker_script",
        ] + KERNEL_DEPS,
    )

    app_labels = []
    for app in apps:
        rust_app(
            name = app["name"],
            srcs = [app["src"]],
            codegen_crate_name = "app_" + app["name"],
            edition = "2024",
            rustc_flags = rustc_flags,
            system_config = system_config,
            tags = ["kernel"],
            target_compatible_with = TARGET_COMPATIBLE_WITH,
            deps = app["deps"],
        )
        app_labels.append(":" + app["name"])

    system_image(
        name = name,
        apps = app_labels,
        kernel = ":target",
        platform = "//target/ast10x0",
        system_config = system_config,
        tags = ["kernel"],
        target_compatible_with = TARGET_COMPATIBLE_WITH,
        visibility = ["//visibility:public"],
    )

    system_image_test(
        name = name + "_test",
        image = ":" + name,
        tags = ["qemu_only"] + (test_tags or []),
        target_compatible_with = TARGET_COMPATIBLE_WITH,
    )

    rust_binary_no_panics_test(
        name = name + "_no_panics_test",
        binary = ":" + name,
        tags = ["kernel"],
    )
