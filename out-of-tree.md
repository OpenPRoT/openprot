# Out-of-Tree Pigweed Kernel Build (from a Fork)

This guide explains how to set up an out-of-tree project that builds against
a **fork** of Pigweed rather than upstream, using the Earl Grey integration
in this repo as the reference pattern.

## 1. Fork Pigweed

```sh
# Fork https://pigweed.googlesource.com/pigweed/pigweed to your GitHub/GitLab
# Example: https://github.com/<your-org>/pigweed
```

## 2. Point MODULE.bazel at Your Fork

In the root `MODULE.bazel`, change the `git_override` for Pigweed:

```starlark
# ── BEFORE (upstream) ──
git_override(
    module_name = "pigweed",
    commit = "f52059663fee1e2ab2f2ab752301b159da3ca806",
    remote = "https://pigweed.googlesource.com/pigweed/pigweed",
)

# ── AFTER (your fork) ──
git_override(
    module_name = "pigweed",
    commit = "<your_fork_commit_sha>",
    remote = "https://github.com/<your-org>/pigweed",
)
```

If you want to track a **branch** instead of a pinned commit, you can use
`local_path_override` during development (faster iteration, no re-fetch):

```starlark
# For local development against a co-located Pigweed checkout:
local_path_override(
    module_name = "pigweed",
    path = "../pigweed",   # Relative path to your local Pigweed clone
)
```

> **Warning**: `local_path_override` is not reproducible across machines.
> Always switch back to `git_override` with a pinned commit for CI/production.

## 3. Full MODULE.bazel Template

```starlark
module(
    name = "my_pigweed_project",
    version = "0.0.1",
)

# ── Core dependencies ──
bazel_dep(name = "bazel_skylib", version = "1.8.2")
bazel_dep(name = "pigweed")
bazel_dep(name = "platforms", version = "1.0.0")
bazel_dep(name = "rules_rust", version = "0.66.0")
bazel_dep(name = "rules_platform", version = "0.1.0")
bazel_dep(name = "rules_python", version = "1.6.3")

# ── Point to your fork ──
git_override(
    module_name = "pigweed",
    commit = "<your_fork_commit_sha>",
    remote = "https://github.com/<your-org>/pigweed",
)

# ── Register crate generator (if using ureg for peripheral registers) ──
bazel_dep(name = "ureg")
git_override(
    module_name = "ureg",
    commit = "412ca40146d5d2012417e493b4a01096b04edf4b",
    remote = "https://github.com/chipsalliance/caliptra-ureg",
)

# ── Rust toolchain (managed by Pigweed) ──
pw_rust = use_extension("@pigweed//pw_toolchain/rust:extensions.bzl", "pw_rust")
pw_rust.toolchain(cipd_tag = "<rust_toolchain_cipd_tag>")
use_repo(pw_rust, "pw_rust_toolchains")

# ── C/C++ and Rust toolchain registration ──
register_toolchains(
    # Host
    "@pigweed//pw_toolchain/host_clang:host_cc_toolchain_linux",
    "@pigweed//pw_toolchain/host_clang:host_cc_toolchain_macos",
    # Target (pick one or more):
    "@pigweed//pw_toolchain/riscv_clang:riscv_clang_cc_toolchain_rv32imc",
    # "@pigweed//pw_toolchain/arm_clang:arm_clang_cc_toolchain_cortex_m4",
    "@pw_rust_toolchains//:all",
)

# ── Rust crate dependencies ──
crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")
crate.from_cargo(
    name = "rust_crates",
    cargo_lockfile = "//third_party/crates_io:Cargo.lock",
    manifests = ["//third_party/crates_io:Cargo.toml"],
    supported_platform_triples = [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        # Your SoC target triple:
        "riscv32imc-unknown-none-elf",
        # "thumbv7em-none-eabihf",
    ],
)
use_repo(crate, "rust_crates")

# Override Pigweed's internal crates with our unified crate universe
pw_rust_crates_ext = use_extension(
    "@pigweed//pw_build:pw_rust_crates_extension.bzl",
    "pw_rust_crates_extension",
)
override_repo(pw_rust_crates_ext, rust_crates = "rust_crates")

# ── (Optional) SoC-specific devbundle for simulator/runner tools ──
# bazel_dep(name = "my_soc_devbundle")
# archive_override(
#     module_name = "my_soc_devbundle",
#     integrity = "sha256-...",
#     url = "https://...",
# )
```

## 4. Required Project Structure

```
my-project/
├── MODULE.bazel                    # See above
├── BUILD.bazel                     # Root BUILD (can be minimal)
├── third_party/
│   └── crates_io/
│       ├── BUILD.bazel             # Empty (just package())
│       ├── Cargo.toml              # Rust deps for Bazel crate universe
│       └── Cargo.lock              # Generated: cargo generate-lockfile
└── target/
    └── <soc>/                      # Your SoC integration
        ├── BUILD.bazel             # platform(), entry, console, config libs
        ├── defs.bzl                # TARGET_COMPATIBLE_WITH
        ├── config.rs               # KernelConfig implementation
        ├── entry.rs                # Boot entry point
        ├── <mpu>.rs                # Memory protection setup
        ├── console.rs              # UART console backend
        ├── target.ld.jinja         # Linker script template
        ├── registers/              # Peripheral register crates
        │   ├── BUILD.bazel
        │   ├── registers.rs
        │   └── uart.rs (etc.)
        └── threads/kernel/         # Minimal test image
            ├── BUILD.bazel
            ├── system.json5
            └── target.rs
```

## 5. Third-Party Rust Crates (`third_party/crates_io/`)

### Cargo.toml
```toml
[package]
name = "rust_crates"
version = "0.1.0"
edition = "2021"

[lib]
path = "fake.rs"

[dependencies]
# Shared embedded deps
embedded-io = "0.6.1"
panic-halt = "1.0.0"
bitflags = "2.9.1"

# RISC-V target
riscv = "0.12.1"
riscv-rt = "0.12.2"

# ARM Cortex-M target (uncomment if needed)
# cortex-m = "0.7"
# cortex-m-rt = "0.7"
```

### BUILD.bazel
```starlark
# Empty — crate_universe handles everything
```

### Generate the lockfile
```sh
cd third_party/crates_io
touch fake.rs
cargo generate-lockfile
```

## 6. Root BUILD.bazel

```starlark
load("@bazel_skylib//rules:native_binary.bzl", "native_binary")

# Pigweed CLI launcher
native_binary(
    name = "pw",
    src = "@pigweed//pw_build/py:workflows_launcher",
)

# Code formatter
alias(
    name = "format",
    actual = "@pigweed//pw_presubmit/py:format",
)
```

## 7. SoC Target — Key Files

See `.github/copilot-instructions.md` "Integrating a New SoC with Pigweed"
section for complete templates of every file below.

### 7a. `target/<soc>/defs.bzl`
```starlark
TARGET_COMPATIBLE_WITH = select({
    "//target/<soc>:target_<soc>": [],
    "//conditions:default": ["@platforms//:incompatible"],
})
```

### 7b. `target/<soc>/BUILD.bazel` (abbreviated)
```starlark
load("@bazel_skylib//rules:common_settings.bzl", "string_flag")
load("@pigweed//pw_build:merge_flags.bzl", "flags_from_dict")
load("@pigweed//pw_kernel:flags.bzl", "KERNEL_DEVICE_COMMON_FLAGS")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("//target/<soc>:defs.bzl", "TARGET_COMPATIBLE_WITH")

platform(
    name = "<soc>",
    constraint_values = [
        ":target_<soc>",
        "@pigweed//pw_kernel/arch/riscv:timer_mtime",
        "@pigweed//pw_build/constraints/riscv/extensions:I",
        "@pigweed//pw_build/constraints/riscv/extensions:M",
        "@pigweed//pw_build/constraints/riscv/extensions:C",
        "@pigweed//pw_build/constraints/rust:no_std",
        "@platforms//cpu:riscv32",
        "@platforms//os:none",
    ],
    flags = flags_from_dict(
        KERNEL_DEVICE_COMMON_FLAGS | {
            "@pigweed//pw_kernel/config:kernel_config": ":config",
            "@pigweed//pw_kernel/subsys/console:console_backend": ":console",
            "@pigweed//pw_log/rust:pw_log_backend":
                "@pigweed//pw_kernel:log_backend_basic",
        },
    ),
    visibility = [":__subpackages__"],
)

constraint_value(
    name = "target_<soc>",
    constraint_setting = "@pigweed//pw_kernel/target:target",
    visibility = [":__subpackages__"],
)

# string_flag, config_settings, rust_library for entry/console/config...
# (see copilot-instructions.md Steps 2a-2d for full templates)
```

### 7c. Test Image — `target/<soc>/threads/kernel/BUILD.bazel`
```starlark
load("@pigweed//pw_kernel/tooling:system_image.bzl", "system_image")
load("@pigweed//pw_kernel/tooling:target_codegen.bzl", "target_codegen")
load("@pigweed//pw_kernel/tooling:target_linker_script.bzl", "target_linker_script")
load("@rules_rust//rust:defs.bzl", "rust_binary")
load("//target/<soc>:defs.bzl", "TARGET_COMPATIBLE_WITH")

system_image(name = "threads", kernel = ":target", platform = "//target/<soc>")

target_linker_script(
    name = "linker_script",
    system_config = ":system_config",
    tags = ["kernel"],
    template = "//target/<soc>:linker_script_template",
)

filegroup(name = "system_config", srcs = ["system.json5"])

target_codegen(
    name = "codegen",
    arch = "@pigweed//pw_kernel/arch/riscv:arch_riscv",
    system_config = ":system_config",
)

rust_binary(
    name = "target",
    srcs = ["target.rs"],
    edition = "2024",
    target_compatible_with = TARGET_COMPATIBLE_WITH,
    deps = [
        ":codegen",
        ":linker_script",
        "//target/<soc>:entry",
        "@pigweed//pw_kernel/arch/riscv:arch_riscv",
        "@pigweed//pw_kernel/kernel",
        "@pigweed//pw_kernel/subsys/console:console_backend",
        "@pigweed//pw_kernel/target:target_common",
        "@pigweed//pw_kernel/tests/threads/kernel:threads",
        "@pigweed//pw_log/rust:pw_log",
    ],
)
```

## 8. Building

```sh
# Install bazelisk (if not already)
# https://github.com/bazelbuild/bazelisk

# Build everything for your SoC
bazelisk build //target/<soc>/...

# Build just the kernel threads test image
bazelisk build //target/<soc>/threads/kernel:threads

# VS Code rust-analyzer setup
bazelisk run @rules_rust//tools/rust_analyzer:gen_rust_project -- //target/...
```

## 9. Syncing with Upstream Pigweed

When upstream Pigweed has changes you want:

```sh
cd /path/to/your/pigweed-fork

# Add upstream remote (one-time)
git remote add upstream https://pigweed.googlesource.com/pigweed/pigweed

# Fetch and merge
git fetch upstream
git merge upstream/main    # or rebase: git rebase upstream/main
git push origin main

# Then update your project's MODULE.bazel commit hash:
# commit = "<new_commit_sha_from_your_fork>"
```

For `local_path_override` workflows, just `git pull` in your local Pigweed
checkout — Bazel picks up changes immediately (no commit hash to update).

## 10. Troubleshooting

| Problem | Fix |
|---------|-----|
| `ERROR: no such package '@pigweed//pw_kernel/...'` | Your fork is missing pw_kernel; merge upstream |
| `ERROR: invalid registered toolchain` | Check `register_toolchains()` matches your SoC arch |
| `crate_universe` resolution failures | Run `cargo generate-lockfile` in `third_party/crates_io/` |
| `incompatible target` warnings | Verify `constraint_values` in your `platform()` |
| Stale Pigweed commit after fork update | Run `bazelisk clean --expunge` then rebuild |
| `local_path_override` not picking up changes | Bazel caches aggressively; try `bazelisk clean` |
