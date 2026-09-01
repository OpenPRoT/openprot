// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Watchdog bookkeeping for the orchestrator runtime.
//!
//! [`TimerManager`] multiplexes a set of logical deadlines — up to `N`
//! per-component boot-progress watchdogs plus one activated-but-not-committed
//! commit watchdog — onto the single deadline the runtime passes to
//! `syscall::object_wait`. It owns no clock and no policy: the runtime supplies
//! absolute deadlines (it knows the clock and the per-component boot windows),
//! and this type only tracks which deadline is nearest and reports which
//! watchdog expired as an [`Expired`].
//!
//! Cancellation is best-effort by design. The consumer already drops a stale
//! boot timeout for a component that is no longer awaiting boot, and a commit
//! timeout with no update in flight, so a late fire that races a cancel is
//! harmless — it becomes a dropped event, never a false recovery.
//!
//! Both the instant type `T` and the component id `Id` are generic, so this is a
//! leaf crate with no orchestrator dependencies and stays host-testable; the
//! runtime instantiates them with `userspace::time::Instant` and
//! orchestrator-sm's `ComponentId`, and maps [`Expired`] onto its event type.

#![no_std]
#![forbid(unsafe_code)]

/// The boot-watchdog buffer is full: more than `N` components are awaiting boot
/// at once, which means `N` was sized below the chain length. Returned by
/// [`TimerManager::arm_boot`] so the caller can escalate rather than lose a
/// boot-progress watchdog silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

/// Which watchdog came due, returned by [`TimerManager::poll`]. The runtime
/// maps it onto orchestrator-sm's event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expired<Id> {
    /// `Id`'s boot-progress watchdog fired.
    Boot(Id),
    /// The commit watchdog fired.
    Commit,
}

/// One armed boot-progress watchdog: which component, and when it is due.
struct BootDeadline<T, Id> {
    id: Id,
    at: T,
}

/// Multiplexes the orchestrator's boot and commit watchdogs onto one deadline.
///
/// `N` bounds the boot watchdogs to the chain length: a speculative `Passive`
/// walk can leave every released component awaiting its boot-progress signal at
/// once, so at most `N` are armed together. The commit watchdog is a single
/// slot.
pub struct TimerManager<T, Id, const N: usize> {
    boot: heapless::Vec<BootDeadline<T, Id>, N>,
    commit: Option<T>,
}

impl<T: Copy + Ord, Id: Copy + Eq, const N: usize> TimerManager<T, Id, N> {
    pub const fn new() -> Self {
        Self {
            boot: heapless::Vec::new(),
            commit: None,
        }
    }

    /// Arm (or re-arm) `id`'s boot watchdog to fire at `deadline`. Re-arming an
    /// already-armed component replaces its deadline rather than stacking a
    /// second entry, so a recovery re-release just restarts the window.
    ///
    /// Returns [`Full`] only when a new component would exceed `N`. A boot
    /// watchdog is the sole source of a component's boot timeout, so the caller
    /// must escalate rather than drop the arm: a lost arm means a stuck
    /// component is never supervised.
    pub fn arm_boot(&mut self, id: Id, deadline: T) -> Result<(), Full> {
        match self.boot.iter_mut().find(|d| d.id == id) {
            Some(d) => {
                d.at = deadline;
                Ok(())
            }
            None => self
                .boot
                .push(BootDeadline { id, at: deadline })
                .map(|_| ())
                .map_err(|_| Full),
        }
    }

    /// Cancel `id`'s boot watchdog. No-op if it is not armed — cancellation
    /// need not be race-free, since a stale boot timeout is dropped by the
    /// consumer anyway.
    pub fn cancel_boot(&mut self, id: Id) {
        if let Some(i) = self.boot.iter().position(|d| d.id == id) {
            self.boot.swap_remove(i);
        }
    }

    /// Arm the single commit watchdog to fire at `deadline`.
    pub fn arm_commit(&mut self, deadline: T) {
        self.commit = Some(deadline);
    }

    /// Cancel the commit watchdog. No-op if it is not armed.
    pub fn cancel_commit(&mut self) {
        self.commit = None;
    }

    /// The nearest outstanding deadline, or `None` when nothing is armed. Feed
    /// this to the runtime's `object_wait` as its deadline.
    pub fn next_deadline(&self) -> Option<T> {
        let boot = self.boot.iter().map(|d| d.at).min();
        match (boot, self.commit) {
            (Some(b), Some(c)) => Some(b.min(c)),
            (b, c) => b.or(c),
        }
    }

    /// Pop the single nearest watchdog that is due at `now`, remove it, and
    /// report which one fired. Returns `None` when nothing is due. Call in a
    /// loop until `None` to drain every deadline that has passed this tick; a
    /// fired watchdog is one-shot and gone until re-armed.
    ///
    /// When a boot and the commit watchdog are due together the boot one is
    /// reported first, matching `next_deadline`'s tie-break so a caller draining
    /// in a loop sees a consistent order.
    pub fn poll(&mut self, now: T) -> Option<Expired<Id>> {
        let earliest_boot = self
            .boot
            .iter()
            .enumerate()
            .filter(|(_, d)| d.at <= now)
            .min_by(|(_, a), (_, b)| a.at.cmp(&b.at))
            .map(|(i, d)| (i, d.at, d.id));
        let commit_due = self.commit.filter(|&c| c <= now);

        match (earliest_boot, commit_due) {
            (Some((i, b_at, id)), Some(c_at)) if b_at <= c_at => {
                self.boot.swap_remove(i);
                Some(Expired::Boot(id))
            }
            (_, Some(_)) => {
                self.commit = None;
                Some(Expired::Commit)
            }
            (Some((i, _, id)), None) => {
                self.boot.swap_remove(i);
                Some(Expired::Boot(id))
            }
            (None, None) => None,
        }
    }
}

impl<T: Copy + Ord, Id: Copy + Eq, const N: usize> Default for TimerManager<T, Id, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C0: u8 = 0;
    const C1: u8 = 1;

    type Tm = TimerManager<u64, u8, 4>;

    #[test]
    fn empty_has_no_deadline_and_polls_none() {
        let mut tm = Tm::new();
        assert_eq!(tm.next_deadline(), None);
        assert_eq!(tm.poll(100), None);
    }

    #[test]
    fn next_deadline_is_the_nearest() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 50).unwrap();
        tm.arm_boot(C1, 30).unwrap();
        tm.arm_commit(40);
        assert_eq!(tm.next_deadline(), Some(30));
    }

    #[test]
    fn boot_expiry_yields_timeout_once() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 10).unwrap();
        assert_eq!(tm.poll(10), Some(Expired::Boot(C0)));
        // one-shot: gone until re-armed
        assert_eq!(tm.poll(10), None);
        assert_eq!(tm.next_deadline(), None);
    }

    #[test]
    fn not_yet_due_is_not_reported() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 100).unwrap();
        assert_eq!(tm.poll(99), None);
        assert_eq!(tm.poll(100), Some(Expired::Boot(C0)));
    }

    #[test]
    fn cancel_boot_disarms() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 10).unwrap();
        tm.cancel_boot(C0);
        assert_eq!(tm.poll(10), None);
    }

    #[test]
    fn rearm_replaces_deadline_not_stacks() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 10).unwrap();
        tm.arm_boot(C0, 50).unwrap();
        assert_eq!(tm.poll(10), None); // old deadline is gone
        assert_eq!(tm.poll(50), Some(Expired::Boot(C0)));
        assert_eq!(tm.poll(50), None); // only one entry ever existed
    }

    #[test]
    fn commit_expiry_yields_commit_timeout_once() {
        let mut tm = Tm::new();
        tm.arm_commit(20);
        assert_eq!(tm.poll(20), Some(Expired::Commit));
        assert_eq!(tm.poll(20), None);
    }

    #[test]
    fn cancel_commit_disarms() {
        let mut tm = Tm::new();
        tm.arm_commit(20);
        tm.cancel_commit();
        assert_eq!(tm.poll(20), None);
    }

    #[test]
    fn drains_all_due_in_a_loop() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 10).unwrap();
        tm.arm_boot(C1, 15).unwrap();
        tm.arm_commit(20);

        let mut drained = heapless::Vec::<Expired<u8>, 4>::new();
        while let Some(ev) = tm.poll(20) {
            drained.push(ev).unwrap();
        }
        assert_eq!(
            drained.as_slice(),
            &[Expired::Boot(C0), Expired::Boot(C1), Expired::Commit]
        );
    }

    #[test]
    fn boot_reported_before_commit_on_a_tie() {
        let mut tm = Tm::new();
        tm.arm_boot(C0, 20).unwrap();
        tm.arm_commit(20);
        assert_eq!(tm.poll(20), Some(Expired::Boot(C0)));
        assert_eq!(tm.poll(20), Some(Expired::Commit));
    }

    #[test]
    fn rearm_of_existing_component_never_overflows() {
        let mut tm = TimerManager::<u64, u8, 2>::new();
        tm.arm_boot(C0, 10).unwrap();
        tm.arm_boot(C1, 10).unwrap();
        // Buffer is full, but re-arming an already-armed id replaces in place.
        assert_eq!(tm.arm_boot(C0, 20), Ok(()));
    }

    #[test]
    fn full_buffer_reports_full() {
        let mut tm = TimerManager::<u64, u8, 2>::new();
        tm.arm_boot(C0, 10).unwrap();
        tm.arm_boot(C1, 10).unwrap();
        assert_eq!(tm.arm_boot(2, 10), Err(Full));
    }
}
