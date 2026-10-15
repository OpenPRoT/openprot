// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 watchdog low-level register accessor.

use ast1060_pac as device;
use core::marker::PhantomData;

/// Safe wrapper around one AST10x0 watchdog register block.
///
/// The SoC exposes four independent watchdog instances (`WDT0`–`WDT3`) at a
/// `0x80` stride; each is reached through its own constructor.
pub struct WdtRegisters {
    base: *const device::wdt::RegisterBlock,
    /// Prevent `Send` and `Sync`.
    ///
    /// MMIO register blocks must not be transferred across threads or
    /// shared by reference due to potential side effects and lack of
    /// synchronization guarantees.
    _not_send_sync: PhantomData<*const ()>,
}

impl WdtRegisters {
    /// Create a register accessor from a raw watchdog register block pointer.
    ///
    /// # Safety
    ///
    /// - `base` must be a valid, non-null pointer to an AST1060 watchdog register block.
    /// - The block must remain valid for the lifetime of this value.
    /// - Caller must enforce exclusive (or otherwise coordinated) access to the
    ///   register block for the duration of use.
    pub const unsafe fn new(base: *const device::wdt::RegisterBlock) -> Self {
        Self {
            base,
            _not_send_sync: PhantomData,
        }
    }

    /// Create a register accessor for watchdog instance 0 (`0x7e78_5000`).
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the singleton `WDT0` peripheral is
    /// coordinated for the lifetime of this value.
    pub unsafe fn new_wdt0() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Wdt::ptr()) }
    }

    /// Create a register accessor for watchdog instance 1 (`0x7e78_5080`).
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the singleton `WDT1` peripheral is
    /// coordinated for the lifetime of this value.
    pub unsafe fn new_wdt1() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Wdt1::ptr()) }
    }

    /// Create a register accessor for watchdog instance 2 (`0x7e78_5100`).
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the singleton `WDT2` peripheral is
    /// coordinated for the lifetime of this value.
    pub unsafe fn new_wdt2() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Wdt2::ptr()) }
    }

    /// Create a register accessor for watchdog instance 3 (`0x7e78_5180`).
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the singleton `WDT3` peripheral is
    /// coordinated for the lifetime of this value.
    pub unsafe fn new_wdt3() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Wdt3::ptr()) }
    }

    #[inline]
    pub(crate) fn regs(&self) -> &device::wdt::RegisterBlock {
        // SAFETY: Constructor guarantees a valid, non-null register block pointer.
        unsafe { &*self.base }
    }
}
