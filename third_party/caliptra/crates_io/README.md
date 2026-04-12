# Caliptra crate_universe workspaces

Two parallel Cargo workspaces drive the `rules_rust` crate_universe
extensions for caliptra-scoped Rust crates:

- `embedded/` → `@rust_caliptra_crates` (host + riscv32imc triples,
  no alloc/std feature leak). Resolves all caliptra + MCU crates
  that reach embedded firmware.
- `host/` → `@rust_caliptra_crates_host` (host triples only, full
  feature set). Resolves host tools, rustcrypto crates with
  `ecdsa`/`pem`, and the `caliptra-emu-*` chain.

## Why two workspaces?

Cargo feature unification is global across a workspace. Host features
like `std`, `alloc`, `ecdsa/pem`, and `caliptra-lms-types/std` would
back-propagate onto embedded rlibs and push `caliptra_rom` over the
96 KB ROM budget enforced by
`sw-emulator/lib/periph/src/root_bus.rs`.

rules_rust 0.68.1's `crate.annotation(crate_features = [...])` is
additive-only and cannot subtract features after Cargo resolution,
so a single-workspace layout has no mechanism to keep host features
off embedded rlibs.

The split can be collapsed only if: (1) `rules_rust` gains a
subtractive `crate.annotation` field that removes features after
Cargo resolution, OR (2) upstream caliptra crates split host-only
features (`std`, `alloc`, `ecdsa/pem`) into separate crates that
embedded consumers can simply omit, OR (3) the emulator `ROM_SIZE`
budget is raised. None of these are close.

## How the pieces wire together

- `../MODULE.bazel` declares the two `crate.from_cargo` extensions
  (`rust_caliptra_crates`, `rust_caliptra_crates_host`) that consume
  `embedded/Cargo.toml` and `host/Cargo.toml` respectively.
- `../BUILD.bazel` defines the platform-switching `crate_*` alias
  layer that routes shared crate names to the correct per-platform
  rlib via a `select()` on `:is_riscv32imc`.
- `../caliptra-sw/BUILD.bazel` holds the hand-written `rust_library`
  wrappers for ROM-budget-blocked crates (`caliptra_drivers`,
  `caliptra_cfi_lib_git`, `caliptra_common`, `caliptra_kat`,
  `caliptra_image_verify`, and their FMC/runtime variants). Those
  wrappers bypass Cargo feature resolution entirely so the `cfi`
  runtime assertions stay out of the ROM binary.
- `../caliptra-sw/BUILD.bazel` also hosts the hand-written
  `caliptra_emu_*` wrappers, which mix types from both workspaces
  at trait boundaries — another reason the two workspaces cannot
  collapse without introducing TypeId mismatches.

## Shared crates

Crates referenced from BOTH workspaces (currently `bitfield`,
`bitflags`, `cfg-if`, `log`, `memoffset`, `registers-generated`,
`serde`, `serde_derive`, `smlang`, `tock-registers`, `zerocopy`,
`zeroize`) must be routed through the platform-switching `crate_*`
alias layer in `third_party/caliptra/BUILD.bazel` — not the raw
`@rust_caliptra_crates*//:<name>` label — or they will trigger
cross-workspace TypeId mismatches.
