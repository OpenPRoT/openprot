// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`SelfUpdate`] persistent session contract for the eRoT's own image.
//!
//! An update that activates a new image and self-resets has to survive the
//! reboot: the running code is gone, and with it every RAM fact about what was
//! staged and why. This trait is what the next boot reads. It holds one
//! session at a time, because the eRoT updates itself one image at a time.
//!
//! Downstream devices need no persistent session. A reset loses the boot
//! observation that would decide the trial, so the only safe action for a
//! downstream device after a reset is abandon, and
//! [`Updatable::abandon`](crate::Updatable::abandon) is infallible and
//! unconditional: calling it on every device at boot gets the same result with
//! no storage. This trait exists for the eRoT's own image only, one per
//! platform.
//!
//! The session carries the state, not only the verified SVN, because the state
//! is what tells two crashes apart. A session that knew only the SVN would read
//! the same after a crash before the trial was armed and after a confirmed
//! trial whose floor advance had not run yet, and those need opposite actions:
//! the first has to be dropped, the second has to advance the anti-rollback
//! floor. Advancing it on the first would push the floor past the image that is
//! running.
//!
//! [`trial_outcome`] reads the session and the image the eRoT booted and says
//! what happened to the trial. What to do about it is the state machine's, the
//! same split [`BootWatch`](crate::BootWatch) and [`Recovery`](crate::Recovery)
//! already use: the seam answers with a verdict, the machine decides.

use crate::Svn;

/// Which image the eRoT is running, as durable storage records it.
///
/// This, not whether an arming is still held, is what tells a trial boot from
/// a fallback. Platforms differ on the arming: a boot selector that consumes
/// the arming leaves nothing held by the time the trial image runs, while one
/// that holds it until the trial is decided still does. Which image booted
/// reads the same on both.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunningImage {
    /// The image a trial armed. This boot is the trial.
    Trial,
    /// The image the last confirmed update left in place.
    Confirmed,
}

/// Where the eRoT's own update session stands, as held in durable storage.
///
/// `Copy` and lifetime-free, like every capability vocabulary type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelfUpdateState {
    /// No update in flight.
    Idle,
    /// An image was authenticated with this SVN and written to the inactive
    /// slot. Nothing is armed yet, so the next reset boots the running image.
    Prepared {
        /// The SVN the verifier read from the image's manifest before
        /// activation. The only authenticated reading of it, so the only
        /// value the anti-rollback commit may trust.
        svn: Svn,
    },
    /// The trial is armed: the next reset boots the prepared image once.
    TrialPending {
        /// The SVN recorded by [`prepare`](SelfUpdate::prepare).
        svn: Svn,
    },
    /// The trial image confirmed itself. The anti-rollback floor has not
    /// necessarily taken the SVN yet, which is what
    /// [`TrialOutcome::ConfirmedUncommitted`] covers.
    Committed {
        /// The SVN to advance the floor to.
        svn: Svn,
    },
}

/// What became of a self-update trial, as the boot after it finds things.
///
/// A verdict, not an instruction: it says what happened, and the state machine
/// decides what that costs. Total over [`SelfUpdateState`] and
/// [`RunningImage`] (see [`trial_outcome`]), so no caller has to read the
/// storage states for itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrialOutcome {
    /// No update was in flight and the confirmed image is running.
    NoSession,
    /// A session exists that nothing will confirm: the trial booted and fell
    /// back, or it was never armed, or an image is running that the session
    /// does not claim. All of them leave an update that has to be dropped
    /// before another can start.
    ///
    /// When the unclaimed trial is the image running right now, reverting
    /// only moves slot metadata: the part keeps running it until the next
    /// reset.
    Unconfirmed,
    /// This boot is the trial. Nothing has judged it yet.
    InProgress,
    /// The trial confirmed itself, and the anti-rollback floor has not
    /// necessarily taken the SVN yet.
    ConfirmedUncommitted {
        /// The verified SVN the session recorded.
        svn: Svn,
    },
}

/// Persistent session for the eRoT's own update, surviving power loss and the
/// self-reset that activation needs.
///
/// Where the session lives (protected flash, a reserved region, a mock in
/// tests) is the implementor's concern, as is how the platform arms the boot
/// selector. A platform whose boot selector reads a record the eRoT writes
/// folds the arming into the session's own write, and its
/// [`arm_trial`](Self::arm_trial) does nothing beyond moving the state. A
/// platform whose boot selector is a register or a ROM-defined table writes
/// twice behind the seam. Neither shows here.
///
/// # Transitions
///
/// [`prepare`](Self::prepare) records a new session from any state,
/// [`arm_trial`](Self::arm_trial) moves `Prepared` to `TrialPending`,
/// [`confirm`](Self::confirm) moves `TrialPending` to `Committed`,
/// [`complete`](Self::complete) ends a committed session, and
/// [`revert`](Self::revert) drops whatever is in flight. Both endings land on
/// `Idle`; they are separate calls because the caller reaching them is
/// different, the commit path and the abandon path.
///
/// Every call is idempotent: repeating a transition that already happened
/// succeeds and changes nothing, the same shape
/// [`SvnFloor::advance`](crate::SvnFloor::advance) uses. A caller that needs
/// to tell a repeat from a session that was never there reads
/// [`state`](Self::state) first.
///
/// # Durability
///
/// A transition that returns `Ok` survives power loss, and a transition cut
/// short by power loss leaves the state before the call or the state the call
/// wrote, never a mix. Storage that cannot write a state atomically gets there
/// the usual way, two copies with a sequence number and a checksum, writing
/// the stale copy and reading the newest valid one.
///
/// # Boot bound
///
/// A boot selector that holds the arming across resets boots the trial again
/// after every reset, so a trial image that dies before anything confirms it
/// loops forever unless something counts attempts. A selector that consumes
/// the arming does not have this problem: the second boot runs the confirmed
/// image. Platforms in the first category need a ROM-side boot counter or
/// equivalent bound on how many boots a trial gets.
///
/// # Ordering
///
/// Record first, then arm. A crash between the two leaves `TrialPending` with
/// the confirmed image still running, which [`trial_outcome`] already reads as
/// a trial nothing confirmed, so the window needs no further distinction.
/// Arming first would boot a trial with no session to judge it.
pub trait SelfUpdate {
    /// The error type of this session's storage.
    ///
    /// Bounded by [`core::error::Error`] so the orchestrator gets `Display`
    /// and a `source()` cause chain, not just a `Debug` dump. Error
    /// categories are implementation-defined.
    type Error: core::error::Error;

    /// The session as durable storage holds it.
    fn state(&self) -> Result<SelfUpdateState, Self::Error>;

    /// Which image this boot is running.
    ///
    /// [`Confirmed`](RunningImage::Confirmed) whenever no trial is in flight,
    /// an `Idle` session included: with nothing armed there is nothing else to
    /// be running.
    ///
    /// Read together with [`state`](Self::state) once at boot, as one
    /// snapshot. Nothing moves the pair in between: the eRoT is the only
    /// writer and it is not running its own update while deciding the last
    /// one.
    fn running(&self) -> Result<RunningImage, Self::Error>;

    /// Records a new session with the verified SVN, leaving the state
    /// `Prepared`. Overwrites any earlier session, so a retry after a crash
    /// records the same SVN again.
    fn prepare(&mut self, svn: Svn) -> Result<(), Self::Error>;

    /// Arms the trial, leaving the state `TrialPending`. Called after the
    /// image is in the inactive slot and before the reset that boots it.
    ///
    /// # Errors
    ///
    /// When the session is not `Prepared` or `TrialPending`: there is nothing
    /// to arm, and arming a boot with no session behind it is what the
    /// ordering rule exists to prevent.
    fn arm_trial(&mut self) -> Result<(), Self::Error>;

    /// Records that the trial image judged itself good, leaving the state
    /// `Committed`. The anti-rollback floor is advanced by the caller
    /// afterwards, not here: this trait holds the session, it does not own
    /// the floor.
    ///
    /// # Errors
    ///
    /// When no trial is pending and the session is not already `Committed`.
    fn confirm(&mut self) -> Result<(), Self::Error>;

    /// Ends a committed session, leaving the state `Idle`. Called once the
    /// floor has taken the SVN.
    ///
    /// # Errors
    ///
    /// When the session is neither `Committed` nor already `Idle`. A session
    /// still in flight is dropped with [`revert`](Self::revert), so that a
    /// mistaken `complete` cannot silently discard a pending trial.
    fn complete(&mut self) -> Result<(), Self::Error>;

    /// Drops whatever is in flight, leaving the state `Idle`. Succeeds from
    /// any state, `Idle` included.
    ///
    /// Undoing the arming is the implementor's, when the platform holds it
    /// outside the session.
    fn revert(&mut self) -> Result<(), Self::Error>;
}

/// What became of the trial, from the session and the image that booted.
///
/// A session and a running image that disagree are not an error: they are the
/// crash windows this function exists to name.
#[must_use]
pub const fn trial_outcome(state: SelfUpdateState, running: RunningImage) -> TrialOutcome {
    match (state, running) {
        // Nothing in flight.
        (SelfUpdateState::Idle, RunningImage::Confirmed) => TrialOutcome::NoSession,
        // A trial image running with no session behind it: nothing records
        // what it was for, so nothing can confirm it.
        (SelfUpdateState::Idle, RunningImage::Trial) => TrialOutcome::Unconfirmed,
        // Prepared, and the reset that would have started the trial never
        // came.
        (SelfUpdateState::Prepared { .. }, RunningImage::Confirmed) => TrialOutcome::Unconfirmed,
        // A trial booted while the session still reads Prepared: the arming
        // ran ahead of the record, the ordering rule's other side.
        (SelfUpdateState::Prepared { .. }, RunningImage::Trial) => TrialOutcome::Unconfirmed,
        // The trial is what is running now.
        (SelfUpdateState::TrialPending { .. }, RunningImage::Trial) => TrialOutcome::InProgress,
        // The trial booted and the platform fell back, so the image either
        // failed or never reached the point that confirms it.
        (SelfUpdateState::TrialPending { .. }, RunningImage::Confirmed) => {
            TrialOutcome::Unconfirmed
        }
        // Confirmed. The floor advance is the only step that may still be
        // missing, and repeating it is harmless.
        (SelfUpdateState::Committed { svn }, _) => TrialOutcome::ConfirmedUncommitted { svn },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock session fault")
        }
    }

    impl core::error::Error for MockFault {}

    /// Implements SelfUpdate with no HAL dependency: the contract must be
    /// satisfiable from any stack (mock, IPC proxy, simulator). The state
    /// stands in for durable storage, and `fail` for a store that is not
    /// answering.
    struct MockSession {
        state: Cell<SelfUpdateState>,
        running: Cell<RunningImage>,
        fail: bool,
    }

    impl MockSession {
        fn idle() -> Self {
            Self {
                state: Cell::new(SelfUpdateState::Idle),
                running: Cell::new(RunningImage::Confirmed),
                fail: false,
            }
        }

        fn faulty() -> Self {
            Self {
                state: Cell::new(SelfUpdateState::Idle),
                running: Cell::new(RunningImage::Confirmed),
                fail: true,
            }
        }

        /// Stands in for the reset that boots what `arm_trial` armed.
        fn reboot_into_the_trial(&self) {
            self.running.set(RunningImage::Trial);
        }
    }

    impl SelfUpdate for MockSession {
        type Error = MockFault;

        fn state(&self) -> Result<SelfUpdateState, MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            Ok(self.state.get())
        }

        fn running(&self) -> Result<RunningImage, MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            Ok(self.running.get())
        }

        fn prepare(&mut self, svn: Svn) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            self.state.set(SelfUpdateState::Prepared { svn });
            Ok(())
        }

        fn arm_trial(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            match self.state.get() {
                SelfUpdateState::Prepared { svn } | SelfUpdateState::TrialPending { svn } => {
                    self.state.set(SelfUpdateState::TrialPending { svn });
                    Ok(())
                }
                _ => Err(MockFault),
            }
        }

        fn confirm(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            match self.state.get() {
                SelfUpdateState::TrialPending { svn } | SelfUpdateState::Committed { svn } => {
                    self.state.set(SelfUpdateState::Committed { svn });
                    Ok(())
                }
                _ => Err(MockFault),
            }
        }

        fn complete(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            match self.state.get() {
                SelfUpdateState::Committed { .. } | SelfUpdateState::Idle => {
                    self.state.set(SelfUpdateState::Idle);
                    Ok(())
                }
                _ => Err(MockFault),
            }
        }

        fn revert(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            self.state.set(SelfUpdateState::Idle);
            Ok(())
        }
    }

    const SVN: Svn = Svn(7);
    const NEWER: Svn = Svn(9);

    #[test]
    fn walks_a_whole_update() {
        let mut session = MockSession::idle();
        session.prepare(SVN).expect("store answers");
        assert_eq!(session.state(), Ok(SelfUpdateState::Prepared { svn: SVN }));
        session.arm_trial().expect("prepared");
        assert_eq!(
            session.state(),
            Ok(SelfUpdateState::TrialPending { svn: SVN })
        );
        session.reboot_into_the_trial();
        assert_eq!(
            trial_outcome(
                session.state().expect("store answers"),
                session.running().expect("store answers")
            ),
            TrialOutcome::InProgress
        );
        session.confirm().expect("trial pending");
        assert_eq!(session.state(), Ok(SelfUpdateState::Committed { svn: SVN }));
        session.complete().expect("committed");
        assert_eq!(session.state(), Ok(SelfUpdateState::Idle));
    }

    /// A retry after a crash records the new SVN over the old session.
    #[test]
    fn preparing_again_replaces_the_session() {
        let mut session = MockSession::idle();
        session.prepare(SVN).expect("store answers");
        session.prepare(NEWER).expect("store answers");
        assert_eq!(
            session.state(),
            Ok(SelfUpdateState::Prepared { svn: NEWER })
        );
    }

    /// Repeating a transition that already landed is how a caller retries
    /// after a crash, so it must not fail.
    #[test]
    fn repeating_a_transition_changes_nothing() {
        let mut session = MockSession::idle();
        session.prepare(SVN).expect("store answers");
        session.arm_trial().expect("prepared");
        session.arm_trial().expect("already armed");
        assert_eq!(
            session.state(),
            Ok(SelfUpdateState::TrialPending { svn: SVN })
        );
        session.confirm().expect("trial pending");
        session.confirm().expect("already committed");
        assert_eq!(session.state(), Ok(SelfUpdateState::Committed { svn: SVN }));
    }

    /// Dropping an update is allowed wherever it stands, including when
    /// there is nothing to drop.
    #[test]
    fn reverting_succeeds_from_any_state() {
        let mut session = MockSession::idle();
        session.revert().expect("idle");
        session.prepare(SVN).expect("store answers");
        session.revert().expect("prepared");
        assert_eq!(session.state(), Ok(SelfUpdateState::Idle));
    }

    /// `complete` is the commit path's ending and must not double as the
    /// abandon path's, or a mistaken call would discard a pending trial.
    #[test]
    fn completing_a_pending_trial_fails() {
        let mut session = MockSession::idle();
        session.prepare(SVN).expect("store answers");
        session.arm_trial().expect("prepared");
        assert_eq!(session.complete(), Err(MockFault));
        assert_eq!(
            session.state(),
            Ok(SelfUpdateState::TrialPending { svn: SVN })
        );
    }

    #[test]
    fn arming_with_no_session_fails() {
        let mut session = MockSession::idle();
        assert_eq!(session.arm_trial(), Err(MockFault));
        assert_eq!(session.state(), Ok(SelfUpdateState::Idle));
    }

    #[test]
    fn confirming_without_a_trial_fails() {
        let mut session = MockSession::idle();
        assert_eq!(session.confirm(), Err(MockFault));

        session.prepare(SVN).expect("store answers");
        assert_eq!(session.confirm(), Err(MockFault));
        assert_eq!(session.state(), Ok(SelfUpdateState::Prepared { svn: SVN }));
    }

    #[test]
    fn a_store_that_faults_surfaces_the_error() {
        let mut session = MockSession::faulty();
        assert_eq!(session.state(), Err(MockFault));
        assert_eq!(session.running(), Err(MockFault));
        assert_eq!(session.prepare(SVN), Err(MockFault));
        assert_eq!(session.arm_trial(), Err(MockFault));
        assert_eq!(session.confirm(), Err(MockFault));
        assert_eq!(session.complete(), Err(MockFault));
        assert_eq!(session.revert(), Err(MockFault));
    }

    #[test]
    fn reports_no_session_when_nothing_is_in_flight() {
        assert_eq!(
            trial_outcome(SelfUpdateState::Idle, RunningImage::Confirmed),
            TrialOutcome::NoSession
        );
    }

    /// The session was dropped while a trial image is the one running, so
    /// nothing records what it was for.
    #[test]
    fn reports_a_trial_image_with_no_session_unconfirmed() {
        assert_eq!(
            trial_outcome(SelfUpdateState::Idle, RunningImage::Trial),
            TrialOutcome::Unconfirmed
        );
    }

    /// Crashed after writing the session, before the reset that would have
    /// started the trial.
    #[test]
    fn reports_a_session_that_never_booted_unconfirmed() {
        assert_eq!(
            trial_outcome(
                SelfUpdateState::Prepared { svn: SVN },
                RunningImage::Confirmed
            ),
            TrialOutcome::Unconfirmed
        );
    }

    /// A trial booted while the session still reads Prepared, the other side
    /// of the record-then-arm rule.
    #[test]
    fn reports_a_trial_ahead_of_its_session_unconfirmed() {
        assert_eq!(
            trial_outcome(SelfUpdateState::Prepared { svn: SVN }, RunningImage::Trial),
            TrialOutcome::Unconfirmed
        );
    }

    /// The boot that is running is the trial itself, on a selector that
    /// consumes the arming and on one that holds it alike.
    #[test]
    fn reports_the_running_trial_in_progress() {
        assert_eq!(
            trial_outcome(
                SelfUpdateState::TrialPending { svn: SVN },
                RunningImage::Trial
            ),
            TrialOutcome::InProgress
        );
    }

    /// The trial booted and the platform fell back, so nothing confirmed it.
    #[test]
    fn reports_a_trial_that_fell_back_unconfirmed() {
        assert_eq!(
            trial_outcome(
                SelfUpdateState::TrialPending { svn: SVN },
                RunningImage::Confirmed
            ),
            TrialOutcome::Unconfirmed
        );
    }

    /// Crashed between `confirm` and the floor advance: the one case that
    /// commits rather than drops, and the reason the session carries a state
    /// and not only an SVN.
    #[test]
    fn reports_a_confirmed_trial_whose_floor_is_unmoved() {
        for running in [RunningImage::Trial, RunningImage::Confirmed] {
            assert_eq!(
                trial_outcome(SelfUpdateState::Committed { svn: SVN }, running),
                TrialOutcome::ConfirmedUncommitted { svn: SVN }
            );
        }
    }
}
