// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct NanoClock;

impl pw_time::Clock for NanoClock {
    const TICKS_PER_SEC: u64 = 1_000_000_000;

    // NanoClock is only used as a compile-time tick configuration for Duration conversion.
    // now() is implemented with a dummy value only to satisfy the Clock trait requirement.
    fn now() -> pw_time::Instant<Self> {
        pw_time::Instant::from_ticks(0)
    }
}

pub type Nanoseconds = pw_time::Duration<NanoClock>;

/// A trait for multiplying duration types by an integer factor.
pub trait MultiplyDuration {
    /// Multiplies the duration by the specified integer factor.
    fn mul(self, factor: i64) -> Self;
}

impl MultiplyDuration for Nanoseconds {
    fn mul(self, factor: i64) -> Self {
        Self::from_nanos(self.ticks() * factor)
    }
}
