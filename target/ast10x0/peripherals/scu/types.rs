// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Public SCU types.

/// Selects the lower or upper 32-bit control register half for reset domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScuRegisterHalf {
    Lower,
    Upper,
}

/// Selects the lower or upper 32-bit control register half for clock domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockRegisterHalf {
    Lower,
    Upper,
}