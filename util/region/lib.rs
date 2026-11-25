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

/// Proof the implementing type is authorized to mint region `T` — i.e. the only thing
/// [`Region::new`] will accept as evidence. Generated code is the only intended source of impls:
/// each process's generated mapping table (`regions.rs.jinja`) emits one zero-sized `Authority`
/// type with an impl per mapping that process's own manifest entry declares. A mapping outside
/// that process's own generated crate has no matching impl reachable from it, so minting it fails
/// to compile — not a runtime check to bypass, code that was never generated to call.
pub trait AuthorizesRegion<T> {}

/// Exclusive ownership of the region described by `T`.
///
/// Move-only, so handing it to a driver transfers sole access and a second
/// claim is a compile error rather than an aliasing hazard.
pub struct Region<T: Mmap>(PhantomData<T>);

impl<T: Mmap> Region<T> {
    /// Mint exclusive ownership of `T`'s address range.
    ///
    /// Requires `auth: A` proving the caller is [`AuthorizesRegion`] for this exact mapping —
    /// generated code is the only intended source of such an `A`, scoped to exactly the mappings
    /// the manifest grants it.
    ///
    /// # Safety
    /// Caller must ensure this exact region is minted at most once within whichever protection
    /// domain calls it.
    #[doc(hidden)]
    pub const unsafe fn new<A: AuthorizesRegion<T>>(_auth: &A) -> Self {
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
