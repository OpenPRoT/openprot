// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO low-level register accessor.

use ast1060_pac as device;
use core::marker::PhantomData;

/// Safe wrapper around the AST1060 GPIO register block.
pub struct GpioRegisters {
    base: *const device::gpio::RegisterBlock,
    /// Prevent `Send` and `Sync`.
    ///
    /// MMIO register blocks must not be transferred across threads or
    /// shared by reference due to potential side effects and lack of
    /// synchronization guarantees.
    _not_send_sync: PhantomData<*const ()>,
}

impl GpioRegisters {
    /// Create a register accessor from a raw GPIO register block pointer.
    ///
    /// # Safety
    ///
    /// - `base` must be a valid, non-null pointer to the AST1060 GPIO register block.
    /// - The block must remain valid for the lifetime of this value.
    /// - Caller must enforce exclusive (or otherwise coordinated) access to the
    ///   register block for the duration of use.
    pub const unsafe fn new(base: *const device::gpio::RegisterBlock) -> Self {
        Self {
            base,
            _not_send_sync: PhantomData,
        }
    }

    /// Create a register accessor for the global GPIO instance.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the singleton GPIO peripheral is
    /// coordinated for the lifetime of this value.
    pub unsafe fn new_global() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Gpio::ptr()) }
    }

    #[inline]
    pub(crate) fn regs(&self) -> &device::gpio::RegisterBlock {
        // SAFETY: Constructor guarantees a valid, non-null register block pointer.
        unsafe { &*self.base }
    }
}
