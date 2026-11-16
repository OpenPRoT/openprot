// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Ownership tokens for the memory regions a process is granted in `system.json5`.
//!
//! The per-process mapping table is generated from the same manifest the kernel
//! uses to program the MPU, so an address is written down in exactly one place.

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;

/// Address range of one memory mapping, generated from the system manifest.
pub trait Mmap {
    const START: usize;
    const LEN: usize;
}

/// Whether `T`'s range contains the `len` bytes starting at `start`.
///
/// Written without `START + LEN` so a region that runs to the end of the
/// address space does not overflow the const evaluator.
pub const fn covers<T: Mmap>(start: usize, len: usize) -> bool {
    start >= T::START && T::LEN >= len && start - T::START <= T::LEN - len
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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMap;
    impl Mmap for TestMap {
        const START: usize = 0x1000;
        const LEN: usize = 0x100;
    }

    #[test]
    fn covers_the_exact_range() {
        assert!(covers::<TestMap>(0x1000, 0x100));
    }

    #[test]
    fn covers_a_sub_range() {
        assert!(covers::<TestMap>(0x1010, 0x10));
    }

    #[test]
    fn rejects_a_start_before_the_mapping() {
        assert!(!covers::<TestMap>(0x0ff0, 0x10));
    }

    #[test]
    fn rejects_a_length_longer_than_the_mapping() {
        assert!(!covers::<TestMap>(0x1000, 0x101));
    }

    #[test]
    fn rejects_a_range_that_runs_past_the_mapping_end() {
        assert!(!covers::<TestMap>(0x10f0, 0x20));
    }

    #[test]
    fn zero_length_at_start_is_covered_even_by_a_zero_length_mapping() {
        struct Empty;
        impl Mmap for Empty {
            const START: usize = 0x2000;
            const LEN: usize = 0;
        }
        assert!(covers::<Empty>(0x2000, 0));
        assert!(!covers::<Empty>(0x2000, 1));
    }
}
