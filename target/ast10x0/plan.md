# target/ast10x0 — Implementation Plan

## Goal

Create `target/ast10x0/` in this repo — a Pigweed kernel target for the ASPEED AST10x0
(Cortex-M4) — self-contained in this repo with no Bazel dependency on any other target.

`pw_kernel/target/ast1030` in the Pigweed tree was used as a **code authoring reference**
only (same SoC family, same ARMv7-M arch). It is not a build-time dependency and does not
need to exist in the Pigweed tree for this target to build.

---

## Hardware Summary

| Property | Value |
|---|---|
| SoC family | ASPEED AST10x0 (AST1030 Cortex-M4F) |
| Architecture | ARMv7-M (`thumbv7m-none-eabi`, soft-float ABI) |
| CPU | Cortex-M4 @ 200 MHz |
| SRAM | 768 KB (non-XIP; code and data both in SRAM) |
| MPU | PMSAv7, 8 regions |
| Interrupt controller | NVIC (480 interrupts) |
| QEMU machine | `ast1030-evb`, `--cpu cortex-m4` |
| Peripheral register access | [`ast1060-pac`](https://github.com/AspeedTech-BMC/ast1060-pac) (svd2rust-generated PAC, v0.1.0) |

---

## Directory Structure (completed)

```
target/ast10x0/
├── BUILD.bazel              ✅ platform, constraint_value, entry/config/linker targets
├── defs.bzl                 ✅ TARGET_COMPATIBLE_WITH for :target_ast10x0
├── config.rs                ✅ KernelConfig (200 MHz / 12 MHz QEMU, 8 MPU regions)
├── entry.rs                 ✅ cortex-m-rt entry + pw_assert_HandleFailure
├── target.ld.tmpl           ✅ ARMv7-M RAM-based linker script template
├── plan.md                  ✅ This file
└── tests/
    ├── ipc/user/            ✅ User-space IPC test (initiator + handler)
    │   ├── BUILD.bazel
    │   ├── system.json5
    │   └── target.rs
    ├── threads/kernel/      ✅ Kernel threading test
    │   ├── BUILD.bazel
    │   ├── system.json5
    │   └── target.rs
    └── unittest_runner/     ✅ Kernel unit test runner
        ├── BUILD.bazel
        ├── system.json5
        └── target.rs
```

---

## Remaining Work

### Step 1 — MODULE.bazel

Four changes needed:

1. **Update the Pigweed commit pin** to include ARMv7-M kernel support.

   The current pin in `MODULE.bazel` is:
   ```python
   git_override(
       module_name = "pigweed",
       commit = "a7059ed6124319250a9c102dd9f0514d8a65be4d",
       remote = "https://pigweed.googlesource.com/pigweed/pigweed",
   )
   ```
   This predates the ARMv7-M work. The required commits are already on pigweed `main`:

   | Commit | Description |
   |---|---|
   | `e6a6bcc12` | `pw_kernel: Add ARMv7-M architecture support` |
   | `f572d358e` | `pw_kernel: Add process and thread objects` |

   **`pw_kernel/target/ast1030` does not need to be in the Pigweed tree.** The ast10x0
   target is fully self-contained here. Only the arch and kernel libraries are needed.

   **Action**: advance the pin to any commit at or after `f572d358e` on `main`
   (current tip is `1fbf83ca3`):
   ```python
   git_override(
       module_name = "pigweed",
       commit = "1fbf83ca3ffb9631e4450af2cd4396d426363b1b",  # or newer main tip
       remote = "https://pigweed.googlesource.com/pigweed/pigweed",
   )
   ```

2. Add `thumbv7m-none-eabi` to `crate.from_cargo(supported_platform_triples)`:
   ```python
   "thumbv7m-none-eabi",
   ```

3. Add `ast1060-pac` to `third_party/crates_io/Cargo.toml`:
   ```toml
   # aspeed-pac is not published to crates.io — pin via git.
   # Enable the `rt` feature to get cortex-m-rt device interrupt vector support.
   ast1060-pac = { git = "https://github.com/AspeedTech-BMC/ast1060-pac", features = ["rt"] }
   ```
   Because it is git-only, also add a `git_override` (or `git_repository`) entry in
   `MODULE.bazel` so `crate_universe` can resolve it:
   ```python
   crate.annotation(
       crate = "ast1060-pac",
       version = "0.1.0",
   )
   ```
   Then re-pin the lockfile:
   ```bash
   bazel run @rules_rust//crate_universe:vendor
   ```
   and commit the updated `third_party/crates_io/Cargo.lock`.

4. Register the Pigweed ARM Clang CC toolchain. Confirm the exact label:
   ```python
   register_toolchains(
       ...
       "@pigweed//pw_toolchain/arm_clang:arm_clang_cc_toolchain_cortex-m",  # verify label
   )
   ```

### Step 2 — `.bazelrc` (or `pigweed.json` workflow flags)

Add Bazel configs matching the ast1030 pattern in `pigweed/pw_kernel/kernel.bazelrc`:

```
# AST10x0 target (ARMv7-M Cortex-M4)
common:k_ast10x0 --platforms=//target/ast10x0:ast10x0
common:k_ast10x0 --//pw_toolchain:cortex-m_toolchain_kind=clang

# QEMU AST10x0
common:k_qemu_ast10x0 --config=k_ast10x0
common:k_qemu_ast10x0 --//target/ast10x0:qemu=true
test:k_qemu_ast10x0 --run_under="@pigweed//pw_kernel/tooling:qemu \
  --cpu cortex-m4 \
  --machine ast1030-evb \
  --semihosting \
  --image "
```

### Step 3 — `workflows.json`

Add a `qemu_ast10x0_tests` workflow entry mirroring the earlgrey verilator workflow.

### Step 4 — Additional tests

Once the core target builds, add remaining tests following the same pattern (using
`pw_kernel/target/ast1030` in the local Pigweed checkout as a code reference):

| Test | Path | Notes |
|---|---|---|
| `async_ipc` | `tests/async_ipc/user/` | 4-app layout |
| `interrupts` | `tests/interrupts/kernel/` and `user/` | Requires IRQ 42 handler |
| `process_termination` | `tests/process_termination/` | kernel + user variants |
| `thread_termination` | `tests/thread_termination/kernel/` | |
| `stress/ipc` | `tests/stress/ipc/user/` | |
| `stress/mutex` | `tests/stress/mutex/kernel/` | |
| `wait_group` | `tests/wait_group/` | |

### Step 5 — Peripheral register access (`ast1060-pac`)

Unlike the earlgrey target (which uses `ureg`-generated registers under `registers/`),
the AST10x0 target uses **[`ast1060-pac`](https://github.com/AspeedTech-BMC/ast1060-pac)**
— the svd2rust-generated Peripheral Access Crate for the ASPEED AST1060 SoC family.

Key facts about the crate:
- **Version**: `0.1.0`; authored by ASPEED (`AspeedTech-BMC` org, contributors include project
  members)
- **Source**: GitHub only — not published to crates.io; must be referenced via `git` in
  `Cargo.toml`
- **Generated from**: `ast1060.svd` (SVD file in the repo root); peripherals include I2C
  (with filter registers), timers, and others defined in the SVD
- **`rt` feature**: enables `cortex-m-rt/device` integration (interrupt vector support);
  always enable this for kernel use
- **`no_std`**: yes; uses `vcell` for volatile register reads/writes
- **Direct deps**: `cortex-m 0.7`, `vcell 0.1.2`, optional `critical-section 1.0`

Usage in the target:
- Reference peripherals as `use ast1060_pac::{Peripherals, ...}`.
- Add to any `BUILD.bazel` that needs register access:
  ```python
  "@rust_crates//:ast1060-pac",
  ```
- No `registers/` subtree is needed; the PAC replaces it entirely (contrast with earlgrey's
  `ureg`-based `registers/` directory).

### Step 6 — Console backend for silicon

The current `BUILD.bazel` uses `console_backend_semihosting`. For real AST10x0
hardware, add a `console.rs` implementing the UART backend via `ast1060-pac`
(e.g. `ast1060_pac::Uart0`) and expose it as a `rust_library(:console)` target,
then update the platform `console_backend` flag to point at it.

---

## Open Questions

1. **`ast1060-pac` peripheral coverage** — the crate is generated from `ast1060.svd` and
   covers the AST1060 SoC; verify that all peripherals needed for AST10x0 bring-up
   (UART, I2C, timers, GPIO, etc.) are present in the SVD before relying on it for
   silicon. The SVD was last updated 9 months ago (commit `35ce819`).
2. **ARM CC toolchain label** — exact `register_toolchains` label for Pigweed's ARM Clang.
   Check `@pigweed//pw_toolchain/arm_clang/BUILD.bazel`.
3. **SRAM addresses** — the `system.json5` files use AST1030 addresses verbatim.
   Verify against AST10x0 production datasheet before silicon bring-up.
4. **`constraint_value` visibility** — if the ast10x0 constraint needs to be referenced
   from outside this package, add `visibility = ["//visibility:public"]` to
   `BUILD.bazel`'s `constraint_value`.

---

## Build Validation Commands

```bash
# Build the platform target (cross-compilation check)
bazel build //target/ast10x0/... --config=k_ast10x0

# Run tests under QEMU
bazel test //target/ast10x0/... --config=k_qemu_ast10x0

# Specific tests
bazel test //target/ast10x0/tests/ipc/user:ipc_test --config=k_qemu_ast10x0
bazel test //target/ast10x0/tests/threads/kernel:threads_test --config=k_qemu_ast10x0
bazel test //target/ast10x0/tests/unittest_runner:unittest_runner --config=k_qemu_ast10x0
```
