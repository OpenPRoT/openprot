# AST1060 Target Guide

This document explains the out-of-tree Pigweed kernel targets for AST1060.

## Overview

There are two AST1060 targets in this workspace:

| Target | Purpose | Console | Use Case |
|--------|---------|---------|----------|
| `ast1060` | QEMU emulation | Semihosting | Development, CI testing |
| `ast1060-evb` | Physical hardware | UART | Real board testing |

## Architecture

```
bazel-stuff/
├── target/
│   ├── ast1060/              # QEMU target
│   │   ├── BUILD.bazel       # Platform + rust_library targets
│   │   ├── defs.bzl          # TARGET_COMPATIBLE_WITH constraint
│   │   ├── config.rs         # KernelConfig (SysTick, MPU regions)
│   │   ├── entry.rs          # Reset handler + boot sequence
│   │   ├── target.ld.tmpl    # Linker script template
│   │   └── threads/kernel/   # Threads test application
│   │       ├── BUILD.bazel
│   │       ├── system.json5  # Memory layout
│   │       └── target.rs     # Application entry point
│   │
│   └── ast1060-evb/          # Physical board target
│       ├── BUILD.bazel
│       ├── defs.bzl
│       ├── config.rs
│       ├── entry.rs          # Initializes UART before kernel
│       ├── console_backend.rs # UART driver via aspeed-ddk
│       ├── target.ld.tmpl
│       └── threads/kernel/
│           ├── BUILD.bazel
│           ├── system.json5
│           └── target.rs
```

## How It Works

### 1. Platform Definition (`BUILD.bazel`)

Each target defines a Bazel platform with CPU and OS constraints:

```python
platform(
    name = "ast1060",
    constraint_values = [
        "@platforms//cpu:armv7-m",
        "@platforms//os:none",
        "@pigweed//pw_kernel:os_freestanding",
        "//platform:ast1060",          # Custom constraint
    ],
)
```

The platform tells Bazel which toolchain to use (ARM Cortex-M cross-compiler).

### 2. Kernel Configuration (`config.rs`)

Provides hardware-specific settings to the kernel:

```rust
impl KernelConfig for Config {
    const SYSTICK_HZ: u32 = 12_000_000;  // 12 MHz clock
    const ARM_V7_MPU_REGIONS: usize = 8; // MPU region count
}
```

### 3. Boot Entry (`entry.rs`)

The reset handler that runs at power-on:

```rust
#[cortex_m_rt::entry]
fn main() -> ! {
    // For ast1060-evb: Initialize UART here
    uart_init();
    
    // Jump to kernel target
    target_common::enter_kernel_main()
}
```

### 4. Console Backend

**QEMU (`ast1060`):** Uses ARM semihosting - debug output goes through QEMU to host terminal.

**Physical Board (`ast1060-evb`):** Uses UART via aspeed-ddk driver:

```rust
// console_backend.rs
static UART: Mutex<RefCell<Option<Uart>>> = ...;

pub fn uart_write(data: &[u8]) {
    critical_section::with(|cs| {
        if let Some(ref mut uart) = *UART.borrow_ref_mut(cs) {
            for byte in data {
                uart.write_byte(*byte);
            }
        }
    });
}
```

### 5. Memory Layout (`system.json5`)

Defines memory regions for the linker:

```json5
{
    memory: {
        vector_table: { start: 0x00000000, size: 0x400 },
        flash:        { start: 0x00000400, size: 0x000ffc00 },
        ram:          { start: 0x80000000, size: 0x00100000 },
    }
}
```

### 6. Linker Script (`target.ld.tmpl`)

Template processed by Bazel to generate final linker script with memory addresses from `system.json5`.

## Build Commands

### QEMU Target

```bash
# Build
bazelisk build //target/ast1060/threads/kernel:threads --config=k_qemu_ast1060

# Run in QEMU
bazelisk run //target/ast1060/threads/kernel:threads --config=k_qemu_ast1060
```

### Physical Board Target

```bash
# Build
bazelisk build //target/ast1060-evb/threads/kernel:threads --config=k_ast1060_evb

# The output binary is at:
# bazel-bin/target/ast1060-evb/threads/kernel/threads.bin
```

### Output Files

| File | Header | Use Case |
|------|--------|----------|
| `threads.bin` | No | OpenOCD/JTAG flashing, direct memory load |
| `threads_uart.bin` | **Yes** (4-byte size) | UART bootloader upload |

### Generate UART Boot Image

The AST1060 UART bootloader requires a special header format:
- 4 bytes: image size (little-endian, 4-byte aligned)
- N bytes: binary image data  
- 0-3 bytes: zero padding to 4-byte alignment

This is automated via the `uart_boot_image` Bazel rule:

```bash
# Build the UART-bootable image
bazelisk build //target/ast1060-evb/threads/kernel:threads_uart --config=k_ast1060_evb

# Output file:
# bazel-bin/target/ast1060-evb/threads/kernel/threads_uart.bin
```

The rule is defined in `//target:uart_boot_image.bzl` and used in BUILD.bazel:

```python
load("//target:uart_boot_image.bzl", "uart_boot_image")

uart_boot_image(
    name = "threads_uart",
    src = ":threads",  # system_image target
    out = "threads_uart.bin",
)
```

## Flashing to Hardware

After building the ast1060-evb target:

```bash
# Using the UART boot image (with header)
# Upload via serial bootloader tool
uart_upload /dev/ttyUSB0 bazel-bin/target/ast1060-evb/threads/kernel/threads_uart.bin

# Or using OpenOCD with raw binary (no header needed)
openocd -f interface/cmsis-dap.cfg -f target/aspeed_ast1060.cfg \
  -c "program bazel-bin/target/ast1060-evb/threads/kernel/threads.bin verify reset exit"
```

## Serial Console

Connect to the AST1060-EVB serial port:

```bash
# Find the USB-UART device
ls /dev/ttyUSB*

# Connect at 115200 baud
screen /dev/ttyUSB2 115200
# or
picocom -b 115200 /dev/ttyUSB2
```

Expected output:
```
Hello from AST1060-EVB!
=== pw_kernel ===
...
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `@ast1060_pac` | AST1060 Peripheral Access Crate (register definitions) |
| `aspeed-ddk` | ASPEED device driver kit (UART driver) |
| `cortex-m-rt` | Cortex-M runtime (reset handler, vector table) |
| `cortex-m-semihosting` | ARM semihosting support |

## Configuration Files

### `.bazelrc` Configs

```bash
# QEMU config
common:k_qemu_ast1060 --config=k_common
common:k_qemu_ast1060 --platforms=//target/ast1060
run:k_qemu_ast1060 --run_under="@pigweed//pw_kernel/tooling:qemu ..."

# Physical board config
common:k_ast1060_evb --config=k_common
common:k_ast1060_evb --platforms=//target/ast1060-evb
```

### `MODULE.bazel` Dependencies

The AST1060 PAC is fetched via git:

```python
ast1060_pac = use_extension("//third_party:ast1060_pac.bzl", "ast1060_pac")
use_repo(ast1060_pac, "ast1060_pac")
```

## Differences: QEMU vs Physical Board

| Aspect | ast1060 (QEMU) | ast1060-evb (Physical) |
|--------|----------------|------------------------|
| Console | Semihosting | UART @ 115200 |
| Exit | `semihosting::exit()` | Loop or reset |
| Clock | Emulated | Real 12 MHz |
| Debug | GDB via QEMU | JTAG/SWD |
| Build config | `k_qemu_ast1060` | `k_ast1060_evb` |

## Troubleshooting

### Build Errors

**"No matching toolchains found"**
- Ensure `--platforms=//target/ast1060` is specified
- Check that ARM toolchain is configured in MODULE.bazel

**"unresolved import `ast1060_pac`"**
- Run `cargo generate-lockfile` in `third_party/crates_io/`
- Verify `Cargo.toml` has the ast1060-pac dependency

### Runtime Issues

**No UART output on physical board**
- Check baud rate (115200)
- Verify TX/RX pin connections
- Ensure UART is initialized before any print calls

**QEMU hangs**
- Check CPU type matches (`cortex-m4` for ast1030-evb machine)
- Verify semihosting is enabled in run_under config
