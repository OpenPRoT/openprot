# AST1060 Target Review Notes

Date: 2026-02-12 (Updated)

## ✅ What's Done

| File | Status | Notes |
|------|--------|-------|
| `BUILD.bazel` | ✓ | Platform definition for Cortex-M4F, semihosting console |
| `defs.bzl` | ✓ | Standard `TARGET_COMPATIBLE_WITH` pattern |
| `config.rs` | ✓ | Implements `KernelConfigInterface`, `CortexMKernelConfigInterface`, `NvicConfigInterface` |
| `entry.rs` | ✓ | Uses `cortex_m_rt::entry`, proper `pw_assert_HandleFailure` |
| `target.ld.tmpl` | ✓ | Complete linker script template with `cortex-m-rt` symbols |
| `threads/kernel/BUILD.bazel` | ✓ | Has `system_image`, `system_image_test`, `rust_binary_no_panics_test` |
| `threads/kernel/system.json5` | ✓ | Proper memory layout (vector table at 0x0, flash after, RAM following) |
| `threads/kernel/target.rs` | ✓ | Correct test target with semihosting exit |
| `MODULE.bazel` | ✓ | Cortex-M4 toolchain, `thumbv7em-none-eabi` triple |

## ✅ Fixed Issues (2026-02-12)

### 1. ~~CPU Architecture Mismatch~~ — ALIGNED WITH IN-TREE

Using Cortex-M3 for **soft-float ABI** compatibility with `thumbv7m-none-eabi` Rust target (matches in-tree `steven/pw_kernel/target/ast1060/`):

```starlark
constraint_values = [
    # Use cortex-m3 to get soft-float ABI (compatible with thumbv7m-none-eabi Rust target)
    # The actual AST1060 has a Cortex-M4F with FPU, but QEMU emulation works fine with M3 settings
    "@pigweed//pw_build/constraints/arm:cortex-m3",
    "@platforms//cpu:armv7-m",
    ...
]
```

MODULE.bazel uses:
- Toolchain: `cc_toolchain_cortex-m3`
- Rust triple: `thumbv7m-none-eabi`

## ⚠️ Remaining Items

### 1. UART Console Backend (Optional for QEMU)

Currently uses semihosting console:
```starlark
"@pigweed//pw_kernel/subsys/console:console_backend": 
    "@pigweed//pw_kernel/subsys/console:console_backend_semihosting",
```

This is sufficient for QEMU testing. For real hardware, add `uart.rs` and `console_backend.rs` using the AST1060 PAC.

### 2. AST1060 PAC Already Configured

The PAC is already set up in `MODULE.bazel`:
```starlark
ast1060_pac_ext = use_extension(
    "//third_party:ast1060_pac.bzl",
    "ast1060_pac_ext",
)
use_repo(ast1060_pac_ext, "ast1060_pac", "vcell")
```

To use it, add to deps: `"@ast1060_pac//:ast1060_pac"`

### 3. MPU Configuration (Future)

For full memory isolation, add an `mpu.rs` file that:
- Configures MPU regions for kernel/user separation
- Sets up memory attributes (XN, cacheable, etc.)

### 4. NvicConfigInterface

Currently empty impl — verify if constants are needed:
```rust
impl NvicConfigInterface for NvicConfig {
    // AST1060 supports up to 480 external interrupts.
}
```

## 📁 Reference: In-Tree AST1060 Target

The complete AST1060 target in `steven/pw_kernel/target/ast1060/` has:

```
ast1060/
├── BUILD.bazel
├── README.md
├── config.rs
├── console_backend.rs      ← UART console (uses PAC)
├── defs.bzl
├── entry.rs
├── uart.rs                 ← UART driver (uses PAC)
├── target.ld.tmpl
├── hello_user/
├── ipc/
├── threads/
├── thread_termination/
└── unittest_runner/
```

## Action Items

1. [x] Fix CPU constraints (aligned with in-tree: Cortex-M3 for soft-float ABI)
2. [x] Rename target from ast1030 to ast1060
3. [x] Update MODULE.bazel toolchain and Rust triple
4. [x] Build verified: `bazelisk build //target/ast1060/threads/kernel:threads --platforms=//target/ast1060:ast1060`
5. [ ] Implement `uart.rs` and `console_backend.rs` using PAC (optional)
6. [ ] Add MPU configuration (`mpu.rs`) (optional)
7. [ ] Verify `NvicConfigInterface` requirements
8. [ ] Add README.md with build/run instructions
