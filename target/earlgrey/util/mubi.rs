// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Multi-bit boolean (MuBi) utilities for OpenTitan Earlgrey.
//!
//! Multi-bit booleans are used in security-critical hardware designs to prevent
//! fault-injection attacks. A single-bit flip should not be able to change a
//! boolean state (e.g., from True to False).
//!
//! This module provides traits to convert standard Rust types to their
//! multi-bit representations used by the hardware.

/// Convert values to Earlgrey 4-bit multi-bit booleans (MuBi4).
///
/// In OpenTitan:
/// - `Mubi4True` is represented as `0x6` (binary `0110`)
/// - `Mubi4False` is represented as `0x9` (binary `1001`)
///
/// Other values are treated as invalid/false by the hardware.
pub trait AsMubi {
    /// Returns the 4-bit MuBi representation of `self` as a `u32`.
    fn as_mubi(&self) -> u32;
}

impl AsMubi for bool {
    /// Converts `bool` to 4-bit MuBi value.
    ///
    /// Returns `0x6` for `true` and `0x9` for `false`.
    #[inline(always)]
    fn as_mubi(&self) -> u32 {
        if *self {
            6
        } else {
            9
        }
    }
}
