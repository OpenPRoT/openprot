# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

exports_files(
    glob(["**/Cargo.toml"]),
    visibility = ["//visibility:public"],
)

exports_files(
    [
        "emulator/app/src/main.rs",
        "emulator/app/src/lib.rs",
        "emulator/app/mcu-mbox/src/lib.rs",
        "emulator/bmc/src/lib.rs",
        "emulator/caliptra/src/lib.rs",
        "emulator/periph/src/lib.rs",
        "rom/src/lib.rs",
        "common/testing/src/lib.rs",
        "registers/generated-emulator/src/lib.rs",
        "emulator/consts/src/lib.rs",
        "caliptra-util-host/apps/mailbox/server/src/lib.rs",
        "builder/src/lib.rs",
        "platforms/emulator/rom/src/main.rs",
        "romtime/src/lib.rs",
    ],
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emulator_srcs",
    srcs = glob(["emulator/app/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "rom_srcs",
    srcs = glob(["rom/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emulator_bmc_srcs",
    srcs = glob(["emulator/bmc/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emulator_caliptra_srcs",
    srcs = glob(["emulator/caliptra/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emulator_periph_srcs",
    srcs = glob(["emulator/periph/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "emulator_registers_generated_srcs",
    srcs = glob(["registers/generated-emulator/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "builder_srcs",
    srcs = glob(["builder/src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "all_files",
    srcs = glob(["**"]),
    visibility = ["//visibility:public"],
)
