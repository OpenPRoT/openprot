# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "ast1060_pac",
    srcs = glob(["src/**/*.rs"]),
    edition = "2021",
    rustc_flags = ["-Amismatched-lifetime-syntaxes"],
    visibility = ["//visibility:public"],
    deps = [
        "@rust_crates//:cortex-m",
        "@rust_crates//:vcell",
    ],
)
