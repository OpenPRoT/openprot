// Licensed under the Apache-2.0 license

//! Static kernel configuration for ASPEED AST1060-EVB target.
//!
//! The AST1060 is a Cortex-M4 based BMC SoC running at 200 MHz with 768KB SRAM.
//! For QEMU emulation, we use 12 MHz (LM3S6965EVB compatible clock) since QEMU's
//! ast1030-evb machine uses the LM3S6965 SysTick implementation. (AST1060 is compatible)

#![no_std]

pub use kernel_config::{
    CortexMKernelConfigInterface, KernelConfigInterface, NvicConfigInterface,
};

pub struct KernelConfig;

impl CortexMKernelConfigInterface for KernelConfig {
    /// SysTick clock frequency in Hz.
    /// Using 12 MHz for QEMU compatibility (LM3S6965EVB SysTick clock).
    /// Real AST1060 hardware runs at 200 MHz.
    const SYS_TICK_HZ: u32 = 12_000_000;

    /// Number of MPU regions available.
    /// ARM Cortex-M4 with PMSAv7 has 8 regions.
    const NUM_MPU_REGIONS: usize = 8;
}

impl KernelConfigInterface for KernelConfig {
    /// System clock frequency in Hz.
    const SYSTEM_CLOCK_HZ: u64 = KernelConfig::SYS_TICK_HZ as u64;
}

pub struct NvicConfig;

// Uses the default configuration (480 interrupts).
impl NvicConfigInterface for NvicConfig {}
