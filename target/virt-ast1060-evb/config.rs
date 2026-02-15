// Licensed under the Apache-2.0 license

//! Virtual AST1060-EVB kernel configuration for QEMU testing.
//!
//! Configures the Pigweed kernel for QEMU emulation of the ASPEED AST1060.
//! Uses semihosting console for stdio instead of UART.
//! No hardware initialization is performed - relies on QEMU defaults.
//!
//! QEMU emulation uses 12 MHz clock.

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
