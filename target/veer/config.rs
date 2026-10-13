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

// MCU memory map for the selected target type, taken from upstream caliptra-mcu-sw
#[cfg(not(feature = "fpga"))]
use mcu_config_emulator::EMULATOR_MEMORY_MAP as MCU_MEMORY_MAP;
#[cfg(feature = "fpga")]
use mcu_config_fpga::FPGA_MEMORY_MAP as MCU_MEMORY_MAP;

/// Base of the MCI register block (MCU timer, device mappings).
pub const MCI_BASE: usize = MCU_MEMORY_MAP.mci_offset as usize;

const PIC_BASE: usize = MCU_MEMORY_MAP.pic_offset as usize;
const DCCM_BASE: usize = MCU_MEMORY_MAP.dccm_offset as usize;

pub struct KernelConfig;

impl KernelConfigInterface for KernelConfig {
    #[cfg(feature = "silicon")]
    const SYSTEM_CLOCK_HZ: u64 = 100_000_000;
    // Matches TIMER_FREQUENCY_HZ in
    // third_party/caliptra/caliptra-mcu-sw/platforms/fpga/runtime/src/main.rs,
    // the confirmed clock rate for this core on the VCK190 build.
    #[cfg(feature = "fpga")]
    const SYSTEM_CLOCK_HZ: u64 = 20_000_000;
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
    // The VeeR core requires the external-interrupt redirect table to live
    // in DCCM; this image does not otherwise use DCCM. 256 IRQs * 4 bytes
    // fits well within the 16KiB DCCM.
    const MEIVT_BASE_ADDRESS: usize = DCCM_BASE;
}

pub struct TimerConfig;

/// MCI `mcu_rv_mtime` register (the MCU timer lives in the MCI block).
pub const MTIME_REGISTER: usize = MCI_BASE + 0xe4;
/// MCI `mcu_rv_mtimecmp` register.
pub const MTIMECMP_REGISTER: usize = MCI_BASE + 0xec;

impl ClintTimerConfigInterface for TimerConfig {
    const MTIME_REGISTER: usize = MTIME_REGISTER;
    const MTIMECMP_REGISTER: usize = MTIMECMP_REGISTER;
}

unsafe extern "C" {
    #[link_name = "_mtvec_table"]
    static MTVEC_TABLE: [u32; 32];
}
