# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Rules and macros for building, signing, and testing VeeR MCU images on the Caliptra emulator and VCK190 FPGA."""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")
load("@rules_rust//rust:defs.bzl", "rust_test")

DEFAULT_VENDOR_PK_HASH = "b17ca877666657ccd100e6926c7206b60c995cb68992c6c9baefce728af05441dee1ff415adfc187e1e4edb4d3b2d909"

_INTERFACE_COMPAT = {
    "emulator": [
        "@platforms//cpu:x86_64",
        "@platforms//os:linux",
    ],
    "fpga": [
        "@platforms//cpu:aarch64",
        "@platforms//os:linux",
    ],
}

def _interface_tags(tags, interface, extra = []):
    """Returns `tags` plus the `interface` tag and `extra`, deduplicated.

    Interface tags are owned by these macros; passing one explicitly is an
    error since it would be applied to every generated interface target.
    """
    tags = tags or []
    for t in _INTERFACE_COMPAT:
        if t in tags:
            fail("Do not pass the '{}' tag; it is added automatically per interface.".format(t))
    return list(set(tags + [interface] + extra))

def _rlocationpath(ctx, f):
    """Returns the rlocation path of `f` (what `$(rlocationpath)` would expand to)."""
    if f.short_path.startswith("../"):
        return f.short_path[len("../"):]
    return ctx.workspace_name + "/" + f.short_path

def _boot_artifact_transition_impl(_, attr):
    # Pin platforms, //target/veer:target_type, and //third_party/caliptra:platform
    # to constants so caller flag state never forks a duplicate ST-<hash> build
    # directory for Caliptra ROM, Caliptra FW (FMC + RT), or MCU ROM.
    platform = "fpga" if attr.interface == "fpga" else "emulator"
    return {
        "//command_line_option:platforms": ["//third_party/caliptra/platforms:caliptra"],
        "//target/veer:target_type": platform,
        "//third_party/caliptra:platform": platform,
    }

_boot_artifact_transition = transition(
    implementation = _boot_artifact_transition_impl,
    inputs = [],
    outputs = [
        "//command_line_option:platforms",
        "//target/veer:target_type",
        "//third_party/caliptra:platform",
    ],
)

def _caliptra_boot_artifact_impl(ctx):
    files = ctx.attr.target[0][DefaultInfo].files.to_list()
    return [DefaultInfo(files = depset(files))]

_caliptra_boot_artifact = rule(
    implementation = _caliptra_boot_artifact_impl,
    attrs = {
        "interface": attr.string(
            default = "emulator",
            values = ["emulator", "fpga"],
        ),
        "target": attr.label(cfg = _boot_artifact_transition),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)

def caliptra_boot_artifact(name, interface = "emulator", tags = None, target_compatible_with = None, **kwargs):
    if target_compatible_with == None:
        target_compatible_with = _INTERFACE_COMPAT[interface]
    _caliptra_boot_artifact(
        name = name,
        interface = interface,
        tags = _interface_tags(tags, interface),
        target_compatible_with = target_compatible_with,
        **kwargs
    )

def _mcu_image_transition_impl(_, attr):
    # Pin platforms, //target/veer:target_type, and //third_party/caliptra:platform
    # so each MCU system_image is built in a single canonical configuration per
    # interface regardless of whether the top-level target is x86_64 or aarch64.
    platform = "fpga" if attr.interface == "fpga" else "emulator"
    return {
        "//command_line_option:platforms": ["//target/veer:veer"],
        "//target/veer:target_type": platform,
        "//third_party/caliptra:platform": platform,
    }

_mcu_image_transition = transition(
    implementation = _mcu_image_transition_impl,
    inputs = [],
    outputs = [
        "//command_line_option:platforms",
        "//target/veer:target_type",
        "//third_party/caliptra:platform",
    ],
)

def _caliptra_mcu_image_impl(ctx):
    system_image_info = ctx.attr.image[0][SystemImageInfo]
    bin_file = system_image_info.bin
    platform = "fpga" if ctx.attr.interface == "fpga" else "emulator"

    manifest = ctx.actions.declare_file("{}.manifest".format(ctx.attr.name))
    ctx.actions.run(
        inputs = [
            bin_file,
            ctx.file.caliptra_rom,
            ctx.file.caliptra_firmware,
        ],
        outputs = [manifest],
        executable = ctx.executable._signer,
        env = {"RUST_BACKTRACE": "1"},
        arguments = [
            "auth-manifest",
            "create",
            "--platform={}".format(platform),
            "--mcu_image={}".format(bin_file.path),
            "--caliptra_rom={}".format(ctx.file.caliptra_rom.path),
            "--caliptra_firmware={}".format(ctx.file.caliptra_firmware.path),
            "--vendor_pk_hash={}".format(ctx.attr.vendor_pk_hash),
            "--output={}".format(manifest.path),
        ],
    )

    # JSON map of boot inputs, read by //target/veer/tooling:test_boot_files.
    # File paths are rlocation paths for the `runfiles` crate.
    boot_env = ctx.actions.declare_file("{}.boot_env".format(ctx.attr.name))
    ctx.actions.write(
        output = boot_env,
        content = json.encode_indent({
            "CALIPTRA_FW": _rlocationpath(ctx, ctx.file.caliptra_firmware),
            "CALIPTRA_ROM": _rlocationpath(ctx, ctx.file.caliptra_rom),
            "MCU_FW": _rlocationpath(ctx, bin_file),
            "MCU_ROM": _rlocationpath(ctx, ctx.file.mcu_rom),
            "SOC_MANIFEST": _rlocationpath(ctx, manifest),
            "VENDOR_PK_HASH": ctx.attr.vendor_pk_hash,
        }) + "\n",
    )

    boot_files = [
        bin_file,
        manifest,
        ctx.file.caliptra_rom,
        ctx.file.caliptra_firmware,
        ctx.file.mcu_rom,
    ]

    return [
        DefaultInfo(
            files = depset([boot_env]),
            runfiles = ctx.runfiles(files = [boot_env] + boot_files),
        ),
    ]

_caliptra_mcu_image = rule(
    implementation = _caliptra_mcu_image_impl,
    attrs = {
        "caliptra_firmware": attr.label(
            allow_single_file = True,
            mandatory = True,
        ),
        "caliptra_rom": attr.label(
            allow_single_file = True,
            mandatory = True,
        ),
        "image": attr.label(
            cfg = _mcu_image_transition,
            mandatory = True,
            providers = [SystemImageInfo],
        ),
        "interface": attr.string(
            mandatory = True,
            values = ["emulator", "fpga"],
        ),
        "mcu_rom": attr.label(
            allow_single_file = True,
            mandatory = True,
        ),
        "vendor_pk_hash": attr.string(
            default = DEFAULT_VENDOR_PK_HASH,
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
        "_signer": attr.label(
            allow_single_file = True,
            cfg = "exec",
            default = "//third_party/caliptra/caliptra-mcu-sw:signer",
            executable = True,
        ),
    },
)

def caliptra_mcu_image(
        name,
        image,
        interface,
        caliptra_rom = "//target/veer/tooling:caliptra_rom_{}",
        caliptra_firmware = "//target/veer/tooling:caliptra_firmware_{}",
        mcu_rom = "//target/veer/tooling:mcu_rom_{}",
        tags = None,
        target_compatible_with = None,
        **kwargs):
    """Transitions a Pigweed system_image to the given interface (emulator or fpga) and signs its SoC manifest once."""
    if interface not in _INTERFACE_COMPAT:
        fail("caliptra_mcu_image interface must be 'emulator' or 'fpga', got: {}".format(interface))
    if target_compatible_with == None:
        target_compatible_with = _INTERFACE_COMPAT[interface]

    _caliptra_mcu_image(
        name = name,
        caliptra_firmware = caliptra_firmware.format(interface),
        caliptra_rom = caliptra_rom.format(interface),
        image = image,
        interface = interface,
        mcu_rom = mcu_rom.format(interface),
        tags = _interface_tags(tags, interface),
        target_compatible_with = target_compatible_with,
        **kwargs
    )

def _mcu_image_label(mcu_image, interface):
    label = native.package_relative_label(mcu_image)
    suffix = "_image_" + interface
    if label.name.endswith(suffix) or label.name.endswith("_" + interface):
        return label
    return label.same_package_label(label.name + suffix)

def caliptra_test(
        name,
        srcs,
        mcu_image,
        interfaces = ["emulator", "fpga"],
        deps = [],
        data = [],
        env = {},
        tags = [],
        edition = "2024",
        **kwargs):
    """Defines host-side rust_test target(s) (<name>_emulator and/or <name>_fpga) that drive `mcu_image` via `caliptra_mcu_hw_model`.

    Args:
        name: Base name for the generated `<name>_emulator` and/or `<name>_fpga` test targets.
        srcs: Rust test source files (mandatory).
        mcu_image: Label of the `veer_system_image` (or `caliptra_mcu_image`) to boot.
        interfaces: List of interfaces to generate targets for; subset of `["emulator", "fpga"]`.
        deps: Additional Rust dependencies for the test binary.
        data: Additional runfile data dependencies.
        env: Additional runtime environment variables.
        tags: Base tags applied to generated test targets.
        edition: Rust edition (defaults to "2024").
        **kwargs: Additional arguments forwarded to `rust_test`.
    """
    if not srcs:
        fail("caliptra_test requires explicit `srcs` (use caliptra_on_device_test for on-device tests)")
    if not interfaces:
        fail("caliptra_test requires at least one interface in `interfaces`")

    full_deps = list(deps)
    for required_dep in [
        "//target/veer/tooling:test_boot_files",
        "//third_party/caliptra/caliptra-mcu-sw:mcu_hw_model",
    ]:
        if required_dep not in full_deps:
            full_deps.append(required_dep)

    for iface in interfaces:
        if iface not in _INTERFACE_COMPAT:
            fail("Unsupported interface '{}' in caliptra_test (expected 'emulator' or 'fpga')".format(iface))

        target_name = name if name.endswith("_" + iface) else "{}_{}".format(name, iface)
        image_label = _mcu_image_label(mcu_image, iface)

        iface_tags = _interface_tags(tags, iface, ["exclusive"] if iface == "fpga" else [])

        test_env = {
            "CALIPTRA_BOOT_ENV": "$(rlocationpath {})".format(image_label),
            "RUST_TEST_THREADS": "1",
        }
        test_env.update(env)

        rust_test(
            name = target_name,
            srcs = srcs,
            data = list(data) + [image_label],
            deps = full_deps,
            edition = edition,
            env = test_env,
            tags = iface_tags,
            target_compatible_with = _INTERFACE_COMPAT[iface],
            **kwargs
        )

def caliptra_on_device_test(
        name,
        mcu_image,
        interfaces = ["emulator", "fpga"],
        **kwargs):
    """Defines `<name>_emulator` and/or `<name>_fpga` test targets for an on-device VeeR test image."""
    caliptra_test(
        name = name,
        srcs = ["//target/veer/tooling:on_device_test.rs"],
        crate_root = "//target/veer/tooling:on_device_test.rs",
        interfaces = interfaces,
        mcu_image = mcu_image,
        **kwargs
    )
