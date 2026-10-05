// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The update pump: one in-flight update job, driven one bounded step
//! at a time by the event loop. Staging pushes bytes to the device,
//! then the board's verifier reads them back out of the staging region.
//!
//! A slot re-sync shares the machinery and ends here without an event,
//! because the SM never asked for it.

use super::{DriverError, PlatformDriver, UpdateJob, UpdatePhase, VerifierState};
use crate::board::{BoardCapabilities, Report};
use openprot_orchestrator_sm::{ComponentId, Event};
use orchestrator_capabilities::{
    IncrementalVerifier, PollOutcome, Progress, StageProgress, Updatable, VerifySession,
};
use util_io::{ByteSource, ByteWindow};

/// What one pump call established, before the stall rule is applied.
enum Step {
    /// The step ran and moved the job this far.
    Working(Progress),
    /// The device holds the complete payload; verification is next.
    Staged,
    /// The verifier authenticated the candidate.
    Authenticated,
    /// The candidate failed, or the device did.
    Rejected,
}

/// One [`PlatformDriver::pump_update`] round.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UpdatePoll {
    /// `UpdateVerified` when the verifier accepts the candidate, or
    /// `UpdateRejected` on fault, rejection, or stall.
    pub event: Option<Event>,
    /// How far the job has come, for the update source's progress
    /// report. `None` once the job has ended.
    pub progress: Option<Progress>,
}

impl UpdatePoll {
    /// No job, or a job whose next move is the SM's.
    pub(crate) const fn idle() -> Self {
        Self {
            event: None,
            progress: None,
        }
    }
}

impl<B: BoardCapabilities, const N: usize> PlatformDriver<B, N> {
    /// The frontend half of the update handshake: record `target` as the
    /// component the staged candidate is for and `len` as how much of the
    /// staging region the candidate occupies. Must succeed BEFORE
    /// [`Event::UpdateRequest`] is dispatched; `AuthenticateStageUpdate`
    /// with no stored job fails secure. Refuses an unknown id and a
    /// second submit while one update is in flight; nothing is stored on
    /// refusal, so a refused request can never surface as an update
    /// event.
    ///
    /// `len` is checked against the staging region here, the first point
    /// the driver sees it. Whether the candidate fits is settled for the
    /// requester when the offer is answered, before a byte transfers
    /// (see `RejectOffer` in the PLDM IPC design); this is the driver
    /// failing closed behind that. Refusing here keeps the SM out of
    /// `Updating` on a job that could never finish.
    ///
    /// The length stays on the job because the staging region is board
    /// geometry and usually larger, so the reader needs to know where
    /// the candidate ends.
    ///
    /// A slot re-sync counts as in flight. It is writing the staging
    /// region to a device, and a new candidate would overwrite those
    /// bytes halfway through.
    pub fn submit_update(&mut self, target: ComponentId, len: u64) -> Result<(), DriverError> {
        self.board
            .updatables
            .get(target.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        if len > self.board.update_staging.len() {
            return Err(DriverError::CandidateOutOfRange);
        }
        if self.pending_update.is_some() {
            return Err(DriverError::UpdateBusy);
        }
        // The region is about to hold a different candidate. The last
        // activation cannot re-stage from it any more.
        self.last_activated = None;
        self.pending_update = Some(UpdateJob {
            target,
            len,
            phase: UpdatePhase::Submitted,
            prepare_commanded: false,
            progress: Progress::start(len),
            progress_since_millis: None,
        });
        Ok(())
    }

    /// Discards the in-flight update: tells the device to drop what it
    /// staged and clears the job.
    ///
    /// Infallible on the device side ([`Updatable::abandon`] cannot fail),
    /// so the only refusal is having no job at all, which means the SM and
    /// the driver have drifted apart.
    pub fn discard_staged(&mut self) -> Result<(), DriverError> {
        let target = self
            .pending_update
            .as_ref()
            .ok_or(DriverError::NoUpdateJob)?
            .target;
        let updatable = self
            .board
            .updatables
            .get_mut(target.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        updatable.abandon();
        self.pending_update = None;
        // The region's contents are being dropped. A later commit must
        // not re-stage from it.
        self.last_activated = None;
        self.abandon_verification();
        Ok(())
    }

    /// Records the SM's `AuthenticateStageUpdate` command. Setting it
    /// twice is harmless, so a repeated command needs no guard.
    pub fn prepare_update(&mut self) -> Result<(), DriverError> {
        let job = self
            .pending_update
            .as_mut()
            .ok_or(DriverError::NoUpdateJob)?;
        job.prepare_commanded = true;
        Ok(())
    }

    /// Activates the staged candidate: the device's next boot runs it,
    /// tentatively. Clears the job, which has reached its end.
    ///
    /// The commit is not here. Activation proposes; `BootConfirmed` and
    /// `CommitSvnFloor` decide.
    pub fn activate_update(&mut self) -> Result<(), DriverError> {
        let job = self
            .pending_update
            .as_ref()
            .ok_or(DriverError::NoUpdateJob)?;
        if job.phase != UpdatePhase::Authenticated {
            return Err(DriverError::CandidateNotAuthenticated);
        }
        let (target, len) = (job.target, job.len);
        let updatable = self
            .board
            .updatables
            .get_mut(target.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        updatable.activate().map_err(|_| DriverError::UpdateFault)?;
        self.pending_update = None;
        self.last_activated = Some((target, len));
        Ok(())
    }

    /// One step of the in-flight update, called by the event loop between
    /// events, as [`poll_boot_walks`](Self::poll_boot_walks) is.
    ///
    /// Staging and verification each run one bounded step per call, so
    /// the loop stays live through a transfer that takes minutes. A job
    /// that stops making progress for longer than the board's stall
    /// budget is abandoned here rather than waited out.
    ///
    /// An update ends in `UpdateVerified` or `UpdateRejected`, and the
    /// job stays until the SM answers with `ActivateUpdate` or
    /// `DiscardStaged`.
    ///
    /// A slot re-sync is different. It ends here with no event and
    /// without verification. The SM never asked for it, a verdict would
    /// activate the image a second time, and the image is the one the
    /// device is already running, which was verified on its way in.
    pub fn pump_update(&mut self, now_millis: u64) -> UpdatePoll {
        let Some(job) = self.pending_update.as_mut() else {
            return UpdatePoll::idle();
        };
        let since = *job.progress_since_millis.get_or_insert(now_millis);
        let phase = job.phase;
        let before = job.progress;

        let stepped = match phase {
            UpdatePhase::Submitted if job.prepare_commanded => {
                job.phase = UpdatePhase::Staging;
                return UpdatePoll {
                    event: None,
                    progress: Some(job.progress),
                };
            }
            // Nothing to pump: the SM has not commanded the work yet,
            // or the SM owns the next move (Authenticated waits for
            // ActivateUpdate).
            UpdatePhase::Submitted | UpdatePhase::Authenticated => return UpdatePoll::idle(),
            UpdatePhase::Staging | UpdatePhase::Resyncing => self.poll_staging(),
            // Bytes are in. The verifier's state says whether this pump
            // starts a session or advances one.
            UpdatePhase::Staged => match self.update_verifier.take() {
                Some(VerifierState::Idle(verifier)) => {
                    self.update_verifier = Some(VerifierState::Verifying(verifier.start()));
                    return self.opened_verification(now_millis);
                }
                Some(VerifierState::Verifying(session)) => self.poll_verification(session),
                None => return self.reject_job(),
            },
        };

        let step = match stepped {
            Ok(step) => step,
            Err(_) => return self.reject_job(),
        };

        let job = match self.pending_update.as_mut() {
            Some(job) => job,
            None => return UpdatePoll::idle(),
        };
        match step {
            Step::Working(progress) => {
                job.progress = progress;
                if progress.written > before.written {
                    job.progress_since_millis = Some(now_millis);
                } else if now_millis.saturating_sub(since) >= self.board.update_stall_budget_millis
                {
                    return match phase {
                        UpdatePhase::Resyncing => self.end_resync(),
                        _ => self.reject_job(),
                    };
                }
                UpdatePoll {
                    event: None,
                    // Nobody asked for a re-sync, so there is no
                    // requester to report its progress to.
                    progress: (phase != UpdatePhase::Resyncing).then_some(progress),
                }
            }
            // A re-sync ends in the driver. Telling the SM the payload
            // is staged would activate the image a second time, and the
            // device is already running it.
            Step::Staged if phase == UpdatePhase::Resyncing => {
                self.pending_update = None;
                // Both slots hold the image now, so the floor may move.
                if let Some((id, svn)) = self.held_floor.take()
                    && self.advance_floor(id, svn).is_err()
                {
                    self.report(Report::SvnFloorCommitFailed(id));
                }
                UpdatePoll::idle()
            }
            Step::Staged => {
                job.phase = UpdatePhase::Staged;
                UpdatePoll {
                    event: None,
                    progress: Some(job.progress),
                }
            }
            Step::Authenticated => {
                job.phase = UpdatePhase::Authenticated;
                UpdatePoll {
                    event: Some(Event::UpdateVerified),
                    progress: None,
                }
            }
            // A failed re-sync leaves the running image committed and
            // the spare slot stale. Report it. The SM has nothing to
            // decide here.
            Step::Rejected if phase == UpdatePhase::Resyncing => self.end_resync(),
            Step::Rejected => self.reject_job(),
        }
    }

    /// Runs on the pump that starts the session. Verification counts its
    /// own bytes from zero, so the progress and the stall budget start
    /// over here.
    fn opened_verification(&mut self, now_millis: u64) -> UpdatePoll {
        let Some(job) = self.pending_update.as_mut() else {
            return UpdatePoll::idle();
        };
        job.progress = Progress::start(job.len);
        job.progress_since_millis = Some(now_millis);
        UpdatePoll {
            event: None,
            progress: Some(job.progress),
        }
    }

    /// Drops a session the driver cannot poll and returns `error`.
    /// `poll_verification` takes the session before it knows the step can
    /// run, so every early return has to put the verifier back. Miss one
    /// and `update_verifier` stays `None`, and nothing is ever verified
    /// again.
    fn drop_session(
        &mut self,
        session: <B::UpdateVerifier as IncrementalVerifier>::Session,
        error: DriverError,
    ) -> DriverError {
        self.update_verifier = Some(VerifierState::Idle(session.abandon()));
        error
    }

    /// One verification step: polls `session` over the staging region,
    /// the same window staging wrote through. An error means the driver
    /// could not run the step. A bad candidate is `Step::Rejected`.
    fn poll_verification(
        &mut self,
        session: <B::UpdateVerifier as IncrementalVerifier>::Session,
    ) -> Result<Step, DriverError> {
        let len = match self.pending_update.as_ref() {
            Some(job) => job.len,
            None => return Err(self.drop_session(session, DriverError::NoUpdateJob)),
        };
        let window = match ByteWindow::new(&self.board.update_staging, 0, len) {
            Ok(window) => window,
            Err(_) => return Err(self.drop_session(session, DriverError::CandidateOutOfRange)),
        };
        Ok(match session.poll(&window) {
            PollOutcome::Processing { session, progress } => {
                self.update_verifier = Some(VerifierState::Verifying(session));
                Step::Working(progress)
            }
            PollOutcome::Authenticated(v) => {
                self.update_verifier = Some(VerifierState::Idle(v));
                Step::Authenticated
            }
            PollOutcome::Rejected(v) => {
                self.update_verifier = Some(VerifierState::Idle(v));
                Step::Rejected
            }
            PollOutcome::Fault(v, _) => {
                self.update_verifier = Some(VerifierState::Idle(v));
                // A fault and a bad image both end the job with
                // UpdateRejected. The report says which one happened.
                self.report(Report::UpdateVerifierFault);
                Step::Rejected
            }
        })
    }

    /// One staging step: borrows the staging region and the device as
    /// separate fields so the window can be read while the device writes.
    fn poll_staging(&mut self) -> Result<Step, DriverError> {
        let job = self
            .pending_update
            .as_ref()
            .ok_or(DriverError::NoUpdateJob)?;
        if !job.prepare_commanded {
            return Err(DriverError::UpdateNotCommanded);
        }
        let (target, len) = (job.target, job.len);
        let window = ByteWindow::new(&self.board.update_staging, 0, len)
            .map_err(|_| DriverError::CandidateOutOfRange)?;
        let updatable = self
            .board
            .updatables
            .get_mut(target.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        match updatable.poll_stage(&window) {
            Ok(StageProgress::Transferring { progress }) => Ok(Step::Working(progress)),
            Ok(StageProgress::Ready) => Ok(Step::Staged),
            Err(_) => Ok(Step::Rejected),
        }
    }

    /// Ends a re-sync that failed or stalled. The SM never knew about
    /// this job, so nothing else clears it and the pump would retry the
    /// same failure forever. The running image stays committed. The
    /// stale slot is reported.
    ///
    /// Rejecting instead would hand the SM a verdict for a job it never
    /// started. `reject_job` keeps the job and waits for `DiscardStaged`,
    /// which only `Updating` sends, and the SM is in `Ready` here. The
    /// job would never clear and every later update would be refused as
    /// busy.
    fn end_resync(&mut self) -> UpdatePoll {
        let Some(job) = self.pending_update.take() else {
            return UpdatePoll::idle();
        };
        if let Some(updatable) = self.board.updatables.get_mut(job.target.get() as usize) {
            updatable.abandon();
        }
        // The spare still holds the old image, so the floor stays where
        // it is. Moving it would leave the spare below the floor and
        // unbootable, which is what the ordering exists to avoid.
        self.held_floor = None;
        self.report(Report::SlotResyncFailed(job.target));
        UpdatePoll::idle()
    }

    /// Ends the job the way the SM understands: the device drops what it
    /// staged and the verdict travels as `UpdateRejected`. The job itself
    /// stays until the SM answers with `DiscardStaged`, so the two sides
    /// never disagree about whether an update is in flight.
    fn reject_job(&mut self) -> UpdatePoll {
        if let Some(job) = self.pending_update.as_mut() {
            job.phase = UpdatePhase::Submitted;
            job.prepare_commanded = false;
        }
        self.abandon_job();
        self.abandon_verification();
        UpdatePoll {
            event: Some(Event::UpdateRejected),
            progress: None,
        }
    }

    /// Tells the device to drop what it was staging. Leaves the job
    /// alone: the SM clears it when it answers `DiscardStaged`.
    fn abandon_job(&mut self) {
        let Some(job) = self.pending_update.as_ref() else {
            return;
        };
        let target = job.target.get() as usize;
        if let Some(updatable) = self.board.updatables.get_mut(target) {
            updatable.abandon();
        }
    }

    /// If a verification session is active, abandons it and returns the
    /// verifier to idle.
    fn abandon_verification(&mut self) {
        let state = self.update_verifier.take();
        self.update_verifier = match state {
            Some(VerifierState::Verifying(session)) => Some(VerifierState::Idle(session.abandon())),
            other => other,
        };
    }

    /// Target of the in-flight update, if one was submitted.
    pub fn pending_update(&self) -> Option<ComponentId> {
        self.pending_update.as_ref().map(|job| job.target)
    }
}
