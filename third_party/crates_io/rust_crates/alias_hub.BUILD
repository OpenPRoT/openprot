# Licensed under the Apache-2.0 license
"""Crate aliases that route to the appropriate crate universe.

Cortex-M embedded crates are routed to @oot_crates_no_std//
Other crates are routed to the main @rust_crates_base// universe

This avoids duplicate symbol issues when embedded crates like aspeed-ddk depend
on cortex-m, which is also used by pw_kernel.
"""

# Cortex-M embedded crates - always use oot_crates_no_std (not in main rust_crates_base)
alias(
    name = "cortex-m",
    actual = "@oot_crates_no_std//:cortex-m",
    visibility = ["//visibility:public"],
)

alias(
    name = "cortex-m-rt",
    actual = "@oot_crates_no_std//:cortex-m-rt",
    visibility = ["//visibility:public"],
)

alias(
    name = "cortex-m-semihosting",
    actual = "@oot_crates_no_std//:cortex-m-semihosting",
    visibility = ["//visibility:public"],
)

alias(
    name = "embedded-hal",
    actual = "@oot_crates_no_std//:embedded-hal",
    visibility = ["//visibility:public"],
)

alias(
    name = "embedded-io",
    actual = "@oot_crates_no_std//:embedded-io",
    visibility = ["//visibility:public"],
)

alias(
    name = "fugit",
    actual = "@oot_crates_no_std//:fugit",
    visibility = ["//visibility:public"],
)

alias(
    name = "heapless",
    actual = "@oot_crates_no_std//:heapless",
    visibility = ["//visibility:public"],
)

alias(
    name = "hex-literal",
    actual = "@oot_crates_no_std//:hex-literal",
    visibility = ["//visibility:public"],
)

alias(
    name = "nb",
    actual = "@oot_crates_no_std//:nb",
    visibility = ["//visibility:public"],
)

alias(
    name = "openprot-hal-blocking",
    actual = "@oot_crates_no_std//:openprot-hal-blocking",
    visibility = ["//visibility:public"],
)

alias(
    name = "proposed-traits",
    actual = "@oot_crates_no_std//:proposed-traits",
    visibility = ["//visibility:public"],
)

alias(
    name = "zerocopy",
    actual = "@oot_crates_no_std//:zerocopy",
    visibility = ["//visibility:public"],
)

# Host crates - route to main rust_crates_base
alias(
    name = "anyhow",
    actual = "@rust_crates_base//:anyhow",
    visibility = ["//visibility:public"],
)

alias(
    name = "bitfield-struct",
    actual = "@rust_crates_base//:bitfield-struct",
    visibility = ["//visibility:public"],
)

alias(
    name = "bitflags",
    actual = "@rust_crates_base//:bitflags",
    visibility = ["//visibility:public"],
)

alias(
    name = "clap",
    actual = "@rust_crates_base//:clap",
    visibility = ["//visibility:public"],
)

alias(
    name = "compiler_builtins",
    actual = "@rust_crates_base//:compiler_builtins",
    visibility = ["//visibility:public"],
)

alias(
    name = "hashlink",
    actual = "@rust_crates_base//:hashlink",
    visibility = ["//visibility:public"],
)

alias(
    name = "minijinja",
    actual = "@rust_crates_base//:minijinja",
    visibility = ["//visibility:public"],
)

alias(
    name = "nom",
    actual = "@rust_crates_base//:nom",
    visibility = ["//visibility:public"],
)

alias(
    name = "object",
    actual = "@rust_crates_base//:object",
    visibility = ["//visibility:public"],
)

alias(
    name = "panic-halt",
    actual = "@rust_crates_base//:panic-halt",
    visibility = ["//visibility:public"],
)

alias(
    name = "paste",
    actual = "@rust_crates_base//:paste",
    visibility = ["//visibility:public"],
)

alias(
    name = "proc-macro2",
    actual = "@rust_crates_base//:proc-macro2",
    visibility = ["//visibility:public"],
)

alias(
    name = "quote",
    actual = "@rust_crates_base//:quote",
    visibility = ["//visibility:public"],
)

alias(
    name = "riscv",
    actual = "@rust_crates_base//:riscv",
    visibility = ["//visibility:public"],
)

alias(
    name = "riscv-rt",
    actual = "@rust_crates_base//:riscv-rt",
    visibility = ["//visibility:public"],
)

alias(
    name = "riscv-semihosting",
    actual = "@rust_crates_base//:riscv-semihosting",
    visibility = ["//visibility:public"],
)

alias(
    name = "rustc-demangle",
    actual = "@rust_crates_base//:rustc-demangle",
    visibility = ["//visibility:public"],
)

alias(
    name = "serde",
    actual = "@rust_crates_base//:serde",
    visibility = ["//visibility:public"],
)

alias(
    name = "serde_json5",
    actual = "@rust_crates_base//:serde_json5",
    visibility = ["//visibility:public"],
)

alias(
    name = "syn",
    actual = "@rust_crates_base//:syn",
    visibility = ["//visibility:public"],
)

alias(
    name = "toml",
    actual = "@rust_crates_base//:toml",
    visibility = ["//visibility:public"],
)
