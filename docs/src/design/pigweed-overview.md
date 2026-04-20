# Pigweed Integration Overview

OpenPRoT uses [Pigweed](https://pigweed.dev) for its build infrastructure,
kernel, logging, and shared `Result` type. This page summarizes what comes
from Pigweed, where it is pinned, and which OpenPRoT directories consume it.

## Pinning

Pigweed is a Bazel module dependency pinned to an explicit upstream commit:

```starlark
# Example from MODULE.bazel
bazel_dep(name = "pigweed")

git_override(
    module_name = "pigweed",
    commit = "cf8b16a4fbf7bc6cb76b804acd510c7cb3adf995",
    remote = "https://pigweed.googlesource.com/pigweed/pigweed",
)
```

See `MODULE.bazel`. Use the canonical `pigweed.googlesource.com` remote when
reading upstream. The GitHub mirror is read-only and not the authoritative
source.

## What OpenPRoT uses

### `pw_kernel` - microkernel

OpenPRoT uses `pw_kernel` as its microkernel. The kernel provides scheduling,
IPC channels, interrupt objects, userspace isolation, and the `#[entry]` /
`#[process_entry]` macros used by application code. System images are
described declaratively in `system.json5` files and assembled by Pigweed's
`system_image()` macro.

### `pw_log` - structured logging

`@pigweed//pw_log/rust:pw_log` is the logging surface used throughout
userspace and target code.

### `pw_status` - shared result type

`@pigweed//pw_status/rust:pw_status` provides `pw_status::Error` and the
`Result<T, pw_status::Error>` alias that every syscall and IPC entry point
returns. The error variants map cleanly to the kernel's rejection reasons
(`InvalidArgument`, `Internal`, `DeadlineExceeded`, …).

### Toolchains

Pigweed owns both the Rust and C/C++ toolchains used by the build:

- Rust toolchain is registered via the `pw_rust` module extension;
  `pw_rust_toolchains//:all` is registered as a Bazel toolchain.
- Host C/C++: `@pigweed//pw_toolchain/host_clang:host_cc_toolchain_linux` and
  `@pigweed//pw_toolchain/host_clang:host_cc_toolchain_macos`.
- RISC-V C/C++ for the OpenPRoT target:
  `@pigweed//pw_toolchain/riscv_clang:riscv_clang_cc_toolchain_rv32imc`.

### Crate universe override

Pigweed ships its own `rust_crates` extension for crates its Rust code uses.
Upstream's default set does not currently include
`riscv32imc-unknown-none-elf`, so OpenPRoT overrides the Pigweed repo to
point at its own `@rust_crates` workspace.

### `./pw` workflow launcher

`./pw` is a shell wrapper around `bazelisk run //:pw -- "$@"`. It dispatches
to the named groups defined in `workflows.json`:

- `./pw presubmit` - format check, license check, clippy aspect build.
- `./pw default` - wildcard build with hardware/disabled filtered out.
- `./pw ci` - CI test suite, same exclusions plus `-verilator`.
- `./pw upstream_pigweed` - `ci` plus the Earlgrey verilator tests.

See `workflows.json` for the group definitions and `usage.md` for the everyday
command surface.

## Where to go next

- `design/pw-kernel-ipc.md` - concrete `pw_kernel` IPC walkthrough.
- `usage.md` - `./pw` and `bazel` command reference.
- `architecture.md` - how the Pigweed pieces slot into the openprot tree.
