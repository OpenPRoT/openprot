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

impl CortexMKernelConfigInterface for KernelConfig {
    // QEMU AST1060 runs at 12 MHz.
    const SYS_TICK_HZ: u32 = 12_000_000;
    const NUM_MPU_REGIONS: usize = 8;
}

impl KernelConfigInterface for KernelConfig {
    const SYSTEM_CLOCK_HZ: u64 = KernelConfig::SYS_TICK_HZ as u64;
}

/// AST1060 NVIC configuration.
pub struct NvicConfig;

// Uses default configuration (480 interrupts).
impl NvicConfigInterface for NvicConfig {}
