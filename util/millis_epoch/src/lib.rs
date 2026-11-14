// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Converts between a kernel [`Instant`] and a `u64` millisecond count.
//!
//! [`Epoch::millis`] turns the time since the origin into whole milliseconds.
//! [`Epoch::instant_at`] turns a millisecond deadline back into the
//! [`Instant`] that `object_wait` waits on. A caller that already counts
//! milliseconds, such as MCTP, never sees a kernel type. The run loop owns
//! the [`Epoch`].
//!
//! These milliseconds are the kernel tick count divided by its configured
//! rate. They order deadlines. They are not wall time, and a wait is only as
//! fine as the scheduler tick.

#![no_std]

use userspace::time::{Clock, Duration, Instant, SystemClock};

/// Fixed origin for a caller that already counts milliseconds.
#[derive(Clone, Copy)]
pub struct Epoch(Instant);

impl Epoch {
    /// Zero is the moment of the call.
    pub fn start() -> Self {
        Self(SystemClock::now())
    }

    /// Zero is the kernel clock's zero, so [`millis`](Self::millis) is time
    /// since boot.
    ///
    /// For code with no loop to hold an [`Epoch`], or that compares times
    /// across processes. Two such epochs always agree.
    pub const fn since_boot() -> Self {
        Self(Instant::from_ticks(0))
    }

    /// Whole milliseconds since the epoch, rounded down.
    pub fn millis(&self) -> u64 {
        (SystemClock::now() - self.0).as_millis() as u64
    }

    /// [`Instant`] to pass to `object_wait` for the server's next deadline.
    ///
    /// `None` returns [`Instant::MAX`] (wait forever). `Some(millis)` returns
    /// an instant by which [`millis`](Self::millis) has reached `millis`, or
    /// [`Instant::MAX`] if that instant does not fit.
    ///
    /// One millisecond late on purpose. Both conversions round down, so an
    /// exact wake can read one short and wait again on a deadline already past.
    pub fn instant_at(&self, deadline_millis: Option<u64>) -> Instant {
        let Some(millis) = deadline_millis else {
            return Instant::MAX;
        };
        self.0
            .checked_add_duration(Duration::from_millis(millis.saturating_add(1)))
            .unwrap_or(Instant::MAX)
    }
}
