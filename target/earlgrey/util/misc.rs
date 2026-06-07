// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Miscellaneous utility types for Earlgrey.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unalign};

/// A wrapper around `zerocopy::Unalign<u64>` to provide safe, unaligned `u64` access.
///
/// This is used in packed hardware structures (like `BootLog` and `BootSvc`)
/// where 64-bit fields might not be aligned to 8-byte boundaries, to avoid
/// undefined behavior (unaligned read/write panics on some architectures).
#[derive(Clone, Copy, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct UnalignedU64(Unalign<u64>);

impl UnalignedU64 {
    /// Read the `u64` value.
    #[inline(always)]
    pub fn get(&self) -> u64 {
        self.0.get()
    }

    /// Write the `u64` value.
    #[inline(always)]
    pub fn set(&mut self, v: u64) {
        self.0.set(v)
    }
}

#[cfg(feature = "ufmt")]
const _: () = {
    impl ufmt::uDisplay for UnalignedU64 {
        fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
        where
            W: ufmt::uWrite + ?Sized,
        {
            let v = self.get();
            ufmt::uwrite!(f, "{:016x}", v)
        }
    }

    impl ufmt::uDebug for UnalignedU64 {
        fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
        where
            W: ufmt::uWrite + ?Sized,
        {
            ufmt::uDisplay::fmt(self, f)
        }
    }
};
