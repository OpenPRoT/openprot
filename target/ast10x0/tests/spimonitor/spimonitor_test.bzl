# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@pigweed//pw_kernel/tooling:system_image.bzl", "system_image", "system_image_test")
load("@pigweed//pw_kernel/tooling/panic_detector:rust_binary_no_panics_test.bzl", "rust_binary_no_panics_test")
load("@rules_rust//rust:defs.bzl", "rust_binary")

def spimonitor_test(index, target_compatible_with):
    target_name = "target_spim{}".format(index)
    image_name = "spimonitor{}_test".format(index)

    rust_binary(
        name = target_name,
        srcs = [
            "test_common.rs",
            "test_spim{}.rs".format(index),
        ],
        crate_root = "test_spim{}.rs".format(index),
        edition = "2024",
        tags = ["kernel"],
        target_compatible_with = target_compatible_with,
        deps = [
            ":codegen",
            ":linker_script",
            "//target/ast10x0:entry",
            "//target/ast10x0/board:ast10x0_board",
            "//target/ast10x0/peripherals",
            "@pigweed//pw_kernel/subsys/console:console_backend",
            "@pigweed//pw_kernel/target:target_common",
            "@pigweed//pw_log/rust:pw_log",
        ],
    )

    system_image(
        name = image_name,
        kernel = ":" + target_name,
        platform = "//target/ast10x0",
        system_config = ":system_config",
        tags = ["kernel"],
        target_compatible_with = target_compatible_with,
        userspace = False,
        visibility = ["//visibility:public"],
    )

    system_image_test(
        name = "spimonitor{}_evb_test".format(index),
        image = ":" + image_name,
        tags = ["hardware"],
        target_compatible_with = select({
            "//target/ast10x0:qemu_enabled": ["@platforms//:incompatible"],
            "//conditions:default": [],
        }),
        visibility = ["//visibility:public"],
    )

    rust_binary_no_panics_test(
        name = "spimonitor{}_no_panics_test".format(index),
        binary = ":" + image_name,
        tags = ["kernel"],
    )

def spimonitor_setup_all(target_compatible_with):
    rust_binary(
        name = "setup_all_spim",
        srcs = [
            "setup_all_spim.rs",
            "test_common.rs",
        ],
        crate_root = "setup_all_spim.rs",
        edition = "2024",
        tags = ["kernel"],
        target_compatible_with = target_compatible_with,
        deps = [
            ":codegen",
            ":linker_script",
            "//target/ast10x0:entry",
            "//target/ast10x0/board:ast10x0_board",
            "//target/ast10x0/peripherals",
            "@pigweed//pw_kernel/subsys/console:console_backend",
            "@pigweed//pw_kernel/target:target_common",
            "@pigweed//pw_log/rust:pw_log",
        ],
    )

    system_image(
        name = "setup_all_spim_image",
        kernel = ":setup_all_spim",
        platform = "//target/ast10x0",
        system_config = ":system_config",
        tags = ["kernel"],
        target_compatible_with = target_compatible_with,
        userspace = False,
        visibility = ["//visibility:public"],
    )

    rust_binary_no_panics_test(
        name = "setup_all_spim_no_panics_test",
        binary = ":setup_all_spim_image",
        tags = ["kernel"],
    )
