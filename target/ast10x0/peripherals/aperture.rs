// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The whole address space, as a region token.

use util_region::{Mmap, Region};

/// Every address on the chip.
///
/// Kernel mode runs with no MPU applied, so a kernel-only binary's grant is the
/// whole space rather than a range carved out of `system.json5`. Drivers still
/// take a region by value, so ownership is tracked the same way either side of
/// the kernel boundary.
pub struct Aperture;

impl Mmap for Aperture {
    const START: usize = 0;
    const LEN: usize = usize::MAX;
}

/// # Safety
/// Mints ownership of the entire address space from nothing. Call once, and
/// only from a kernel-only binary, where no process holds a conflicting grant.
pub const unsafe fn take_aperture() -> Region<Aperture> {
    unsafe { Region::new() }
}
