// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SPI monitor (SPIPF) low-level register access.
//!

use ast1060_pac as device;
use core::cell::UnsafeCell;
use core::marker::PhantomData;

/// SPI monitor controller identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiMonitorController {
    Spim0 = 0,
    Spim1 = 1,
    Spim2 = 2,
    Spim3 = 3,
}

impl SpiMonitorController {
    /// Returns the SPIPF register block base address for the controller.
    #[must_use]
    pub const fn base_address(self) -> usize {
        match self {
            Self::Spim0 => SPIPF1_BASE,
            Self::Spim1 => SPIPF2_BASE,
            Self::Spim2 => SPIPF3_BASE,
            Self::Spim3 => SPIPF4_BASE,
        }
    }

    /// Returns the PAC register block pointer for this controller.
    #[must_use]
    pub const fn ptr(self) -> *const device::spipf::RegisterBlock {
        match self {
            Self::Spim0 => device::Spipf::ptr(),
            Self::Spim1 => device::Spipf1::ptr(),
            Self::Spim2 => device::Spipf2::ptr(),
            Self::Spim3 => device::Spipf3::ptr(),
        }
    }
}

/// SPIPF1 base address (used by SPIM0).
pub const SPIPF1_BASE: usize = 0x7E79_1000;
/// SPIPF2 base address (used by SPIM1).
pub const SPIPF2_BASE: usize = 0x7E79_2000;
/// SPIPF3 base address (used by SPIM2).
pub const SPIPF3_BASE: usize = 0x7E79_3000;
/// SPIPF4 base address (used by SPIM3).
pub const SPIPF4_BASE: usize = 0x7E79_4000;

/// Size of each SPIPF register block.
pub const SPIPF_REG_SIZE: usize = 0x1000;

/// Safe wrapper around a SPI monitor register block.
pub struct SpiMonitorRegisters {
    base: *const device::spipf::RegisterBlock,
    _not_sync: PhantomData<UnsafeCell<()>>, // Prevent Sync, allow Send.
}

impl SpiMonitorRegisters {
    /// Create a register accessor from a raw register block pointer.
    ///
    /// # Safety
    /// Caller must ensure:
    /// - `base` points to a valid SPIPF register block.
    /// - Only one mutable owner accesses this hardware instance.
    pub const unsafe fn new(base: *const device::spipf::RegisterBlock) -> Self {
        Self {
            base,
            _not_sync: PhantomData,
        }
    }

    /// Create a register accessor for a specific SPI monitor controller.
    ///
    /// # Safety
    /// Caller must ensure exclusive ownership of the selected controller.
    pub const unsafe fn new_for_controller(controller: SpiMonitorController) -> Self {
        // SAFETY: Caller upholds exclusive-access requirement.
        unsafe { Self::new(controller.ptr()) }
    }

    #[inline]
    fn regs(&self) -> &device::spipf::RegisterBlock {
        // SAFETY: Constructor ensures pointer validity and ownership discipline.
        unsafe { &*self.base }
    }

    /// SPIPF000: Common control register.
    pub fn read_ctrl(&self) -> u32 {
        self.regs().spipf000().read().bits()
    }

    pub fn write_ctrl(&self, value: u32) {
        self.regs().spipf000().write(|w| unsafe { w.bits(value) });
    }

    pub fn modify_ctrl<F>(&self, f: F)
    where
        F: FnOnce(&mut u32),
    {
        self.regs().spipf000().modify(|r, w| {
            let mut bits = r.bits();
            f(&mut bits);
            // SAFETY: Callback is responsible for valid register image.
            unsafe { w.bits(bits) }
        });
    }

    /// SPIPF004: Secondary control/config register.
    pub fn read_ctrl2(&self) -> u32 {
        self.regs().spipf004().read().bits()
    }

    pub fn write_ctrl2(&self, value: u32) {
        self.regs().spipf004().write(|w| unsafe { w.bits(value) });
    }

    pub fn modify_ctrl2<F>(&self, f: F)
    where
        F: FnOnce(&mut u32),
    {
        self.regs().spipf004().modify(|r, w| {
            let mut bits = r.bits();
            f(&mut bits);
            unsafe { w.bits(bits) }
        });
    }

    /// SPIPF07C: Lock/status register.
    pub fn read_lock_status(&self) -> u32 {
        self.regs().spipf07c().read().bits()
    }

    pub fn write_lock_status(&self, value: u32) {
        self.regs().spipf07c().write(|w| unsafe { w.bits(value) });
    }

    pub fn modify_lock_status<F>(&self, f: F)
    where
        F: FnOnce(&mut u32),
    {
        self.regs().spipf07c().modify(|r, w| {
            let mut bits = r.bits();
            f(&mut bits);
            unsafe { w.bits(bits) }
        });
    }

    /// SPIPFWT[n]: Allow-command table entry.
    pub fn read_allow_cmd_slot(&self, index: usize) -> u32 {
        self.regs().spipfwt(index).read().bits()
    }

    pub fn write_allow_cmd_slot(&self, index: usize, value: u32) {
        self.regs()
            .spipfwt(index)
            .write(|w| unsafe { w.bits(value) });
    }

    /// Read an allow-command slot without panicking for an invalid index.
    pub fn read_allow_cmd_slot_checked(&self, index: usize) -> Option<u32> {
        self.regs()
            .spipfwt_iter()
            .nth(index)
            .map(|register| register.read().bits())
    }

    /// SPIPFWA[n]: Address filter table entry.
    pub fn read_addr_filter_slot(&self, index: usize) -> u32 {
        self.regs().spipfwa(index).read().bits()
    }

    pub fn write_addr_filter_slot(&self, index: usize, value: u32) {
        self.regs()
            .spipfwa(index)
            .write(|w| unsafe { w.bits(value) });
    }

    /// Read an address-filter slot without panicking for an invalid index.
    pub fn read_addr_filter_slot_checked(&self, index: usize) -> Option<u32> {
        self.regs()
            .spipfwa_iter()
            .nth(index)
            .map(|register| register.read().bits())
    }

    /// Write an address-filter slot, returning false for an invalid index.
    #[must_use]
    pub fn write_addr_filter_slot_checked(&self, index: usize, value: u32) -> bool {
        let Some(register) = self.regs().spipfwa_iter().nth(index) else {
            return false;
        };
        register.write(|w| unsafe { w.bits(value) });
        true
    }

    // -----------------------------------------------------------------------
    // Violation log registers
    // -----------------------------------------------------------------------

    /// Current violation log write index (number of 32-bit entries written).
    pub fn read_log_idx_reg(&self) -> u32 {
        self.regs()
            .spipf018()
            .read()
            .block_log_dmawr_pointer()
            .bits()
    }

    /// Maximum violation log capacity in 32-bit entries.
    pub fn read_log_capacity_entries(&self) -> u32 {
        self.regs()
            .spipf014()
            .read()
            .size_of_block_log_dmabuffer()
            .bits()
    }

    /// Base address of the violation log RAM region.
    pub fn log_ram_base_addr(&self) -> usize {
        (self.regs().spipf010().read().bits() & !0x3) as usize
    }

    /// Configure the violation-log DMA buffer.
    pub fn write_log_config(&self, base_addr: u32, entries: u32) {
        self.regs()
            .spipf010()
            .write(|w| unsafe { w.bits(base_addr & !0x3) });
        self.regs()
            .spipf014()
            .write(|w| unsafe { w.bits(LOG_DMA_ENABLE | entries) });
    }
}

const LOG_DMA_ENABLE: u32 = 1 << 31;
