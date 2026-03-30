// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]

use core::ops::Range;
use memory_config::MemoryRegion;

pub use kernel_config::{
    ClintTimerConfigInterface, ExceptionMode, KernelConfigInterface, RiscVKernelConfigInterface,
    VeerPicConfigInterface,
};

//const RAM_BASE: usize = 0x4000_0000;
//const RAM_SIZE: usize = 384*1024;
//
//const UART0_BASE: usize = 0x1000_1000;
//const UART0_SIZE: usize = 0x100;
//
//const TIMER_BASE: usize = 0x4010_0000;
//const TIMER_SIZE: usize = 0x200;
//
//const PLIC_SIZE: usize = 0x0000_5400;
const PIC_BASE: usize = 0x6000_0000;

pub struct KernelConfig;

impl KernelConfigInterface for KernelConfig {
    #[cfg(feature = "silicon")]
    const SYSTEM_CLOCK_HZ: u64 = 100_000_000;
    #[cfg(feature = "fpga")]
    const SYSTEM_CLOCK_HZ: u64 = 10_000_000; //FIXME
    #[cfg(feature = "emulator")]
    const SYSTEM_CLOCK_HZ: u64 = 1_000_000;
}

impl RiscVKernelConfigInterface for KernelConfig {
    type Timer = TimerConfig;

    const MTIME_HZ: u64 = KernelConfig::SYSTEM_CLOCK_HZ;
    const PMP_ENTRIES: usize = 16;
    const PMP_USERSPACE_ENTRIES: Range<usize> = Range {
        start: 0usize,
        end: 16usize,
    };
    const PMP_GRANULARITY: usize = 0;
    const KERNEL_MEMORY_REGIONS: &'static [MemoryRegion] = &[];

    fn get_exception_mode() -> ExceptionMode {
        ExceptionMode::Vectored(unsafe { MTVEC_TABLE.as_ptr() as usize })
    }
}

pub struct VeerPicConfig;

impl VeerPicConfigInterface for VeerPicConfig {
    const PIC_BASE_ADDRESS: usize = PIC_BASE;
    const MAX_IRQS: u32 = 256;
}

pub struct TimerConfig;

const TIMER_BASE: usize = 0x2100_0000;

impl ClintTimerConfigInterface for TimerConfig {
    const MTIME_REGISTER: usize = TIMER_BASE + 0xe4;
    const MTIMECMP_REGISTER: usize = TIMER_BASE + 0xec;
}

unsafe extern "C" {
    #[link_name = "_mtvec_table"]
    static MTVEC_TABLE: [u32; 32];
}
