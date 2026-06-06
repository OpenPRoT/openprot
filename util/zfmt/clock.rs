// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

/// Returns the current system clock time in ticks.
///
/// If `clock-earlgrey` feature is enabled, it queries the Earlgrey timer.
/// Otherwise, it returns 0.
#[cfg(feature = "clock-earlgrey")]
pub fn now_ticks() -> u64 {
    earlgrey_util::clock::now_ticks()
}

/// Returns the current system clock time in ticks.
///
/// If `clock-earlgrey` feature is enabled, it queries the Earlgrey timer.
/// Otherwise, it returns 0.
#[cfg(not(feature = "clock-earlgrey"))]
pub fn now_ticks() -> u64 {
    0
}
