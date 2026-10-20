// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The eRoT's own update: settling the last one at boot, confirming a
//! trial the chain walk judged, and advancing the eRoT's own floor once
//! the update agent asks.
//!
//! The activation that starts a trial is in
//! [`update`](super::update), with the rest of the update pump.

use openprot_orchestrator_sm::{Event, Orchestrator};
use orchestrator_capabilities::{
    RunningImage, SelfUpdate, SelfUpdateState, Svn, SvnFloor, TrialOutcome,
};

use super::{DriverError, PlatformDriver};
use crate::board::BoardCapabilities;

impl<B: BoardCapabilities, const N: usize> PlatformDriver<B, N> {
    /// Settles the eRoT's last self-update at the start of this boot, from
    /// the session and floor the board wired, and says what it found.
    ///
    /// The same call as [`settle_self_update`], reaching the pair through
    /// [`Board::self_image`] so an event loop that only holds the driver can
    /// make it. A board that wired no self-image answers `Settled`: no
    /// session is no session, whether it is missing or empty.
    ///
    /// Runs once, before [`Event::PowerGood`], and the caller dispatches
    /// [`SelfUpdateSettlement::event`] before that. A later run names an
    /// event the state machine drops: the trial is judged once per boot, by
    /// the walk that has already run.
    pub fn settle_self_image(
        &mut self,
    ) -> Result<SelfUpdateSettlement, SettleError<SelfUpdateError<B>, SvnFloorError<B>>> {
        let Some(self_image) = self.board.self_image.as_mut() else {
            return Ok(SelfUpdateSettlement::Settled);
        };
        settle_self_update(&mut self_image.session, &self_image.floor)
    }

    /// Confirms the eRoT's own update session: the trial image booted and
    /// the chain came up behind it.
    ///
    /// Moves the session to `Committed`. The update agent sends
    /// UpdateSecurityRevision when it sends it; what the confirm decides is
    /// the answer, since `commit_self_svn_floor` refuses a session that does
    /// not read `Committed`. The floor does not move here, and nothing reads
    /// the SVN: the session recorded it at `prepare`, and that is the only
    /// authenticated reading of it.
    ///
    /// `confirm` is idempotent, so a board whose session already reads
    /// `Committed` answers `Ok`. A board with no self-image at all is a
    /// different matter and fails secure: the effect only ever follows the
    /// event the self-image produced, so reaching here without one means the
    /// board and the machine disagree about whether this eRoT updates
    /// itself.
    pub(super) fn confirm_self_trial(&mut self) -> Result<(), DriverError> {
        let self_image = self
            .board
            .self_image
            .as_mut()
            .ok_or(DriverError::NoSelfImage)?;
        self_image
            .session
            .confirm()
            .map_err(|_| DriverError::SelfUpdateFault)
    }
}

/// The error type of the board's self-update session, spelled once so
/// [`PlatformDriver::settle_self_image`] fits on a line.
type SelfUpdateError<B> = <<B as BoardCapabilities>::SelfUpdate as SelfUpdate>::Error;

/// The error type of the board's anti-rollback floor, for the same reason.
type SvnFloorError<B> = <<B as BoardCapabilities>::SvnFloor as SvnFloor>::Error;

/// What [`settle_self_update`] found in the eRoT's last self-update.
///
/// What to do about it is the caller's, the same split the capability seams
/// use.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelfUpdateSettlement {
    /// Nothing is left to do, so the boot carries on and a new self-update
    /// may start: no session, a session nothing would confirm and that was
    /// reverted, or a confirmed session whose floor had already taken its
    /// SVN.
    Settled,
    /// This boot is the trial run of the eRoT's own update, and nothing has
    /// judged it yet. The chain walk is what judges it: the trial image
    /// booted, and bringing the platform up is what it has to prove. No new
    /// self-update may start: recording one would overwrite the session.
    TrialRunning {
        /// The SVN the session recorded, and the floor's target once the
        /// trial is confirmed.
        svn: Svn,
    },
    /// The trial was confirmed and the floor is still below `svn`. The
    /// advance is the update agent's to ask for with UpdateSecurityRevision,
    /// so this boot leaves it alone. No new self-update may start: recording
    /// one would overwrite the session.
    AwaitingFloorAdvance {
        /// The SVN the session recorded, and the floor's target.
        svn: Svn,
    },
    /// A trial image is running that no session claims, so the eRoT must not
    /// keep running it. The session is reverted, which clears the pending
    /// mark, so the confirmed image runs after a reset. An implementation
    /// whose `revert` leaves the mark in place would run the same image
    /// again.
    UnclaimedImageRunning,
}

impl SelfUpdateSettlement {
    /// Which event the caller dispatches into the state machine for this
    /// settlement, if any.
    ///
    /// Only a running trial needs one: the state machine judges that trial
    /// by reaching [`State::Ready`](openprot_orchestrator_sm::State), so it
    /// has to know the trial is running before the walk starts. The other
    /// settlements are the caller's to act on and change nothing about the
    /// walk, so they dispatch no event.
    #[must_use]
    pub const fn event(self) -> Option<Event> {
        match self {
            SelfUpdateSettlement::TrialRunning { .. } => Some(Event::SelfTrialBooted),
            SelfUpdateSettlement::Settled
            | SelfUpdateSettlement::AwaitingFloorAdvance { .. }
            | SelfUpdateSettlement::UnclaimedImageRunning => None,
        }
    }
}

/// Finishes off the eRoT's last self-update, at the start of the next boot.
///
/// An update writes the new image to the secondary slot and marks the session
/// pending, so the next reset runs that image once. That trial run is then
/// either confirmed or reverted. The eRoT loses all of RAM across the reset,
/// so the session in durable storage plus the image this boot is running is
/// everything it has to go on. This reads both and says what it found:
///
/// - No session: nothing happened, carry on.
/// - The session is pending and the trial image booted: this boot is the trial
///   run. Nothing here judges it. The chain walk does, by bringing the
///   platform up, so the session is left alone.
/// - The session is prepared or pending but the confirmed image booted: the
///   update never ran, or its trial failed and the platform fell back. The
///   session is reverted, so the update is done again rather than counted as
///   finished.
/// - A trial image is running that no session claims: the session is reverted,
///   which also clears the pending mark, so the confirmed image runs after a
///   reset. The eRoT must not keep running an image nothing vouched for.
/// - The session is confirmed and the floor is still below its SVN: the floor
///   advance is what is left, and the update agent asks for it, so this boot
///   must not do it.
/// - The session is confirmed and the floor already reads that SVN or higher:
///   the advance landed before a crash, so the session is closed here.
///
/// Running it twice lands in the same place, which is what lets a boot that
/// died partway through simply repeat it. A storage fault leaves the session
/// as it is and comes back as [`SettleError`] with the storage's own error
/// inside, rather than guessing.
///
/// Must run before the machine can grant a new update: `prepare` overwrites
/// whatever session is there, so a new update recorded over one still being
/// judged would lose what it owes.
///
/// The trial gets one boot. A platform whose pending mark survives a reset has
/// to clear it before this runs, otherwise an unplanned reset runs the trial
/// image again.
///
/// Takes the session and the eRoT's own floor directly. Neither is board
/// wiring: nothing calls this yet, and a seam joins [`BoardCapabilities`] when
/// an executor needs it.
pub fn settle_self_update<S: SelfUpdate, F: SvnFloor>(
    session: &mut S,
    floor: &F,
) -> Result<SelfUpdateSettlement, SettleError<S::Error, F::Error>> {
    let state = session.state().map_err(SettleError::Session)?;
    let running = session.running().map_err(SettleError::Session)?;
    match orchestrator_capabilities::trial_outcome(state, running) {
        TrialOutcome::NoSession => Ok(SelfUpdateSettlement::Settled),
        TrialOutcome::InProgress => {
            // Nothing here judges the trial. The chain walk that follows
            // does, by reaching Ready.
            let SelfUpdateState::TrialPending { svn } = state else {
                // trial_outcome answers InProgress for TrialPending alone.
                // Fail secure rather than guess which SVN was recorded.
                return Err(SettleError::InconsistentSession(state));
            };
            Ok(SelfUpdateSettlement::TrialRunning { svn })
        }
        TrialOutcome::Unconfirmed => {
            session.revert().map_err(SettleError::Session)?;
            if running == RunningImage::Trial {
                return Ok(SelfUpdateSettlement::UnclaimedImageRunning);
            }
            Ok(SelfUpdateSettlement::Settled)
        }
        TrialOutcome::ConfirmedUncommitted { svn } => {
            let reached = floor.floor().map_err(SettleError::Floor)?;
            if reached >= svn {
                session.complete().map_err(SettleError::Session)?;
                return Ok(SelfUpdateSettlement::Settled);
            }
            Ok(SelfUpdateSettlement::AwaitingFloorAdvance { svn })
        }
    }
}

/// Why [`settle_self_update`] could not finish.
///
/// Carries the storage error rather than flattening it, which is what the
/// `core::error::Error` bound on both seams is for. A caller that only needs
/// to fail secure collapses it to one [`DriverError`] in a line.
#[derive(Debug, PartialEq, Eq)]
pub enum SettleError<S, F> {
    /// The session could not be read or written.
    Session(S),
    /// The eRoT's own anti-rollback floor could not be read.
    Floor(F),
    /// `trial_outcome` said a trial is in progress for a state that is not
    /// `TrialPending`. It cannot today, since that is the only state it
    /// answers `InProgress` for. Kept so a change there fails secure rather
    /// than guessing which SVN was recorded.
    InconsistentSession(SelfUpdateState),
}

impl<S: core::fmt::Display, F: core::fmt::Display> core::fmt::Display for SettleError<S, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SettleError::Session(err) => write!(f, "self-update session: {err}"),
            SettleError::Floor(err) => write!(f, "self-update floor: {err}"),
            SettleError::InconsistentSession(state) => {
                write!(
                    f,
                    "self-update session reads {state:?} and cannot be settled"
                )
            }
        }
    }
}

impl<S: core::error::Error + 'static, F: core::error::Error + 'static> core::error::Error
    for SettleError<S, F>
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            SettleError::Session(err) => Some(err),
            SettleError::Floor(err) => Some(err),
            SettleError::InconsistentSession(_) => None,
        }
    }
}

/// Advances the eRoT's own anti-rollback floor to the SVN of its confirmed
/// self-update, then closes the session.
///
/// Call this when the update agent sends UpdateSecurityRevision, which the FD
/// reports as `SvnCommitPending`. The caller answers the agent with what comes
/// back: done on `Ok`, refused on `Err`. Activating an image never moves the
/// floor. It moves only here, when the update agent asks after a trial boot
/// was confirmed.
///
/// Refuses unless the session says confirmed and is still open. With no
/// confirmed session, nothing has run the image this SVN belongs to, so the
/// floor stays where it is.
///
/// Asking twice is safe. If the eRoT crashes after the floor moved but before
/// the session closed, the next UpdateSecurityRevision advances the floor to
/// the same value again, which changes nothing, and closes the session. Once
/// the session is closed, any further one gets
/// [`CommitFloorError::NotConfirmed`]: an answer lost on the way back means
/// the agent's retry is refused even though the floor is already at the SVN
/// it asked for.
///
/// If the floor cannot be written, the session stays open, so the next
/// UpdateSecurityRevision runs the whole thing again and the advance is not
/// lost. The update agent is the only thing that retries; nothing here does.
///
/// Takes the session and the floor as arguments, as [`settle_self_update`]
/// does. Neither is wired into the board yet.
pub fn commit_self_svn_floor<S: SelfUpdate, F: SvnFloor>(
    session: &mut S,
    floor: &mut F,
) -> Result<(), CommitFloorError<S::Error, F::Error>> {
    let state = session.state().map_err(CommitFloorError::Session)?;
    let running = session.running().map_err(CommitFloorError::Session)?;
    let TrialOutcome::ConfirmedUncommitted { svn } =
        orchestrator_capabilities::trial_outcome(state, running)
    else {
        return Err(CommitFloorError::NotConfirmed(state));
    };
    floor.advance(svn).map_err(CommitFloorError::Floor)?;
    session.complete().map_err(CommitFloorError::Session)
}

/// Why [`commit_self_svn_floor`] could not advance the floor.
///
/// Keeps the session's or the floor's own error inside instead of flattening
/// both into one code, the same as [`SettleError`] does.
#[derive(Debug, PartialEq, Eq)]
pub enum CommitFloorError<S, F> {
    /// Reading or writing the session failed.
    Session(S),
    /// Writing the eRoT's own anti-rollback floor failed.
    Floor(F),
    /// No confirmed self-update is waiting for the floor, so there is no SVN
    /// an image has proven itself at. Carries what the session says: `Idle`
    /// when no self-update is in flight or the request arrived a second time,
    /// `TrialPending` when it arrived before the trial boot was judged.
    NotConfirmed(SelfUpdateState),
}

impl<S: core::fmt::Display, F: core::fmt::Display> core::fmt::Display for CommitFloorError<S, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CommitFloorError::Session(err) => write!(f, "self-update session: {err}"),
            CommitFloorError::Floor(err) => write!(f, "self-update floor: {err}"),
            CommitFloorError::NotConfirmed(state) => {
                write!(
                    f,
                    "no confirmed self-update to commit the floor to: session reads {state:?}"
                )
            }
        }
    }
}

impl<S: core::error::Error + 'static, F: core::error::Error + 'static> core::error::Error
    for CommitFloorError<S, F>
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            CommitFloorError::Session(err) => Some(err),
            CommitFloorError::Floor(err) => Some(err),
            CommitFloorError::NotConfirmed(_) => None,
        }
    }
}

/// Settles the eRoT's own self-update at the start of a boot and tells the
/// machine what it found.
///
/// Belongs between [`bring_up`] and the first [`Event::PowerGood`]. The
/// settlement decides whether a new self-update may start, and a trial that
/// is running has to be declared before the walk begins, because what judges
/// it is the walk reaching [`State::Ready`](openprot_orchestrator_sm::State).
///
/// Returns what [`PlatformDriver::settle_self_image`] found, so a caller that
/// needs more than the event (refusing a new self-update, acting on an
/// unclaimed image) reads it off the settlement. A board that wired no
/// self-image answers `Settled` and nothing is dispatched.
///
/// A storage fault comes back unchanged. Nothing is dispatched then: the
/// boot has no honest answer about what the last self-update was doing, and
/// the caller decides what that costs.
pub fn settle_self_image_at_boot<B: BoardCapabilities, const N: usize, const E: usize>(
    orchestrator: &mut Orchestrator<N, E>,
    driver: &mut PlatformDriver<B, N>,
) -> Result<SelfUpdateSettlement, SettleError<SelfUpdateError<B>, SvnFloorError<B>>> {
    let settlement = driver.settle_self_image()?;
    if let Some(event) = settlement.event() {
        orchestrator.dispatch(driver, event);
    }
    Ok(settlement)
}
