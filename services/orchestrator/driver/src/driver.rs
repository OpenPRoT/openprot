// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`PlatformDriver`]: one executor method per [`Effect`] variant, routed from
//! the SM through the [`Platform`] impl.

use openprot_orchestrator_sm::{
    BootFailureKind, ComponentId, ComponentKind, Effect, EffectError, Event, Platform,
};

use crate::board::{
    Board, BoardCapabilities, ImageSource, Report, ReportSink, SvnFloorBinding, Verdict, Verifier,
    VerifyStatus,
};
use orchestrator_capabilities::{BootControl, BootWatch, FailureCause, Svn, SvnFloor, WalkVerdict};

/// Why the driver could not carry out an effect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriverError {
    /// The effect names a component the driver has no device for.
    UnknownComponent,
    /// The component's image source could not be opened.
    ImageUnavailable,
    /// Verify was asked for a component whose image was never staged.
    NotStaged,
    /// The verifier could not perform the check (a failed image is a
    /// [`Verdict`], not an error).
    VerifierFault,
    /// The component's boot control could not actuate the reset line.
    BootControlFault,
    /// A floor commit was asked for a component with no verified image —
    /// the SVN to advance to is unknown; fail closed.
    NoVerifiedImage,
    /// The component's SVN floor could not be advanced.
    SvnFloorFault,
}

impl core::fmt::Display for DriverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            DriverError::UnknownComponent => "no device for this component id",
            DriverError::ImageUnavailable => "image source could not be opened",
            DriverError::NotStaged => "no image staged for this component",
            DriverError::VerifierFault => "verifier could not perform the check",
            DriverError::BootControlFault => "boot control could not actuate the reset",
            DriverError::NoVerifiedImage => "no verified image to commit the floor to",
            DriverError::SvnFloorFault => "svn floor could not be advanced",
        })
    }
}

impl core::error::Error for DriverError {}

/// The effect executors. Everything device-specific lives in the [`Board`];
/// the driver's own fields are bookkeeping.
pub struct PlatformDriver<B: BoardCapabilities, const N: usize> {
    board: Board<B, N>,
    /// Component whose image is staged (source opened) for verification.
    staged: Option<ComponentId>,
    /// Component whose verification is outstanding with the verifier. At most
    /// one: a new start or a re-stage cancels the old one, so its answer can
    /// never be taken for the current image's.
    verifying: Option<ComponentId>,
    /// When the outstanding check will give up, as of the last `poll_verify`.
    verify_deadline: Option<u64>,
    /// `watching[i]`: `ComponentId(i)` is out of reset with a walk in
    /// flight. Set on `ReleaseReset`, cleared on `AssertReset` and on a
    /// terminal verdict. Only watched walks are polled, so a finished or
    /// quiesced walk emits no stale event.
    watching: [bool; N],
    /// `verified_svn[i]` is the manifest SVN of `ComponentId(i)`'s last
    /// authenticated image — the only value a floor commit may trust.
    /// `None` until a verification passes; cleared again on rejection.
    verified_svn: [Option<Svn>; N],
}

impl<B: BoardCapabilities, const N: usize> PlatformDriver<B, N> {
    pub fn new(board: Board<B, N>) -> Self {
        // ComponentId is a u8, so ids for N > 256 components would wrap.
        const { assert!(N <= 256) };
        Self {
            board,
            staged: None,
            verifying: None,
            verify_deadline: None,
            watching: [false; N],
            verified_svn: [None; N],
        }
    }

    /// The board wiring, read-only, for the tests: they observe a
    /// capability after it moved into the driver, instead of every mock
    /// smuggling out a shared handle. Real consumers get targeted queries
    /// when they exist — not this.
    #[cfg(test)]
    pub(crate) fn board(&self) -> &Board<B, N> {
        &self.board
    }

    /// `id`'s image source. Takes the array rather than `&mut self` so the
    /// caller can borrow `board.verifier` alongside the returned image.
    fn source(images: &mut [B::Image; N], id: ComponentId) -> Result<&mut B::Image, DriverError> {
        images
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)
    }

    /// Stage `id`'s image: open its source so
    /// [`start_verify`](Self::start_verify) can read it.
    ///
    /// # Errors
    ///
    /// [`DriverError::UnknownComponent`] for an id the driver has no device
    /// for, [`DriverError::ImageUnavailable`] when the image cannot be
    /// opened, and [`DriverError::VerifierFault`] when cancelling a check
    /// still outstanding fails.
    pub fn stage_firmware(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.staged = None;
        // A new staging replaces the image an outstanding check was verifying.
        self.cancel_verify()?;
        let source = Self::source(&mut self.board.images, id)?;
        source.open().map_err(|_| DriverError::ImageUnavailable)?;
        self.staged = Some(id);
        Ok(())
    }

    /// Abandon the outstanding verification, if any. Its answer is discarded.
    fn cancel_verify(&mut self) -> Result<(), DriverError> {
        self.verify_deadline = None;
        if self.verifying.take().is_some() {
            self.board
                .verifier
                .cancel()
                .map_err(|_| DriverError::VerifierFault)?;
        }
        Ok(())
    }

    /// Begin verifying the staged image. Returns at once; the verdict arrives
    /// from [`poll`](Self::poll) as
    /// `Event::VerificationPassed(id)` or `Event::VerificationFailed(id)`.
    ///
    /// Starting supersedes any check still outstanding, and voids `id`'s
    /// recorded SVN: until this check answers, no earlier authenticated
    /// reading may reach the anti-rollback commit.
    ///
    /// # Errors
    ///
    /// [`DriverError::UnknownComponent`] for an id the driver has no device
    /// for, [`DriverError::NotStaged`] when `id`'s image was not staged, and
    /// [`DriverError::VerifierFault`] when the verifier could not cancel the
    /// old check or start the new one. Either way the recorded SVN is already
    /// void.
    pub fn start_verify(&mut self, id: ComponentId) -> Result<(), DriverError> {
        // Id validity first: an unknown component is UnknownComponent even
        // though it can never be staged.
        Self::source(&mut self.board.images, id)?;
        if self.staged != Some(id) {
            return Err(DriverError::NotStaged);
        }
        self.cancel_verify()?;
        // Void the recorded SVN before the verifier is asked, so it is gone
        // even if the verifier cannot start: no earlier reading may reach the
        // anti-rollback commit until a new check answers.
        self.verified_svn[id.get() as usize] = None;
        let source = Self::source(&mut self.board.images, id)?;
        self.board
            .verifier
            .start(id, source)
            .map_err(|_| DriverError::VerifierFault)?;
        self.verifying = Some(id);
        Ok(())
    }

    /// Collect the outstanding verification's outcome, if it has one. Never
    /// waits. A run loop calls [`poll`](Self::poll), which includes this and
    /// the boot walks; call this directly only to poll the verification alone.
    ///
    /// A passed check records the image's SVN for the floor commit; a
    /// rejection or a timeout clears it and reports
    /// `Event::VerificationFailed(id)`. A check that is still pending
    /// refreshes the deadline [`next_verify_deadline`](Self::next_verify_deadline)
    /// returns, so poll once after [`start_verify`](Self::start_verify)
    /// before relying on it.
    ///
    /// An error means the check could not be carried through. It ends the
    /// check, and the run loop should dispatch `Event::EffectFailed`, the same
    /// fail-closed answer an executor error gets.
    pub fn poll_verify(&mut self, now_millis: u64) -> Result<Option<Event>, DriverError> {
        let Some(id) = self.verifying else {
            return Ok(None);
        };
        let idx = id.get() as usize;
        match self.board.verifier.poll(now_millis) {
            Err(_) => {
                self.verifying = None;
                self.verify_deadline = None;
                Err(DriverError::VerifierFault)
            }
            Ok(VerifyStatus::Pending { deadline_millis }) => {
                self.verify_deadline = deadline_millis;
                Ok(None)
            }
            Ok(VerifyStatus::Done(Verdict::Authenticated { svn })) => {
                self.verifying = None;
                self.verify_deadline = None;
                self.verified_svn[idx] = Some(svn);
                Ok(Some(Event::VerificationPassed(id)))
            }
            Ok(VerifyStatus::Done(Verdict::Rejected) | VerifyStatus::TimedOut) => {
                self.verifying = None;
                self.verify_deadline = None;
                self.verified_svn[idx] = None;
                Ok(Some(Event::VerificationFailed(id)))
            }
        }
    }

    /// When the outstanding verification will give up, as of the last
    /// [`poll_verify`](Self::poll_verify), or `None` if none is outstanding or
    /// it has no deadline. The run loop must wake no later than this and poll
    /// again: a verifier that has stopped answering signals nothing, so the
    /// deadline is the only wake-up that turns its silence into a failed
    /// verification.
    pub fn next_verify_deadline(&self) -> Option<u64> {
        self.verify_deadline
    }

    /// Advance `id`'s anti-rollback floor to its verified image's SVN.
    /// A self-managed component keeps its own floor; the commit is a
    /// no-op. A target at or below the current floor is the capability's
    /// documented no-op, so a replayed commit is harmless.
    pub fn commit_svn_floor(&mut self, id: ComponentId) -> Result<(), DriverError> {
        let idx = id.get() as usize;
        let SvnFloorBinding::Erot(floor) = self
            .board
            .svn_floors
            .get_mut(idx)
            .ok_or(DriverError::UnknownComponent)?
        else {
            return Ok(());
        };
        let svn = self.verified_svn[idx].ok_or(DriverError::NoVerifiedImage)?;
        floor.advance(svn).map_err(|_| DriverError::SvnFloorFault)
    }

    /// `id`'s reset actuator.
    fn boot_control(&mut self, id: ComponentId) -> Result<&mut B::BootControl, DriverError> {
        self.board
            .boot_controls
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)
    }

    /// Release `id` from reset and arm its boot walk;
    /// [`poll_boot_walks`](Self::poll_boot_walks) feeds the verdict back
    /// as `ComponentReady(id)`/`Booted(id)`/`BootFailed { id, .. }`. Arms on every
    /// release: a retry re-release starts a fresh walk.
    pub fn release_reset(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.boot_control(id)?
            .release()
            .map_err(|_| DriverError::BootControlFault)?;
        let idx = id.get() as usize;
        // In bounds: boot_control(id) above already rejected unknown ids.
        self.board.boot_watches[idx].arm();
        self.watching[idx] = true;
        Ok(())
    }

    /// Hold `id` in reset — a durable quiesce, not a pulse; at-rest
    /// verification and the recovery re-walk depend on it. Also stops the
    /// boot walk: a held device produces no boot signal, so polling it
    /// could only yield a stale `BootFailed`.
    pub fn assert_reset(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.boot_control(id)?
            .hold_in_reset()
            .map_err(|_| DriverError::BootControlFault)?;
        self.watching[id.get() as usize] = false;
        Ok(())
    }

    /// Polls every watched walk at `now_millis` and returns the first
    /// terminal verdict as its event: [`WalkVerdict::Complete`] becomes
    /// `ComponentReady(id)` (`Active`) or `Booted(id)` (`Passive`),
    /// [`WalkVerdict::Failed`] becomes `BootFailed { id, checkpoint, kind }`.
    /// The finished walk stops being watched;
    /// each verdict is delivered once.
    ///
    /// Returns at the first event; drain by calling until
    /// [`BootWalkPoll::event`] is `None`. Only that last poll carries a
    /// complete [`next_deadline_millis`](BootWalkPoll::next_deadline_millis)
    /// — the earliest deadline among the still-waiting walks.
    pub fn poll_boot_walks(&mut self, now_millis: u64) -> BootWalkPoll {
        let mut next_deadline_millis: Option<u64> = None;
        for idx in 0..N {
            if !self.watching[idx] {
                continue;
            }
            let id: ComponentId = (idx as u8).into();
            match self.board.boot_watches[idx].poll(now_millis) {
                WalkVerdict::Waiting { deadline_millis } => {
                    next_deadline_millis = Some(match next_deadline_millis {
                        Some(d) => d.min(deadline_millis),
                        None => deadline_millis,
                    });
                }
                WalkVerdict::Complete => {
                    self.watching[idx] = false;
                    let event = match self.board.component_kinds[idx] {
                        ComponentKind::Active => Event::ComponentReady(id),
                        ComponentKind::Passive => Event::Booted(id),
                    };
                    return BootWalkPoll {
                        event: Some(event),
                        next_deadline_millis,
                    };
                }
                WalkVerdict::Failed { checkpoint, cause } => {
                    self.watching[idx] = false;
                    let kind = match cause {
                        FailureCause::TimedOut => BootFailureKind::TimedOut,
                        FailureCause::DeviceRetriable => BootFailureKind::DeviceRetriable,
                        FailureCause::DeviceFatal => BootFailureKind::DeviceFatal,
                    };
                    return BootWalkPoll {
                        event: Some(Event::BootFailed {
                            id,
                            checkpoint,
                            kind,
                        }),
                        next_deadline_millis,
                    };
                }
            }
        }
        BootWalkPoll {
            event: None,
            next_deadline_millis,
        }
    }

    /// One round of everything the run loop must poll: the verification, then
    /// the boot walks. Returns [`DriverPoll::Event`] at the first event, so
    /// dispatch it and call again. When nothing is ready it returns
    /// [`DriverPoll::Idle`] with the earliest deadline among the outstanding
    /// verification and the waiting walks: the loop sleeps until then or until
    /// a service signals, and polls again.
    ///
    /// Calling this after every dispatch, and sleeping only on `Idle`, is what
    /// keeps the loop from sleeping on a check it has not polled yet: a new
    /// check's deadline is learnt only from its first poll.
    ///
    /// # Errors
    ///
    /// [`DriverError::VerifierFault`] when the verification could not be
    /// carried through. Dispatch `Event::EffectFailed`, as for an executor
    /// error.
    pub fn poll(&mut self, now_millis: u64) -> Result<DriverPoll, DriverError> {
        if let Some(event) = self.poll_verify(now_millis)? {
            return Ok(DriverPoll::Event(event));
        }
        let walks = self.poll_boot_walks(now_millis);
        if let Some(event) = walks.event {
            return Ok(DriverPoll::Event(event));
        }
        let next_wake_millis = match (walks.next_deadline_millis, self.verify_deadline) {
            (Some(walk), Some(verify)) => Some(walk.min(verify)),
            (walk, verify) => walk.or(verify),
        };
        Ok(DriverPoll::Idle { next_wake_millis })
    }

    /// Hands one report to the board's sink. Cannot fail, so reporting stays
    /// off the fail-closed path; reports arrive in the order the SM emitted
    /// them.
    pub fn report(&mut self, report: Report) {
        self.board.report_sink.report(report);
    }
}

/// One [`PlatformDriver::poll`] round. Dropping it loses an event the driver
/// has already marked as delivered.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriverPoll {
    /// The first event from the verification or a boot walk. Dispatch it, then
    /// poll again.
    Event(Event),
    /// Nothing is ready. Sleep until a service signals or `next_wake_millis`
    /// (the earliest deadline among the outstanding verification and the
    /// waiting walks, `None` if there is none), then poll again.
    Idle { next_wake_millis: Option<u64> },
}

/// One [`PlatformDriver::poll_boot_walks`] round. Dropping it loses an event
/// the driver has already marked as delivered.
#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BootWalkPoll {
    /// The first terminal verdict's event; `None` when every watched walk
    /// is still waiting.
    pub event: Option<Event>,
    /// Earliest deadline among walks seen waiting this round. Complete only
    /// when [`event`](Self::event) is `None`: an early return skips the
    /// walks after the finished one.
    pub next_deadline_millis: Option<u64>,
}

impl<B: BoardCapabilities, const N: usize> Platform for PlatformDriver<B, N> {
    /// Routes each effect to its executor. Exhaustive: a new [`Effect`]
    /// variant must get an executor before this compiles. Nothing here waits
    /// on a slow service: the verification verdict, for one, arrives later
    /// from [`poll`](PlatformDriver::poll). Every executor
    /// error reports as [`EffectError`] — the SM treats all actuation
    /// failures the same, fail-closed.
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
        match effect {
            Effect::ReadFirmware(id) => self.stage_firmware(id).map(|_| None),
            Effect::VerifyFirmware(id) => self.start_verify(id).map(|_| None),
            Effect::ReleaseReset(id) => self.release_reset(id).map(|_| None),
            Effect::AssertReset(id) => self.assert_reset(id).map(|_| None),
            Effect::CommitSvnFloor(id) => self.commit_svn_floor(id).map(|_| None),
            // Reports carry no error, so they never reach the fail-closed
            // group below.
            Effect::ReportIsolated(id) => {
                self.report(Report::Isolated(id));
                Ok(None)
            }
            Effect::ReportRecoveryFailed(id) => {
                self.report(Report::RecoveryFailed(id));
                Ok(None)
            }
            Effect::ReportUpdateDeferred => {
                self.report(Report::UpdateDeferred);
                Ok(None)
            }
            Effect::ReportUpdateAborted => {
                self.report(Report::UpdateAborted);
                Ok(None)
            }
            Effect::ReportBootFailed {
                id,
                checkpoint,
                kind,
            } => {
                self.report(Report::BootFailed {
                    id,
                    checkpoint,
                    kind,
                });
                Ok(None)
            }
            // No board capability is composed for these seams yet, so they
            // fail closed here instead of behind stub methods. Each group
            // gains an executor when its capability joins
            // [`BoardCapabilities`], as BootControl did above: recovery
            // sourcing for RecoverComponent; update staging, authentication
            // and trial activation for the update quartet; evidence signing
            // for SignAttestation; the terminal latch for LatchLockdown.
            Effect::RecoverComponent { .. }
            | Effect::AuthenticateUpdate
            | Effect::StageUpdate
            | Effect::ActivateUpdate
            | Effect::DiscardStaged
            | Effect::SignAttestation
            | Effect::LatchLockdown => return Err(EffectError),
            // Emit is consumed by the orchestrator; receiving one is a
            // driver bug.
            Effect::Emit(_) => return Err(EffectError),
        }
        .map_err(|_| EffectError)
    }
}
