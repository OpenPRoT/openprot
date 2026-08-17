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
    mmio.write_reg(0x000, SCU_UNLOCK_KEY);
}

/// Safe accessor for the AST10x0 SCU register block — the shared clock/reset/pin-mux authority.
///
/// The SCU is a stateless, re-entrant control block: this handle is just a fixed MMIO pointer,
/// and every operation is a register read/write via a safe `&self` method. Its meaningful
/// invariant is *provenance* (past the boot gate), not exclusive ownership — so it is `Copy`.
/// Mint it once through the sole `unsafe` constructor ([`ScuRegisters::new_global_unlocked`]) at
/// boot and copy that handle into every capability that needs SCU access; the copy is the shared
/// authority. Holding one is proof the boot gate ran, which is what lets the register methods be
/// safe.
#[derive(Clone, Copy)]
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

/// Offset-addressed access for the coalesced mux applier — one RMW per touched SCU register via the `ScuBlock` singleton applier.
impl openprot_hal::field_mux::RegBlock for ScuRegisters {
    fn read_reg(&self, offset: u32) -> u32 {
        crate::Mmio::<ScuBlock>::block().read_reg(offset)
    }

    fn write_reg(&self, offset: u32, val: u32) {
        crate::Mmio::<ScuBlock>::block().write_reg(offset, val)
    }
}
