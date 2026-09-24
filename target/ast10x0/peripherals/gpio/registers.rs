// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO low-level register accessor.

use ast1060_pac as device;
use core::marker::PhantomData;
use util_region::{covers, Mmap, Region};

/// Base address of the AST10x0 GPIO register block.
const GPIO_BASE: usize = 0x7e78_0000;

/// Proof that this process was granted the GPIO register block.
///
/// Carries no pointer. The address comes from `R`, which exists only in the
/// per-process mapping table generated from `system.json5`, so a process that
/// was not granted the block has no mapping to name and the driver will not
/// compile for it.
pub struct GpioRegisters<R: Mmap> {
    _region: PhantomData<R>,
    /// Prevent `Send` and `Sync`.
    ///
    /// MMIO register blocks must not be transferred across threads or
    /// shared by reference due to potential side effects and lack of
    /// synchronization guarantees.
    _not_send_sync: PhantomData<*const ()>,
}

impl<R: Mmap> GpioRegisters<R> {
    /// Take ownership of the granted GPIO register block.
    ///
    /// Safe because the region is minted once per process from the same
    /// manifest the kernel uses to program the MPU, and taking it by value
    /// spends it.
    pub fn new(_region: Region<R>) -> Self {
        const {
            assert!(
                covers::<R>(
                    GPIO_BASE,
                    core::mem::size_of::<device::gpio::RegisterBlock>()
                ),
                "mapped region does not contain the GPIO register block"
            );
        }
        Self {
            _region: PhantomData,
            _not_send_sync: PhantomData,
        }
    }
}

/// The granted GPIO register block.
///
/// The one place in the driver that turns an address into a reference.
#[inline]
pub(crate) fn regs_of<R: Mmap>() -> &'static device::gpio::RegisterBlock {
    // SAFETY: `R` was checked to contain this block when the region was taken.
    unsafe { &*(GPIO_BASE as *const device::gpio::RegisterBlock) }
}
