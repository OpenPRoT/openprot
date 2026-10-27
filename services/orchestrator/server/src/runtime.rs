// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Kernel-clock binding for [`TimerManager`].
//!
//! [`BootWatchdogs`] instantiates the host-generic [`TimerManager`] with the
//! kernel's [`Instant`] and translates the run loop's relative boot/commit
//! windows into the absolute deadlines the manager tracks. The absolute
//! [`wait_deadline`](BootWatchdogs::wait_deadline) it returns is exactly the
//! argument the loop hands to `object_wait`; after each wake the loop drains
//! [`poll_expired`](BootWatchdogs::poll_expired) into orchestrator-sm.

use openprot_orchestrator_sm::{ComponentId, Event};
use openprot_orchestrator_timer::{Expired, Full, TimerManager};
use userspace::time::{Clock, Duration, Instant, SystemClock};

/// The orchestrator's watchdogs, driven by the kernel monotonic clock.
///
/// `N` bounds the boot watchdogs to the chain length, matching
/// [`TimerManager`].
pub struct BootWatchdogs<const N: usize> {
    timers: TimerManager<Instant, ComponentId, N>,
}

impl<const N: usize> BootWatchdogs<N> {
    pub const fn new() -> Self {
        Self {
            timers: TimerManager::new(),
        }
    }

    /// Now plus `after`, saturating to [`Instant::MAX`] on overflow so a huge
    /// window degrades to "wait indefinitely" rather than firing immediately.
    fn deadline_in(after: Duration) -> Instant {
        SystemClock::now()
            .checked_add_duration(after)
            .unwrap_or(Instant::MAX)
    }

    /// Arm (or re-arm) `id`'s boot watchdog to fire `after` from now. Returns
    /// [`Full`] when a new component would exceed `N`; the run loop must
    /// escalate rather than proceed with an unsupervised component.
    pub fn arm_boot(&mut self, id: ComponentId, after: Duration) -> Result<(), Full> {
        self.timers.arm_boot(id, Self::deadline_in(after))
    }

    /// Cancel `id`'s boot watchdog.
    pub fn cancel_boot(&mut self, id: ComponentId) {
        self.timers.cancel_boot(id);
    }

    /// Arm the commit watchdog to fire `after` from now.
    pub fn arm_commit(&mut self, after: Duration) {
        self.timers.arm_commit(Self::deadline_in(after));
    }

    /// Cancel the commit watchdog.
    pub fn cancel_commit(&mut self) {
        self.timers.cancel_commit();
    }

    /// Absolute deadline to pass to `object_wait`; [`Instant::MAX`] when nothing
    /// is armed, so the loop blocks until a signal wakes it.
    pub fn wait_deadline(&self) -> Instant {
        self.timers.next_deadline().unwrap_or(Instant::MAX)
    }

    /// Pop the next watchdog due as of now, or `None`. Call in a loop after each
    /// `object_wait` return to drain every deadline that has passed this tick.
    pub fn poll_expired(&mut self) -> Option<Event> {
        self.timers
            .poll(SystemClock::now())
            .map(|expired| match expired {
                Expired::Boot(id) => Event::Timeout(id),
                Expired::Commit => Event::CommitTimeout,
            })
    }
}

impl<const N: usize> Default for BootWatchdogs<N> {
    fn default() -> Self {
        Self::new()
    }
}
