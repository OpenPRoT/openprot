// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`PlatformDriver`]: one executor method per [`Effect`] variant, routed from
//! the SM through the [`Platform`] impl.

use openprot_orchestrator_sm::{
    BootFailureKind, ComponentId, ComponentKind, Effect, EffectError, Event, Orchestrator, Platform,
};

use crate::board::{
    Board, BoardCapabilities, ImageSource, Report, ReportSink, SvnFloorBinding, Verdict, Verifier,
};
use orchestrator_capabilities::{
    BootControl, BootWatch, FailureCause, Recovery, RestoreOutcome, Svn, SvnFloor, WalkVerdict,
};

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
    /// An update is already in flight; the frontend answers the requester
    /// over its own protocol, the SM never sees the refused request.
    UpdateBusy,
    /// The recovery mechanism faulted (bus error, unreachable source).
    /// Distinct from source exhaustion, which is a verdict, not a fault.
    RecoveryFault,
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
            DriverError::UpdateBusy => "an update is already in flight",
            DriverError::RecoveryFault => "recovery mechanism faulted",
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
    /// `watching[i]`: `ComponentId(i)` is out of reset with a walk in
    /// flight. Set on `ReleaseReset`, cleared on `AssertReset` and on a
    /// terminal verdict. Only watched walks are polled, so a finished or
    /// quiesced walk emits no stale event.
    watching: [bool; N],
    /// `verified_svn[i]` is the manifest SVN of `ComponentId(i)`'s last
    /// authenticated image — the only value a floor commit may trust.
    /// `None` until a verification passes; cleared again on rejection.
    verified_svn: [Option<Svn>; N],
    /// The update job submitted by the frontend, target only for now; the
    /// pump state joins it when the executors land. Held until the update
    /// is activated or discarded.
    pending_update: Option<UpdateJob>,
}

/// One in-flight update, recorded by [`PlatformDriver::submit_update`].
struct UpdateJob {
    target: ComponentId,
}

impl<B: BoardCapabilities, const N: usize> PlatformDriver<B, N> {
    pub fn new(board: Board<B, N>) -> Self {
        // ComponentId is a u8, so ids for N > 256 components would wrap.
        const { assert!(N <= 256) };
        Self {
            board,
            staged: None,
            watching: [false; N],
            verified_svn: [None; N],
            pending_update: None,
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

    /// The frontend half of the update handshake: record `target` as the
    /// component the staged candidate is for. Must succeed BEFORE
    /// [`Event::UpdateRequest`] is dispatched; `StageUpdate` with no stored
    /// job fails closed. Refuses an unknown id and a second submit while
    /// one update is in flight; nothing is stored on refusal, so a refused
    /// request can never surface as an update event.
    pub fn submit_update(&mut self, target: ComponentId) -> Result<(), DriverError> {
        self.board
            .updatables
            .get(target.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        if self.pending_update.is_some() {
            return Err(DriverError::UpdateBusy);
        }
        self.pending_update = Some(UpdateJob { target });
        Ok(())
    }

    /// Target of the in-flight update, if one was submitted.
    pub fn pending_update(&self) -> Option<ComponentId> {
        self.pending_update.as_ref().map(|job| job.target)
    }

    /// `id`'s image source. Takes the array rather than `&mut self` so the
    /// caller can borrow `board.verifier` alongside the returned image.
    fn source(images: &mut [B::Image; N], id: ComponentId) -> Result<&mut B::Image, DriverError> {
        images
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)
    }

    /// Stage `id`'s image: open its source so
    /// [`verify_firmware`](Self::verify_firmware) can read it.
    pub fn stage_firmware(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.staged = None;
        let source = Self::source(&mut self.board.images, id)?;
        source.open().map_err(|_| DriverError::ImageUnavailable)?;
        self.staged = Some(id);
        Ok(())
    }

    /// Judge the staged image via the [`Verifier`] and return the verdict:
    /// `Event::VerificationPassed(id)` or `Event::VerificationFailed(id)`.
    pub fn verify_firmware(&mut self, id: ComponentId) -> Result<Event, DriverError> {
        // Id validity first: an unknown component is UnknownComponent even
        // though it can never be staged.
        let source = Self::source(&mut self.board.images, id)?;
        if self.staged != Some(id) {
            return Err(DriverError::NotStaged);
        }
        let verdict = self
            .board
            .verifier
            .verify(id, source)
            .map_err(|_| DriverError::VerifierFault)?;
        let idx = id.get() as usize;
        Ok(match verdict {
            Verdict::Authenticated { svn } => {
                self.verified_svn[idx] = Some(svn);
                Event::VerificationPassed(id)
            }
            Verdict::Rejected => {
                self.verified_svn[idx] = None;
                Event::VerificationFailed(id)
            }
        })
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

    /// Restore `id`'s image from its recovery source. The verdict travels
    /// as an event, not an error: `Restored` and `SourceExhausted` are
    /// outcomes the SM handles per failure policy, while an `Err` from
    /// the mechanism is a genuine actuation fault that fails closed.
    pub fn recover_component(
        &mut self,
        id: ComponentId,
        attempt: u8,
    ) -> Result<Event, DriverError> {
        let recovery = self
            .board
            .recovery
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        match recovery.restore(attempt) {
            Ok(RestoreOutcome::Restored) => Ok(Event::Restored(id)),
            Ok(RestoreOutcome::SourceExhausted) => Ok(Event::RecoveryUnavailable(id)),
            Err(_) => Err(DriverError::RecoveryFault),
        }
    }

    /// Hands one report to the board's sink. Cannot fail, so reporting stays
    /// off the fail-closed path; reports arrive in the order the SM emitted
    /// them.
    pub fn report(&mut self, report: Report) {
        self.board.report_sink.report(report);
    }
}

/// One [`PlatformDriver::poll_boot_walks`] round.
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
    /// variant must get an executor before this compiles. Synchronous
    /// results (the verification verdict) come back as the returned event;
    /// every executor error reports as [`EffectError`] — the SM treats all
    /// actuation failures the same, fail-closed.
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
        match effect {
            Effect::ReadFirmware(id) => self.stage_firmware(id).map(|_| None),
            Effect::VerifyFirmware(id) => self.verify_firmware(id).map(Some),
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
                self.pending_update = None;
                self.report(Report::UpdateDeferred);
                Ok(None)
            }
            Effect::ReportUpdateAborted => {
                self.pending_update = None;
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
            Effect::RecoverComponent { id, attempt } => {
                self.recover_component(id, attempt).map(Some)
            }
            // No board capability is composed for these seams yet, so they
            // fail closed here instead of behind stub methods. Each group
            // gains an executor when its capability joins
            // [`BoardCapabilities`], as BootControl did above: update
            // staging, authentication and trial activation for the update
            // quartet; evidence signing for SignAttestation; the terminal
            // latch for LatchLockdown.
            Effect::AuthenticateUpdate
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

/// The connection between an update frontend and the SM: called (by the
/// event loop, on the frontend's behalf) once a complete candidate for
/// `target` sits in the staging region. Records the job first, then injects
/// [`Event::UpdateRequest`]; that order is load-bearing, `StageUpdate` can
/// never run without a target. On refusal no event is injected and the
/// frontend answers the requester over its own protocol.
pub fn request_update<B: BoardCapabilities, const N: usize, const E: usize>(
    orchestrator: &mut Orchestrator<N, E>,
    driver: &mut PlatformDriver<B, N>,
    target: ComponentId,
) -> Result<(), DriverError> {
    driver.submit_update(target)?;
    orchestrator.dispatch(driver, Event::UpdateRequest);
    Ok(())
}
