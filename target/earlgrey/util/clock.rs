// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! System clock interface for the Earlgrey target.

use crate::timer::EarlGreyTimer;

// Safety: called only from the logging task after early boot; RvTimer is a
// read-only counter and no exclusive-access invariant is violated.
static TIMER: EarlGreyTimer = unsafe { EarlGreyTimer::new() };

/// Returns the current system clock time in ticks.
///
/// This uses the global read-only `RvTimer` instance.
#[inline(always)]
pub fn now_ticks() -> u64 {
    TIMER.read_ticks()
}
