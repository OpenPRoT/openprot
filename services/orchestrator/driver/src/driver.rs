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
    trial_outcome, BootControl, BootWatch, FailureCause, Progress, Recovery, RestoreOutcome,
    SelfUpdate, StageProgress, Svn, SvnFloor, TrialOutcome, Updatable, WalkVerdict,
};
use util_io::{ByteSource, ByteWindow};

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
    /// A floor commit was asked for a component with no verified image,
    /// so the SVN to advance to is unknown; fail closed.
    NoVerifiedImage,
    /// The component's SVN floor could not be advanced.
    SvnFloorFault,
    /// An update is already in flight; the frontend answers the requester
    /// over its own protocol, the SM never sees the refused request.
    UpdateBusy,
    /// The device refused to activate what it staged.
    UpdateFault,
    /// The machine is in a state that answers nothing: pre-service or
    /// locked down. The request is refused instead of being dropped
    /// inside the state machine with no report.
    Unsupervised,
    /// The eRoT's own update session could not be read or written.
    SelfUpdateFault,
    /// A floor commit was asked for with no confirmed self-update behind
    /// it. Nothing has proven an image at that SVN, so the floor stays.
    NoSelfUpdateToCommit,
    /// The recovery mechanism faulted (bus error, unreachable source).
    /// Distinct from source exhaustion, which is a verdict, not a fault.
    RecoveryFault,
    /// An update effect ran with no job recorded. The frontend records
    /// the job before the SM sees `UpdateRequest`, so this means the two
    /// have drifted apart.
    NoUpdateJob,
    /// The candidate does not fit the staging region the board wired, so
    /// there is nothing well-formed to read. Refused at submit; the
    /// pump's window gives the same answer if it ever gets that far.
    CandidateOutOfRange,
    /// Activation was asked for before the device held the whole
    /// payload. The job survives, so `DiscardStaged` can still end it.
    CandidateNotStaged,
    /// A staging step ran before the SM commanded the work. The pump
    /// only reaches staging through `AuthenticateStageUpdate`, so this
    /// means the phase and the flag disagree.
    UpdateNotCommanded,
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
            DriverError::UpdateFault => "device refused to activate the staged image",
            DriverError::Unsupervised => "the platform is not in a state that answers requests",
            DriverError::SelfUpdateFault => "self-update session could not be read or written",
            DriverError::NoSelfUpdateToCommit => "no confirmed self-update to commit the floor to",
            DriverError::RecoveryFault => "recovery mechanism faulted",
            DriverError::NoUpdateJob => "no update job for this effect",
            DriverError::CandidateOutOfRange => "candidate does not fit the staging region",
            DriverError::CandidateNotStaged => "device does not hold the whole candidate yet",
            DriverError::UpdateNotCommanded => "staging ran before the SM commanded it",
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
    /// authenticated image, the only value a floor commit may trust.
    /// `None` until a verification passes; cleared again on rejection.
    verified_svn: [Option<Svn>; N],
    /// The update job submitted by the frontend. Held until the update is
    /// activated or discarded.
    pending_update: Option<UpdateJob>,
    /// The component and candidate length of the last activation, so a
    /// commit can re-stage the same payload into the spare slot. Cleared
    /// once that re-sync is armed.
    last_activated: Option<(ComponentId, u64)>,
}

/// What one pump call established, before the stall rule is applied.
enum Step {
    /// The step ran and moved the job this far.
    Working(Progress),
    /// The device holds the complete payload; verification is next.
    Staged,
    /// The crypto service authenticated the candidate.
    #[allow(dead_code)]
    Authenticated,
    /// The candidate failed, or the device did.
    Rejected,
}

/// One in-flight update, recorded by [`PlatformDriver::submit_update`].
struct UpdateJob {
    target: ComponentId,
    /// Candidate length in bytes, from the offer the source accepted. The
    /// staging region is board geometry and usually larger, so the job
    /// carries what part of it holds this candidate.
    len: u64,
    phase: UpdatePhase,
    /// Set when the SM commands `AuthenticateStageUpdate`. The pump reads
    /// this to know the SM has spoken; `phase` tracks how far execution
    /// has gotten. Setting it twice is harmless, so a repeated command
    /// needs no guard.
    prepare_commanded: bool,
    /// Progress at the last pump call, and when it last moved. The pump
    /// judges a stall against these; both phases count bytes the same
    /// way, so one rule covers staging and authentication.
    progress: Progress,
    /// `None` until the first pump call: the job is recorded before the
    /// event loop has a clock reading for it.
    progress_since_millis: Option<u64>,
}

/// How far the in-flight update has come.
///
/// The SM emits `AuthenticateStageUpdate` on entry to `Updating` and the
/// driver sequences the work: bytes are staged first, then the crypto
/// service verifies the staged image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdatePhase {
    /// Recorded by the frontend, no executor has run yet.
    Submitted,
    /// `poll_stage` is pushing bytes to the device.
    Staging,
    /// The device holds the complete payload. The crypto service has not
    /// started yet.
    Staged,
    /// The committed image is being written a second time, to bring the
    /// slot the device just stopped booting from up to date. The SM is
    /// not involved: this job ends in the driver.
    Resyncing,
    // Authenticating and Authenticated arrive with the crypto
    // verify-client trait. Until then the pump parks at Staged.
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
            last_activated: None,
        }
    }

    /// The board wiring, read-only, for the tests: they observe a
    /// capability after it moved into the driver, instead of every mock
    /// smuggling out a shared handle. Real consumers get targeted queries
    /// when they exist, not this.
    #[cfg(test)]
    pub(crate) fn board(&self) -> &Board<B, N> {
        &self.board
    }

    /// The board wiring, to fault a capability mid-test.
    #[cfg(test)]
    pub(crate) fn board_mut(&mut self) -> &mut Board<B, N> {
        &mut self.board
    }

    /// The frontend half of the update handshake: record `target` as the
    /// component the staged candidate is for and `len` as how much of the
    /// staging region the candidate occupies. Must succeed BEFORE
    /// [`Event::UpdateRequest`] is dispatched; `AuthenticateStageUpdate`
    /// with no stored job fails closed. Refuses an unknown id and a
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
    /// A slot re-sync counts as in flight: it is writing the staging
    /// region's bytes to a device, and a new candidate would overwrite
    /// them mid-pass.
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
        // The region is about to hold a different candidate, so the last
        // activation loses its claim on a re-sync from it.
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
        // Whatever the region held is being dropped, so a later commit
        // must not re-stage from it.
        self.last_activated = None;
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
        if job.phase != UpdatePhase::Staged {
            // TODO: gate on Authenticated once the crypto verify-client
            // trait is wired into the board.
            return Err(DriverError::CandidateNotStaged);
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

    /// Settles the eRoT's last self-update, once, at boot before the walk.
    ///
    /// The eRoT's own update is judged by a boot that has to read what the
    /// previous one left behind, so the verdict comes from durable state:
    /// the session plus the image this boot is running.
    ///
    /// - No session, or a trial that is running right now: nothing to
    ///   settle. A trial in progress is judged later, by the boot it is
    ///   part of, not here.
    /// - A session nothing will confirm (the trial fell back, or was
    ///   never armed): reverted, so the next update can start.
    /// - Confirmed but the floor has not taken the SVN: held. The floor
    ///   advance is the update agent's to ask for, with
    ///   UpdateSecurityRevision once the platform is in service, so this
    ///   boot must not advance it. If the floor already reads at or above
    ///   the session's SVN the advance did land before the crash, and the
    ///   session is completed here: that is the one gap between advancing
    ///   the floor and closing the session.
    ///
    /// Must run before the machine can grant a new update: `prepare`
    /// overwrites any earlier session, so a new update recorded over a
    /// `Committed` one would drop the floor advance it still owes.
    pub fn resume_self_update(&mut self) -> Result<(), DriverError> {
        let session = &mut self.board.self_update;
        let state = session.state().map_err(|_| DriverError::SelfUpdateFault)?;
        let running = session
            .running()
            .map_err(|_| DriverError::SelfUpdateFault)?;
        match trial_outcome(state, running) {
            TrialOutcome::NoSession | TrialOutcome::InProgress => Ok(()),
            TrialOutcome::Unconfirmed => session.revert().map_err(|_| DriverError::SelfUpdateFault),
            TrialOutcome::ConfirmedUncommitted { svn } => {
                let floor = self
                    .board
                    .self_svn_floor
                    .floor()
                    .map_err(|_| DriverError::SvnFloorFault)?;
                if floor >= svn {
                    self.board
                        .self_update
                        .complete()
                        .map_err(|_| DriverError::SelfUpdateFault)?;
                }
                Ok(())
            }
        }
    }

    /// Advances the eRoT's own floor to the SVN its confirmed session
    /// recorded, then closes the session.
    ///
    /// This is the update agent's request arriving as
    /// UpdateSecurityRevision, which the FD reports as
    /// `SvnCommitPending`: the caller runs this and grants on `Ok`, or
    /// denies on `Err`. The floor never moves at activation, so a
    /// downgrade needs a confirmed trial boot first.
    ///
    /// Refuses unless the session is confirmed and uncommitted. A
    /// request with nothing behind it must not move the floor, because
    /// nothing has proven the image that SVN belongs to.
    ///
    /// Replay-safe, which is what makes the crash window harmless:
    /// `advance` is a no-op at or below the floor and `complete`
    /// succeeds from `Idle`, so a request repeated after a crash between
    /// the two lands in the same place.
    pub fn commit_self_svn_floor(&mut self) -> Result<(), DriverError> {
        let session = &mut self.board.self_update;
        let state = session.state().map_err(|_| DriverError::SelfUpdateFault)?;
        let running = session
            .running()
            .map_err(|_| DriverError::SelfUpdateFault)?;
        let TrialOutcome::ConfirmedUncommitted { svn } = trial_outcome(state, running) else {
            return Err(DriverError::NoSelfUpdateToCommit);
        };
        self.board
            .self_svn_floor
            .advance(svn)
            .map_err(|_| DriverError::SvnFloorFault)?;
        self.board
            .self_update
            .complete()
            .map_err(|_| DriverError::SelfUpdateFault)
    }

    /// One step of the in-flight update, called by the event loop between
    /// events, as [`poll_boot_walks`](Self::poll_boot_walks) is.
    ///
    /// Staging runs one bounded step per call, so the loop stays live
    /// through a transfer that takes minutes. A job that stops making
    /// progress for longer than the board's stall budget is abandoned
    /// here rather than waited out.
    ///
    /// Only `UpdateRejected` is emitted today (fault or stall).
    /// `UpdateVerified` arrives with the crypto verify-client; until
    /// then the pump parks at `Staged` and returns idle. The job stays
    /// until the SM answers with `ActivateUpdate` or `DiscardStaged`.
    ///
    /// A slot re-sync is the exception: it ends here with no event,
    /// because the SM never asked for it and a verdict would start a
    /// second activation.
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
            // or the device already holds the payload and the SM owns
            // the next move.
            UpdatePhase::Submitted | UpdatePhase::Staged => return UpdatePoll::idle(),
            UpdatePhase::Staging | UpdatePhase::Resyncing => self.poll_staging(),
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
                    progress: Some(progress),
                }
            }
            // A re-sync ends in the driver. Telling the SM the payload
            // is staged would start a second activation of an image the
            // device is already running.
            Step::Staged if phase == UpdatePhase::Resyncing => {
                self.pending_update = None;
                UpdatePoll::idle()
            }
            Step::Staged => {
                job.phase = UpdatePhase::Staged;
                // Fail closed: no UpdateVerified until the crypto
                // verify-client is wired. The pump parks here.
                UpdatePoll::idle()
            }
            Step::Authenticated => {
                // Unreachable until Authenticating is a real phase.
                UpdatePoll {
                    event: Some(Event::UpdateVerified(job.target)),
                    progress: None,
                }
            }
            // A failed re-sync leaves the running image committed and the
            // spare slot stale, which is a report rather than a verdict
            // the SM acts on.
            Step::Rejected if phase == UpdatePhase::Resyncing => self.end_resync(),
            Step::Rejected => self.reject_job(),
        }
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
    /// this job, so nothing else would clear it and the pump would
    /// retry the same failure forever. The running image is committed
    /// either way, so this is a report, not a verdict.
    fn end_resync(&mut self) -> UpdatePoll {
        let target = self.pending_update.as_ref().map(|job| job.target);
        self.abandon_job();
        self.pending_update = None;
        if let Some(target) = target {
            self.report(Report::SlotResyncFailed(target));
        }
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
        UpdatePoll {
            event: Some(Event::UpdateRejected),
            progress: None,
        }
    }

    /// Tells the device to drop what it was staging. Leaves the job
    /// itself alone: who clears it differs between an update, which the
    /// SM answers for, and a re-sync, which ends here.
    fn abandon_job(&mut self) {
        let Some(job) = self.pending_update.as_ref() else {
            return;
        };
        let target = job.target.get() as usize;
        if let Some(updatable) = self.board.updatables.get_mut(target) {
            updatable.abandon();
        }
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
        floor.advance(svn).map_err(|_| DriverError::SvnFloorFault)?;
        self.arm_slot_resync(id);
        Ok(())
    }

    /// Queues a second staging pass for the image just committed, so the
    /// slot the device stopped booting from stops holding the version
    /// before it. The pump runs it and nothing activates afterwards: the
    /// payload lands in the inactive slot and stays there.
    ///
    /// Re-stages rather than copying between slots, because `Updatable`
    /// keeps slot identity on the device's side. The candidate is still
    /// in the staging region: one job at a time, so nothing has
    /// overwritten it since the activation.
    ///
    /// Silent when there is nothing to re-sync (a confirmed boot with no
    /// update behind it) and when a job is already in flight, which a
    /// re-sync must never displace. A failure during the pass is a
    /// report, not an error: the running image is committed either way.
    fn arm_slot_resync(&mut self, id: ComponentId) {
        let Some((target, len)) = self.last_activated else {
            return;
        };
        if target != id || self.pending_update.is_some() {
            return;
        }
        self.last_activated = None;
        self.pending_update = Some(UpdateJob {
            target,
            len,
            phase: UpdatePhase::Resyncing,
            prepare_commanded: true,
            progress: Progress::start(len),
            progress_since_millis: None,
        });
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

    /// Hold `id` in reset, a durable quiesce, not a pulse; at-rest
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
    /// complete [`next_deadline_millis`](BootWalkPoll::next_deadline_millis),
    /// the earliest deadline among the still-waiting walks.
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

/// One [`PlatformDriver::pump_update`] round.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UpdatePoll {
    /// `UpdateRejected` on fault or stall. `UpdateVerified` is not
    /// emitted until the crypto verify-client is wired; the pump parks
    /// at Staged instead.
    pub event: Option<Event>,
    /// How far the job has come, for the update source's progress
    /// report. `None` once the job has ended, whichever way it ended.
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
    /// every executor error reports as [`EffectError`], the SM treats all
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
            Effect::AuthenticateStageUpdate => self.prepare_update().map(|_| None),
            Effect::ActivateUpdate => self.activate_update().map(|_| None),
            Effect::DiscardStaged => self.discard_staged().map(|_| None),
            // No board capability is composed for these seams yet, so they
            // fail closed here instead of behind stub methods.
            Effect::SignAttestation | Effect::LatchLockdown => return Err(EffectError),
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
/// [`Event::UpdateRequest`]; that order is load-bearing,
/// `AuthenticateStageUpdate` can never run without a target. On refusal no
/// event is injected and the frontend answers the requester over its own
/// protocol.
///
/// Exactly one answer per request. A supervised machine always produces
/// one: `Ready` runs the update, and the other supervised states report
/// it deferred. An unsupervised one (pre-service, or locked down) drops
/// what it does not handle, so the request is refused here rather than
/// recorded and forgotten. Refusing outside is what keeps `Locked` inert:
/// giving it an arm that emits a report would breach that.
pub fn request_update<B: BoardCapabilities, const N: usize, const E: usize>(
    orchestrator: &mut Orchestrator<N, E>,
    driver: &mut PlatformDriver<B, N>,
    target: ComponentId,
    len: u64,
) -> Result<(), DriverError> {
    if !orchestrator.state().is_supervised() {
        return Err(DriverError::Unsupervised);
    }
    driver.submit_update(target, len)?;
    orchestrator.dispatch(driver, Event::UpdateRequest);
    Ok(())
}
