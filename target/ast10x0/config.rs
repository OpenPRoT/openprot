// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Static kernel configuration for ASPEED AST10x0 target.
//!
//! Cortex-M4 BMC SoC @ 200 MHz, 768 KB SRAM.
//! With the `qemu` feature: 12 MHz (QEMU ast1030-evb SysTick).
//!
//! TODO(ast10x0): Verify SYSTEM_CLOCK_HZ and NUM_MPU_REGIONS against
//! the production AST10x0 hardware datasheet.
#![no_std]

pub use kernel_config::{CortexMKernelConfigInterface, KernelConfigInterface, NvicConfigInterface};

pub struct KernelConfig;

impl CortexMKernelConfigInterface for KernelConfig {
    #[cfg(not(feature = "qemu"))]
    const SYS_TICK_HZ: u32 = 200_000_000;
    #[cfg(feature = "qemu")]
    const SYS_TICK_HZ: u32 = 12_000_000;
    const NUM_MPU_REGIONS: usize = 8;
}

impl KernelConfigInterface for KernelConfig {
    const SYSTEM_CLOCK_HZ: u64 = KernelConfig::SYS_TICK_HZ as u64;

    // The pigweed default kernel stack is 2 KiB, which is too small for the
    // bootstrap thread that runs the peripheral KAT suites. The HACE SHA-2/HMAC
    // test in particular builds a single large stack frame (~3.6 KiB) plus a
    // ~1.3 KiB `HaceHmacCtx` (1 KiB message buffer) and nested device locals,
    // overflowing a 2 KiB stack into adjacent `.bss` (`INPUT_BUF`) and faulting
    // with an unaligned access on the first case that writes that buffer
    // (`sha256 stream-9000`). 16 KiB gives comfortable headroom for these
    // crypto workloads. The AST10x0 has 768 KiB SRAM, so the extra kernel-stack
    // RAM is negligible.
    const KERNEL_STACK_SIZE_BYTES: usize = 16384;
}

pub struct NvicConfig;
// Uses default configuration (480 interrupts).
impl NvicConfigInterface for NvicConfig {}
