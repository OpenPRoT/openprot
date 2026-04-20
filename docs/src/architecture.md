# Architecture

openprot is a `no_std` Rust firmware project for Platform Root-of-Trust
devices, organized as a Bazel module so that HAL traits, OS-abstraction
traits, OS-agnostic services, and target-specific glue can stay decoupled.

## Repository layout

```
openprot/
├── MODULE.bazel              # Bazel module + crate_universe extensions
├── workflows.json            # Pigweed workflow groups (./pw …)
├── BUILD.bazel               # Top-level Bazel package
├── openprot/                 # Main application crate (lib.rs + main.rs)
├── hal/
│   ├── async/                # Async HAL trait crates
│   ├── blocking/             # Blocking HAL trait crates
│   └── nb/                   # Non-blocking (nb) HAL trait crates
├── platform/
│   ├── traits/               # OS-abstraction traits
│   └── impls/                # Concrete impls per host environment
├── services/
│   ├── storage/              # OS-agnostic services on top of HAL + platform
│   └── telemetry/
├── target/
│   ├── earlgrey/             # OpenTitan Earl Grey target
│   └── veer/                 # Caliptra VeeR-EL2 target (pw_kernel-based)
├── third_party/
│   ├── pigweed/              # Pigweed integration
│   └── caliptra/             # caliptra-sw / caliptra-mcu-sw integration
├── presubmit/                # Python presubmit scripts
└── docs/                     # mdbook sources (this site)
```

The intent of the split is that anything touching hardware registers goes
through a `hal/` trait, anything touching the host OS goes through a
`platform/traits/` trait, and reusable logic lives in `services/`. Targets
under `target/` provide the silicon-specific glue (linker scripts, ePMP
setup, console wiring, register definitions) without leaking those details
into the rest of the tree.

## Build system

The build is driven by Bazel via Pigweed's workflow launcher; there is no
Cargo workspace. The relevant pieces:

- `MODULE.bazel` declares Bazel module dependencies (`bazel_skylib`,
  `pigweed`, `platforms`, `rules_rust`, `rules_rust_mdbook`, `rules_python`,
  plus `caliptra_deps` and `ureg` via overrides).
- `git_override` pins Pigweed to a specific upstream commit
  (`MODULE.bazel:23-27`), keeping the kernel/build/log/status/toolchain
  pieces reproducible.
- `pw_rust.toolchain` registers Pigweed's managed Rust toolchains
  (`MODULE.bazel:35-37`); host C/C++ toolchains and the `rv32imc` RISC-V
  C/C++ toolchain are registered via `register_toolchains`
  (`MODULE.bazel:39-44`).
- Three `crate_universe` workspaces govern Rust crate dependencies:
  - `@rust_crates` — cross-platform crates declared in
    `third_party/crates_io/Cargo.toml`.
  - `@rust_caliptra_crates` — embedded Caliptra crates (rv32imc, no
    std/alloc).
  - `@rust_caliptra_crates_host` — Caliptra host tools (need std).
- `./pw` is `bazelisk run //:pw -- "$@"` and dispatches to the named groups
  in `workflows.json` (`presubmit`, `default`, `ci`, `upstream_pigweed`).

See `usage.md` for the everyday command surface and `workflows.json` for the
authoritative workflow definitions.

## Pigweed integration

The veer target builds on Pigweed's microkernel and runtime crates:

- `@pigweed//pw_kernel` — the microkernel running on `target/veer/`.
  Provides scheduling, IPC channels, interrupt objects, userspace
  isolation, and the `#[entry]` / `#[process_entry]` macros. System images
  are described declaratively in `system.json5` files (see `target/veer/`
  for examples) and assembled by Pigweed's `system_image()` macro.
- `@pigweed//pw_log/rust:pw_log` — structured logging used throughout
  userspace and target code.
- `@pigweed//pw_status/rust:pw_status` — `Result<T, pw_status::Error>` is
  the syscall and IPC return type.
- `@pigweed//pw_toolchain/riscv_clang:riscv_clang_cc_toolchain_rv32imc` —
  RISC-V C/C++ toolchain used by the veer target's mixed-language build.

For a concrete cross-process example, see `design/pw-kernel-ipc.md` and
`target/veer/ipc/`.

## Targets

Each silicon/SoC target lives under `target/<name>/` and provides its own
linker script, entry point, `defs.bzl` helpers, register definitions, and
test/runner tooling.

- `target/earlgrey/` — OpenTitan Earl Grey. Verilator-driven tests are
  gated behind the `verilator` tag and only run via the
  `earlgrey_verilator_tests` build (see `workflows.json`).
- `target/veer/` — Caliptra VeeR-EL2. Built on `pw_kernel` and exercised
  on the Caliptra emulator via `target/veer/tooling/caliptra_runner.bzl`.

When adding a new target, prefer the `target/<name>/defs.bzl` helpers over
hand-rolled `rust_binary` rules, and always set `target_compatible_with =
TARGET_COMPATIBLE_WITH` so wildcard host builds skip target-only crates.

## Third-party integration

- `third_party/pigweed/` — local Pigweed integration including a
  `visibility.patch`.
- `third_party/caliptra/` — integration with `caliptra-sw` and
  `caliptra-mcu-sw`. Version bumps go through `uprev.py`, which writes both
  `versions.bzl` (commit pins) and the Caliptra `Cargo.lock` files.
- `third_party/crates_io/` — pinned crates.io dependencies for the main
  workspace; `third_party/caliptra/crates_io/{embedded,host}/` for the two
  Caliptra crate hubs.

## Where to read next

- `coding-style.md` — formatter configs and Rust conventions.
- `contributing.md` and `development-process.md` — review and merge process.
- `design/` — focused notes on individual subsystems (e.g. pw_kernel IPC).
