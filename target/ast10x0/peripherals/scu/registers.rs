// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SCU low-level register access.

use super::pins::ScuBlock;
use ast1060_pac as device;
use core::marker::PhantomData;

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
    pub(crate) const unsafe fn new_global() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Scu::ptr()) }
    }

    /// Create a register accessor for the global SCU instance, with write
    /// protection immediately unlocked.
    ///
    /// Follows the aspeed-rust pattern: unlock once, then perform all
    /// register writes in sequence without re-locking between operations.
    ///
    /// Unlike a pin driven through
    /// [`GpioPin::into_output`](crate::gpio::GpioPin::into_output)/
    /// [`into_input`](crate::gpio::GpioPin::into_input), which only compiles for a process the
    /// manifest's `pin_claims_<process>` list actually names, this has no equivalent gate: it is
    /// not bound to any manifest declaration, and not gated by a `Region` the way `GpioBlock::new`
    /// gates GPIO access. Closing that is tracked as follow-up work, not done here.
    ///
    /// # Safety
    /// Caller must ensure access to the singleton SCU is coordinated.
    pub unsafe fn new_global_unlocked() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        let scu = unsafe { Self::new_global() };
        scu.unlock_write_protection();
        scu
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
    pub(crate) fn unlock_write_protection(&self) {
        self.regs()
            .scu000()
            .write(|w| unsafe { w.bits(SCU_UNLOCK_KEY) });
    }
}
