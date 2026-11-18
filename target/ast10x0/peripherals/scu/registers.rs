// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SCU low-level register access.

use super::pins::ScuBlock;
use ast1060_pac as device;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, Ordering};

const SCU_UNLOCK_KEY: u32 = 0x1688_A8A8;

/// Write-enable the SCU's protected registers, through the confined `Mmio` accessor. The key stays
/// private here; the pin-mux route path calls this before applying a pin's coalesced mux writes.
pub(crate) fn unlock_scu_writes(mmio: &crate::Mmio<ScuBlock>) {
    mmio.write_reg_at::<0x000>(SCU_UNLOCK_KEY);
}

/// Write-protect the SCU's protected registers again: any value other than [`SCU_UNLOCK_KEY`]
/// re-asserts protection, per the standard Aspeed SCU protection-key convention (same key across
/// their SCU family). The pin-mux route path calls this once it's done applying a boot-time mux —
/// nothing after boot has a legitimate reason to need the SCU open for writes.
pub(crate) fn lock_scu_writes(mmio: &crate::Mmio<ScuBlock>) {
    mmio.write_reg_at::<0x000>(0);
}

/// Nesting depth of [`ScuRegisters::unlock_and_lock_after`]; the SCU is
/// unlocked while this is non-zero.
static UNLOCK_DEPTH: AtomicU32 = AtomicU32::new(0);

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
    #[inline]
    fn unlock_write_protection(&self) {
        self.regs()
            .scu000()
            .write(|w| unsafe { w.bits(SCU_UNLOCK_KEY) });
    }

    /// Lock SCU write-protected registers.
    #[inline]
    fn lock_write_protection(&self) {
        self.regs().scu000().write(|w| unsafe { w.bits(0) });
    }

    /// Run `f` with the SCU write-protected registers unlocked, then re-lock.
    ///
    /// Calls may be nested: only the outermost call unlocks and only the
    /// outermost call re-locks, so a setter that takes the lock itself can be
    /// invoked from inside another `unlock_and_lock_after` closure without
    /// re-locking the SCU under the caller.
    ///
    /// The SCU is not re-locked if `f` panics.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ast10x0_peripherals::scu::ScuRegisters;
    /// // SAFETY: example only; the caller owns the singleton SCU.
    /// let scu = unsafe { ScuRegisters::new_global() };
    /// scu.unlock_and_lock_after(|scu| {
    ///     let mut bits = scu.regs().scu0f0().read().bits();
    ///     bits &= !0xF;
    ///     scu.regs().scu0f0().write(|w| unsafe { w.bits(bits) });
    /// });
    /// ```
    #[inline]
    pub fn unlock_and_lock_after<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ScuRegisters) -> R,
    {
        if UNLOCK_DEPTH.fetch_add(1, Ordering::AcqRel) == 0 {
            self.unlock_write_protection();
        }
        let result = f(self);
        if UNLOCK_DEPTH.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.lock_write_protection();
        }
        result
    }
}
