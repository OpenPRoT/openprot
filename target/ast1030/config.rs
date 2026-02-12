// Licensed under the Apache-2.0 license

//! AST1030 BMC kernel configuration.
//!
//! Configures the Pigweed kernel for the ASPEED AST1030 BMC SoC (ARM Cortex-M4).
//! The AST1030 has 768KB SRAM and executes from RAM (non-XIP).
//!
//! QEMU emulation uses 12 MHz clock; real silicon runs at 200 MHz.

#![no_std]

pub use kernel_config::{
    CortexMKernelConfigInterface, KernelConfigInterface, NvicConfigInterface,
};

/// AST1030 kernel configuration.
pub struct KernelConfig;

impl KernelConfigInterface for KernelConfig {
    // QEMU AST1030 runs at 12 MHz.
    const SYSTEM_CLOCK_HZ: u64 = 12_000_000;
}

impl CortexMKernelConfigInterface for KernelConfig {
    const SYS_TICK_HZ: u32 = KernelConfig::SYSTEM_CLOCK_HZ as u32;
    const NUM_MPU_REGIONS: usize = 8;
}

/// AST1030 NVIC configuration.
pub struct NvicConfig;

impl NvicConfigInterface for NvicConfig {
    // AST1030 supports up to 480 external interrupts.
}
