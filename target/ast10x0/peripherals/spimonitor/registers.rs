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

    /// SPIPF07C: Lock/status register.
    pub fn read_lock_status(&self) -> u32 {
        self.regs().spipf07c().read().bits()
    }

    pub fn write_lock_status(&self, value: u32) {
        self.regs().spipf07c().write(|w| unsafe { w.bits(value) });
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

    /// SPIPFWA[n]: Address filter table entry.
    pub fn read_addr_filter_slot(&self, index: usize) -> u32 {
        self.regs().spipfwa(index).read().bits()
    }

    pub fn write_addr_filter_slot(&self, index: usize, value: u32) {
        self.regs()
            .spipfwa(index)
            .write(|w| unsafe { w.bits(value) });
    }

    // -----------------------------------------------------------------------
    // Violation log registers
    //
    // TODO: confirm SPIPF register offsets for log control from the AST10x0
    // datasheet once available. Offsets below are placeholders consistent with
    // known Aspeed SPIPF register map patterns.
    // -----------------------------------------------------------------------

    /// Current violation log write index (number of entries written so far).
    ///
    /// Maps to the SPIPF log index register (placeholder offset 0x080).
    pub fn read_log_idx_reg(&self) -> u32 {
        // SAFETY: raw offset read within the known SPIPF register block page.
        unsafe {
            let ptr = (self.base as *const u8).add(0x080) as *const u32;
            core::ptr::read_volatile(ptr)
        }
    }

    /// Maximum violation log capacity in bytes.
    ///
    /// Maps to the SPIPF log size register (placeholder offset 0x084).
    pub fn read_log_max_sz(&self) -> u32 {
        // SAFETY: same as above.
        unsafe {
            let ptr = (self.base as *const u8).add(0x084) as *const u32;
            core::ptr::read_volatile(ptr)
        }
    }

    /// Base address of the violation log RAM region.
    ///
    /// Returns a `usize` suitable for casting to `*const u32` by the caller.
    /// Maps to the SPIPF log RAM address register (placeholder offset 0x088).
    pub fn log_ram_base_addr(&self) -> usize {
        // SAFETY: same as above.
        unsafe {
            let ptr = (self.base as *const u8).add(0x088) as *const u32;
            core::ptr::read_volatile(ptr) as usize
        }
    }
}
