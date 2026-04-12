# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

exports_files(
    glob(["**/Cargo.toml"]),
    visibility = ["//visibility:public"],
)

exports_files(
    [
        "common/src/lib.rs",
        "kat/src/lib.rs",
        "image/verify/src/lib.rs",
        "sw-emulator/lib/bus/src/lib.rs",
        "sw-emulator/lib/cpu/src/lib.rs",
        "sw-emulator/lib/derive/src/lib.rs",
        "sw-emulator/lib/periph/src/lib.rs",
        "hw-model/types/src/lib.rs",
        "image/gen/src/lib.rs",
        "image/crypto/src/lib.rs",
        "image/fake-keys/src/lib.rs",
        "drivers/src/lib.rs",
        "builder/src/lib.rs",
        "cfi/lib/src/lib.rs",
        "cpu/src/lib.rs",
        "fmc/src/main.rs",
        "rom/dev/src/main.rs",
        "rom/dev/src/start.S",
        "runtime/src/lib.rs",
        "runtime/src/main.rs",
        "auth-manifest/gen/src/lib.rs",
    ],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "all_files",
    srcs = glob(["**"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "rom_srcs",
    srcs = glob(["rom/dev/src/**/*.rs", "rom/dev/src/**/*.S", "rom/dev/src/**/*.ld"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "common_srcs",
    srcs = glob(["common/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "kat_srcs",
    srcs = glob(["kat/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "image_verify_srcs",
    srcs = glob(["image/verify/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "caliptra_sw_fmc_srcs",
    srcs = glob([
        "fmc/src/**/*.rs",
        "fmc/src/**/*.S",
    ]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "caliptra_sw_runtime_srcs",
    srcs = glob([
        "runtime/src/**/*.rs",
        "runtime/src/**/*.S",
    ]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emu_bus_srcs",
    srcs = glob(["sw-emulator/lib/bus/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emu_cpu_srcs",
    srcs = glob(["sw-emulator/lib/cpu/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emu_periph_srcs",
    srcs = glob(["sw-emulator/lib/periph/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emu_derive_srcs",
    srcs = glob(["sw-emulator/lib/derive/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "hw_model_types_srcs",
    srcs = glob(["hw-model/types/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)
