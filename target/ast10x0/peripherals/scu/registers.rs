// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SCU low-level register access.

use ast1060_pac as device;
use core::cell::UnsafeCell;
use core::marker::PhantomData;

const SCU_UNLOCK_KEY: u32 = 0x1688_A8A8;

/// Safe wrapper around the AST10x0 SCU register block.
pub struct ScuRegisters {
    base: *const device::scu::RegisterBlock,
    _not_sync: PhantomData<UnsafeCell<()>>, // Prevent Sync, allow Send.
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
            _not_sync: PhantomData,
        }
    }

    /// Create a register accessor for the global SCU instance.
    ///
    /// # Safety
    /// Caller must ensure access to the singleton SCU is coordinated.
    const unsafe fn new_global() -> Self {
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
    fn unlock_write_protection(&self) {
        self.regs()
            .scu000()
            .write(|w| unsafe { w.bits(SCU_UNLOCK_KEY) });
    }
}