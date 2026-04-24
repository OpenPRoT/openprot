// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use core::hint::assert_unchecked;

/// Represents a `usize`` that is guaranteed to be a power-of-two. The compiler
/// can take advantage of this fact when optimizing (for example, using bitwise
/// arithmetic instead of division).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct PowerOf2Usize(usize);
impl PowerOf2Usize {
    // WARNING: Do not add any functions or derives (such as
    // zerocopy::FromBytes) that make it possible to modify this value without
    // confirming that it is still a power-of-two. As the compiler is relying on
    // the power-of-two assertion for safety, any such changes are unsound.

    #[inline(always)]
    pub const fn new(val: usize) -> Option<Self> {
        if !val.is_power_of_two() {
            return None;
        }
        Some(Self(val))
    }
    #[inline(always)]
    pub const fn get(self) -> usize {
        // SAFETY: These assertions are safe because self.0 can only be set by
        // Self::new, and we check for the same preconditions there.
        // (LLVM is too stupid to realize that is_power_of_two() implies != 0)
        unsafe { assert_unchecked(self.0 != 0) };
        unsafe { assert_unchecked(self.0.is_power_of_two()) };
        self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::PowerOf2Usize;

    #[test]
    pub fn test() {
        assert_eq!(PowerOf2Usize::new(0), None);
        assert_eq!(PowerOf2Usize::new(1).unwrap().get(), 1);
        assert_eq!(PowerOf2Usize::new(2).unwrap().get(), 2);
        assert_eq!(PowerOf2Usize::new(3), None);
        assert_eq!(PowerOf2Usize::new(4).unwrap().get(), 4);
        assert_eq!(PowerOf2Usize::new(5), None);
        assert_eq!(PowerOf2Usize::new(6), None);
        assert_eq!(PowerOf2Usize::new(7), None);
        assert_eq!(PowerOf2Usize::new(8).unwrap().get(), 8);
        assert_eq!(PowerOf2Usize::new(9), None);
        assert_eq!(PowerOf2Usize::new(0x7fff_ffff), None);
        assert_eq!(PowerOf2Usize::new(0x8000_0000).unwrap().get(), 0x8000_0000);
        assert_eq!(PowerOf2Usize::new(0x8000_0001), None);
        assert_eq!(PowerOf2Usize::new(0xc000_0000), None);
        assert_eq!(PowerOf2Usize::new(0xffff_ffff), None);

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(PowerOf2Usize::new(0x7fff_ffff_ffff_ffff), None);
            assert_eq!(
                PowerOf2Usize::new(0x8000_0000_0000_0000).unwrap().get(),
                0x8000_0000_0000_0000
            );
            assert_eq!(PowerOf2Usize::new(0x8000_0000_0000_0001), None);
            assert_eq!(PowerOf2Usize::new(0xc000_0000_0000_0000), None);
            assert_eq!(PowerOf2Usize::new(0xffff_ffff_ffff_ffff), None);
        }
    }
}
