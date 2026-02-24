// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]

use core::ops::Range;
use memory_config::{MemoryRegion, MemoryRegionType};

pub use kernel_config::{
    ExceptionMode, KernelConfigInterface, MTimeTimerConfigInterface, PlicConfigInterface,
    RiscVKernelConfigInterface,
};

// TODO(cfrantz): Make the flash address base feature-configurable so that if
// we link in the virtual window, the base is relocated to the correct address.
const FLASH_BASE: usize = 0xA000_0000;
const FLASH_SIZE: usize = 0x0008_0000;

const UART0_BASE: usize = 0x4000_0000;
const UART0_SIZE: usize = 0x40;

const TIMER_BASE: usize = 0x4010_0000;
const TIMER_SIZE: usize = 0x200;

const PLIC_BASE: usize = 0x4800_0000;
const PLIC_SIZE: usize = 0x0800_0000;

pub struct KernelConfig;

impl KernelConfigInterface for KernelConfig {
    #[cfg(feature = "silicon")]
    const SYSTEM_CLOCK_HZ: u64 = 100_000_000;
    #[cfg(feature = "fpga")]
    const SYSTEM_CLOCK_HZ: u64 = 6_000_000;
    #[cfg(feature = "verilator")]
    const SYSTEM_CLOCK_HZ: u64 = 125_000;
}

impl RiscVKernelConfigInterface for KernelConfig {
    type Timer = TimerConfig;

    const MTIME_HZ: u64 = KernelConfig::SYSTEM_CLOCK_HZ;
    const PMP_ENTRIES: usize = 16;
    const PMP_USERSPACE_ENTRIES: Range<usize> = Range {
        start: 3usize,
        end: 15usize,
    };
    const PMP_GRANULARITY: usize = 0;
    const KERNEL_MEMORY_REGIONS: &'static [MemoryRegion] = &[
        MemoryRegion::new(
            MemoryRegionType::ReadWriteData,
            FLASH_BASE,
            FLASH_BASE + FLASH_SIZE,
        ),
        MemoryRegion::new(
            MemoryRegionType::Device,
            UART0_BASE,
            UART0_BASE + UART0_SIZE,
        ),
        MemoryRegion::new(
            MemoryRegionType::Device,
            TIMER_BASE,
            TIMER_BASE + TIMER_SIZE,
        ),
        MemoryRegion::new(MemoryRegionType::Device, PLIC_BASE, PLIC_BASE + PLIC_SIZE),
    ];

    fn get_exception_mode() -> ExceptionMode {
        ExceptionMode::Vectored(unsafe { MTVEC_TABLE.as_ptr() as usize })
    }
}

pub struct PlicConfig;

impl PlicConfigInterface for PlicConfig {
    const PLIC_BASE_ADDRESS: usize = PLIC_BASE;
    const MAX_IRQS: u32 = 186;
}

pub struct TimerConfig;

impl MTimeTimerConfigInterface for TimerConfig {
    const MTIME_REGISTER: usize = TIMER_BASE + 0x110;
    const MTIMECMP_REGISTER: usize = TIMER_BASE + 0x118;
    const TIMER_CTRL_REGISTER: usize = TIMER_BASE + 0x04;
    const TIMER_INTR_ENABLE_REGISTER: usize = TIMER_BASE + 0x100;
    const TIMER_INTR_STATE_REGISTER: usize = TIMER_BASE + 0x104;
}

unsafe extern "C" {
    #[link_name = "_mtvec_table"]
    static MTVEC_TABLE: [u32; 32];
}
