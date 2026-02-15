// Licensed under the Apache-2.0 license

//! AST1060 BMC kernel configuration.
//!
//! Configures the Pigweed kernel for the ASPEED AST1060 BMC SoC.
//! The actual chip has a Cortex-M4F with FPU, but we use Cortex-M3 settings
//! for soft-float ABI compatibility with thumbv7m-none-eabi Rust target.
//! The AST1060 has 768KB SRAM and executes from RAM (non-XIP).
//!
//! QEMU emulation uses 12 MHz clock; real silicon runs at 200 MHz.

#![no_std]

pub use kernel_config::{
    CortexMKernelConfigInterface, KernelConfigInterface, NvicConfigInterface,
};

/// AST1060 kernel configuration.
pub struct KernelConfig;

impl KernelConfigInterface for KernelConfig {
    // QEMU AST1060 runs at 12 MHz.
    const SYSTEM_CLOCK_HZ: u64 = 12_000_000;
}

impl CortexMKernelConfigInterface for KernelConfig {
    const SYS_TICK_HZ: u32 = KernelConfig::SYSTEM_CLOCK_HZ as u32;
    const NUM_MPU_REGIONS: usize = 8;
}

/// AST1060 NVIC configuration.
pub struct NvicConfig;

impl NvicConfigInterface for NvicConfig {
    // AST1060 supports up to 480 external interrupts.
}
