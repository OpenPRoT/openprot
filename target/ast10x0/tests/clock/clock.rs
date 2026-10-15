// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The system clock, in the units the test apps actually use.
//!
//! Every scenario app measures something against [`SystemClock`]: boot
//! windows, stall budgets, IPC deadlines, the FD_T1 and FD_T2 timers.
//! `SystemClock` counts ticks and all of them want milliseconds, so each
//! app had written the same conversion, three of them inline and two
//! behind a local helper of the same name.

#![no_std]

use userspace::time::{Clock, Instant, SystemClock};

/// Milliseconds for a tick count.
pub fn ticks_to_millis(ticks: u64) -> u64 {
    ticks * 1000 / SystemClock::TICKS_PER_SEC
}

/// Ticks for a duration in milliseconds.
pub fn millis_to_ticks(millis: u64) -> u64 {
    millis * SystemClock::TICKS_PER_SEC / 1000
}

/// Now, in milliseconds. What the orchestrator's pumps and the firmware
/// device's timers are both measured against.
pub fn now_millis() -> u64 {
    ticks_to_millis(SystemClock::now().ticks())
}

/// An [`Instant`] `millis` from now, for a syscall that takes a deadline.
pub fn deadline_in(millis: u64) -> Instant {
    Instant::from_ticks(SystemClock::now().ticks() + millis_to_ticks(millis))
}
