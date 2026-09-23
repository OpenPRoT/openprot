// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Ownership tokens for the memory regions a process is granted in `system.json5`.
//!
//! The per-process mapping table is generated from the same manifest the kernel
//! uses to program the MPU, so an address is written down in exactly one place.

#![no_std]

use core::marker::PhantomData;

/// Address range of one memory mapping, generated from the system manifest.
pub trait Mmap {
    const START: usize;
    const LEN: usize;
}

/// Exclusive ownership of the region described by `T`.
///
/// Move-only, so handing it to a driver transfers sole access and a second
/// claim is a compile error rather than an aliasing hazard.
pub struct Region<T: Mmap>(PhantomData<T>);

impl<T: Mmap> Region<T> {
    /// # Safety
    /// Mints ownership of `T`'s address range from nothing. Only the generated
    /// per-process mapping table may call this, and only once.
    #[doc(hidden)]
    pub const unsafe fn new() -> Self {
        Self(PhantomData)
    }

    /// Start of the granted range, as a mutable pointer.
    pub const fn as_mut_ptr(&self) -> *mut u8 {
        T::START as *mut u8
    }
}
