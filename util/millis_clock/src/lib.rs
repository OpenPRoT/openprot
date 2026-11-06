// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Ties the kernel's tick clock to the portable `u64` millisecond clock that
//! the platform-independent servers use.
//!
//! A server keeps its deadlines in whole milliseconds since an arbitrary
//! origin, so it never touches a kernel type. The loop that runs it owns an
//! [`Epoch`]: it reads the elapsed milliseconds to hand to the server, and
//! turns the server's next deadline (or the lack of one) into the kernel
//! [`Instant`] a blocking wait (`object_wait`) takes.

#![no_std]

use userspace::time::{Clock, Duration, Instant, SystemClock};

/// A fixed origin tying the kernel's tick clock to a portable `u64`
/// millisecond clock.
#[derive(Clone, Copy)]
pub struct Epoch(Instant);

impl Epoch {
    /// An epoch whose zero is the moment of the call.
    pub fn start() -> Self {
        Self(SystemClock::now())
    }

    /// An epoch whose zero is the kernel clock's own zero, so
    /// [`millis`](Self::millis) is the time since the kernel's clock started.
    ///
    /// For code that has no loop of its own to hold an [`Epoch`] in, or that
    /// compares times across processes. Two such epochs always agree.
    pub const fn since_boot() -> Self {
        Self(Instant::from_ticks(0))
    }

    /// Whole milliseconds elapsed since the epoch.
    pub fn millis(&self) -> u64 {
        (SystemClock::now() - self.0).as_millis() as u64
    }

    /// The kernel instant to wake at for a server's next deadline.
    ///
    /// `None` means no deadline and gives `Instant::MAX`, which is what a
    /// blocking wait takes for "wait forever". For `Some(millis)`, the instant
    /// is one by which [`millis`](Self::millis) is guaranteed to have reached
    /// `millis`, or `Instant::MAX` if that is unrepresentable.
    ///
    /// One millisecond late on purpose. `millis` rounds down and the
    /// conversion to ticks may too, so waking exactly on time could read the
    /// clock as one short, find nothing due, and wait again on a deadline
    /// already in the past.
    pub fn instant_at(&self, deadline_millis: Option<u64>) -> Instant {
        let Some(millis) = deadline_millis else {
            return Instant::MAX;
        };
        self.0
            .checked_add_duration(Duration::from_millis(millis.saturating_add(1)))
            .unwrap_or(Instant::MAX)
    }
}
