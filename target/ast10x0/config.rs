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

pub use kernel_config::{
    CortexMKernelConfigInterface, KernelConfigInterface, NvicConfigInterface,
};

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
}

pub struct NvicConfig;
// Uses default configuration (480 interrupts).
impl NvicConfigInterface for NvicConfig {}
