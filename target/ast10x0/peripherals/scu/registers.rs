// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SCU low-level register access.

use ast1060_pac as device;
use core::marker::PhantomData;

const SCU_UNLOCK_KEY: u32 = 0x1688_A8A8;

/// Safe wrapper around the AST10x0 SCU register block.
pub struct ScuRegisters {
    base: *const device::scu::RegisterBlock,
    /// Prevent `Send` and `Sync`.
    ///
    /// MMIO register blocks must not be transferred across threads or
    /// shared by reference due to potential side effects and lack of
    /// synchronization guarantees.
    _not_send_sync: PhantomData<*const ()>,
}

impl ScuRegisters {
    /// Create a register accessor from a raw SCU register block pointer.
    ///
    /// # Safety
    /// Caller must ensure:
    /// - `base` points to a valid SCU register block.
    /// - access to the SCU instance is serialized appropriately.
    const unsafe fn new(base: *const device::scu::RegisterBlock) -> Self {
        Self {
            base,
            _not_send_sync: PhantomData,
        }
    }

    /// Create a register accessor for the global SCU instance.
    ///
    /// # Safety
    /// Caller must ensure access to the singleton SCU is coordinated.
    pub const unsafe fn new_global() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Scu::ptr()) }
    }

    #[inline]
    pub fn regs(&self) -> &device::scu::RegisterBlock {
        // SAFETY: Constructor guarantees a valid SCU register block pointer.
        unsafe { &*self.base }
    }

    /// Unlock SCU write-protected registers for subsequent write operations.
    ///
    /// Call this once before a sequence of SCU writes, following the aspeed-rust
    /// pattern of a single unlock per batch of register operations.
    #[inline]
    fn unlock_write_protection(&self) {
        self.regs()
            .scu000()
            .write(|w| unsafe { w.bits(SCU_UNLOCK_KEY) });
    }

    /// Lock SCU write-protected registers after a sequence of write operations.
    #[inline]
    fn lock_write_protection(&self) {
        self.regs().scu000().write(|w| unsafe { w.bits(0) });
    }

    /// Lock SCU write-protected registers after a sequence of write operations.
    /// ## Example
    /// ```rust,no_run
    /// let scu = ScuRegisters::new_global();
    /// scu.unlock_and_lock_after(|scu| {
    ///   let mut bits = scu.regs().scu0f0().read().bits();
    ///   bits &= !0xF;
    ///   scu.regs().scu0f0().write(|w| unsafe { w.bits(bits) });
    /// });
    /// ```
    #[inline]
    pub fn unlock_and_lock_after<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ScuRegisters) -> R,
    {
        self.unlock_write_protection();
        let result = f(self);
        self.lock_write_protection();
        result
    }
}
