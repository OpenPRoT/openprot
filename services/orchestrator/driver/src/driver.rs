// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`PlatformDriver`]: one executor method per [`Effect`] variant, routed from
//! the SM through the [`Platform`] impl.

use openprot_orchestrator_sm::{
    BootFailureKind, Chain, ComponentAttrs, ComponentId, ComponentKind, Effect, EffectError, Event,
    Orchestrator, Platform,
};

use orchestrator_config::ChainEntries;

use crate::board::{
    Board, BoardCapabilities, ImageSource, Report, ReportSink, SvnFloorBinding, Verdict, Verifier,
};
use orchestrator_capabilities::{
    BootControl, BootWatch, FailureCause, IncrementalVerifier, PollOutcome, Progress, Recovery,
    RestoreOutcome, RunningImage, SelfUpdate, SelfUpdateState, StageProgress, Svn, SvnFloor,
    TrialOutcome, Updatable, VerifySession, WalkVerdict,
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
    /// so the SVN to advance to is unknown; fail secure.
    NoVerifiedImage,
    /// The component's SVN floor could not be advanced.
    SvnFloorFault,
    /// An update is already in flight; the frontend answers the requester
    /// over its own protocol, the SM never sees the refused request.
    UpdateBusy,
    /// The device refused to activate what it staged.
    UpdateFault,
    /// The machine is pre-service or locked down, so it would drop the
    /// request without reporting it. Refused here instead.
    Unsupervised,
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
    /// Activation was asked before the candidate passed verification.
    /// The job survives, so `DiscardStaged` can still end it.
    CandidateNotAuthenticated,
    /// A staging step ran before anything commanded the work. Updates
    /// get the flag from `AuthenticateStageUpdate`, and a slot re-sync
    /// sets it when it queues the job. So the phase and the flag
    /// disagree.
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
            DriverError::Unsupervised => "the platform is pre-service or locked down",
            DriverError::RecoveryFault => "recovery mechanism faulted",
            DriverError::NoUpdateJob => "no update job for this effect",
            DriverError::CandidateOutOfRange => "candidate does not fit the staging region",
            DriverError::CandidateNotAuthenticated => "candidate has not passed verification yet",
            DriverError::UpdateNotCommanded => "staging ran before the SM commanded it",
        })
    }
}

impl core::error::Error for DriverError {}

/// Whether the update verifier is idle or mid-session. Move semantics
/// on the traits require this enum so the driver can hold either state.
enum VerifierState<V: IncrementalVerifier> {
    Idle(V),
    Verifying(V::Session),
}

/// The effect executors. Everything device-specific lives in the [`Board`];
/// the driver's own fields are bookkeeping.
pub struct PlatformDriver<B: BoardCapabilities, const N: usize> {
    board: Board<B, N>,
    /// `kinds[i]` classifies `ComponentId(i)`, derived from the chain the
    /// state machine runs on. A completed walk becomes `ComponentReady` for
    /// an `Active` component and `Booted` for a `Passive` one.
    kinds: [ComponentKind; N],
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
    /// Component and candidate length of the last activation. A commit
    /// re-stages that payload into the spare slot. Cleared once the
    /// re-sync is queued.
    last_activated: Option<(ComponentId, u64)>,
    /// A floor advance waiting for the spare slot to catch up. CSA 5.3.2
    /// wants both slots at the same SVN before the floor moves, so the
    /// commit holds the advance here and the pump applies it when the
    /// re-sync finishes.
    held_floor: Option<(ComponentId, Svn)>,
    /// The incremental verifier, idle or mid-session. Moved out of the
    /// board at construction so the pump can drive it without borrowing
    /// the whole board. `None` only transiently during a pump step.
    update_verifier: Option<VerifierState<B::UpdateVerifier>>,
}

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
    /// The device holds the complete payload. Whether verification has
    /// started is the verifier's to say, not a second flag here.
    Staged,
    /// The committed image is being written a second time, into the
    /// slot the device just stopped booting from. The SM does not know
    /// about this job. It ends in the driver.
    Resyncing,
    /// The verifier accepted the candidate; activation is next.
    Authenticated,
}

impl<B: BoardCapabilities, const N: usize> PlatformDriver<B, N> {
    /// Derives what the chain already states instead of taking it twice: the
    /// component kinds come from `entries`, the same entries the state machine
    /// is built from.
    ///
    /// # Panics
    ///
    /// Panics if an entry's id is not its position. The driver indexes every
    /// per-component array by `id.get()`, so an entry out of position would
    /// address the wrong component's reset line, flash and floor.
    ///
    /// Panics if the board's `update_verifier` is `None`. The driver takes
    /// it out of the board here and owns it from then on, so a board that
    /// leaves it empty could never verify an update.
    pub(crate) fn new(entries: &[(ComponentId, ComponentAttrs); N], board: Board<B, N>) -> Self {
        // ComponentId is a u8; Chain rejects more than u8::MAX entries.
        const { assert!(N <= u8::MAX as usize) };
        let mut kinds = [ComponentKind::Passive; N];
        let mut i = 0;
        while i < N {
            assert!(
                entries[i].0.get() as usize == i,
                "a chain entry's id must be its position in the chain"
            );
            kinds[i] = entries[i].1.kind;
            i += 1;
        }
        let mut board = board;
        let update_verifier = board
            .update_verifier
            .take()
            .expect("Board::update_verifier must be Some");
        Self {
            board,
            update_verifier: Some(VerifierState::Idle(update_verifier)),
            kinds,
            staged: None,
            watching: [false; N],
            verified_svn: [None; N],
            pending_update: None,
            last_activated: None,
            held_floor: None,
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

    /// Queue the spare slot's re-sync, then advance `id`'s anti-rollback
    /// floor to its verified image's SVN. A target at or below the
    /// current floor is the capability's documented no-op, so a replayed
    /// commit is harmless.
    ///
    /// A self-managed component gets neither. It keeps its own floor, and
    /// a device that owns its anti-rollback owns its slot metadata too,
    /// the same split `DeviceTrialBoot` draws. Writing into a slot the
    /// eRoT does not control would reach past that seam. This reads slot
    /// ownership off the floor binding, which is strictly a floor fact;
    /// a board that needs the two apart wants its own attribute.
    ///
    /// CSA 5.3.2 wants both slots at the same SVN before the floor moves,
    /// so when a re-sync is queued the advance waits for it. The pump
    /// writes the floor once the spare holds the image. A pass that
    /// fails leaves the floor alone: moving it would put the spare below
    /// the floor, where it will not boot.
    ///
    /// A floor the pump cannot write is reported, not escalated. The
    /// same failure on this path, with no re-sync queued, returns an
    /// error and the SM latches `Locked`. Past `BootConfirmed` the SM
    /// has no job to fail, so the two postures differ.
    pub fn commit_svn_floor(&mut self, id: ComponentId) -> Result<(), DriverError> {
        let idx = id.get() as usize;
        let SvnFloorBinding::Erot(_) = self
            .board
            .svn_floors
            .get(idx)
            .ok_or(DriverError::UnknownComponent)?
        else {
            // The component tracks its own SVN, so there is no floor
            // here to move and no slot of ours to bring up to date.
            return Ok(());
        };
        let svn = self.verified_svn[idx].ok_or(DriverError::NoVerifiedImage)?;
        if self.queue_slot_resync(id) {
            // The spare still holds the old image. Hold the advance
            // until the pass has put the new one there.
            self.held_floor = Some((id, svn));
            return Ok(());
        }
        if self.held_floor.is_some() {
            // A pass is already running with an advance held behind it,
            // so this commit arrived twice. Leave the held one to the
            // pump rather than writing the floor while the spare is
            // still being written.
            return Ok(());
        }
        self.advance_floor(id, svn)
    }

    /// Writes `svn` to `id`'s floor. Only reached for a component whose
    /// floor the eRoT holds.
    fn advance_floor(&mut self, id: ComponentId, svn: Svn) -> Result<(), DriverError> {
        let idx = id.get() as usize;
        // Cannot fire: both callers read the binding first and only get
        // an SVN for a component whose floor the eRoT holds. Kept so a
        // third caller cannot write a floor that is not ours.
        let Some(SvnFloorBinding::Erot(floor)) = self.board.svn_floors.get_mut(idx) else {
            return Ok(());
        };
        floor.advance(svn).map_err(|_| DriverError::SvnFloorFault)
    }

    /// Queues a second staging pass for the image just committed. The
    /// slot the device stopped booting from still holds the old
    /// firmware, and this pass overwrites it. The pump runs the
    /// transfer. Nothing activates afterwards, so the payload stays in
    /// the inactive slot.
    ///
    /// CSA 5.3.2 puts slot parity on the eRoT and its tooling. For a
    /// component whose floor the eRoT holds, that is this pass. A
    /// component that keeps its own SVN keeps its own slots as well, so
    /// the commit never gets this far.
    ///
    /// It re-stages instead of copying slot to slot, because `Updatable`
    /// keeps slot identity on the device's side. The candidate is still
    /// in the staging region, since the machine runs one job at a time.
    ///
    /// Does nothing when there is nothing to re-sync. That is a
    /// confirmed boot with no update behind it, or a staging region that
    /// changed hands since the activation; `submit_update` and
    /// `discard_staged` clear the claim when that happens. The in-flight
    /// check below cannot fire today, because those same two clear the
    /// claim before any job starts. It stays so a re-sync can never push
    /// a job aside. A failure during the pass is reported. The running
    /// image is committed either way.
    fn queue_slot_resync(&mut self, id: ComponentId) -> bool {
        let Some((target, len)) = self.last_activated else {
            return false;
        };
        if target != id || self.pending_update.is_some() {
            return false;
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
        true
    }

    /// `id`'s reset actuator.
    fn boot_control(&mut self, id: ComponentId) -> Result<&mut B::BootControl, DriverError> {
        self.board
            .boot_controls
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)
    }

    /// Release `id` from reset and start its boot walk;
    /// [`poll_boot_walks`](Self::poll_boot_walks) feeds the verdict back
    /// as `ComponentReady(id)`/`Booted(id)`/`BootFailed { id, .. }`. Starts on every
    /// release: a retry re-release starts a fresh walk.
    pub fn release_reset(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.boot_control(id)?
            .release()
            .map_err(|_| DriverError::BootControlFault)?;
        let idx = id.get() as usize;
        // In bounds: boot_control(id) above already rejected unknown ids.
        self.board.boot_watches[idx].start();
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
                    let event = match self.kinds[idx] {
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
    /// the mechanism is a genuine actuation fault that fails secure.
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
    /// off the fail-secure path; reports arrive in the order the SM emitted
    /// them.
    pub fn report(&mut self, report: Report) {
        self.board.report_sink.report(report);
    }
}

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
    /// A session is still being judged, either a trial run waiting on the
    /// update agent or a confirmed one whose floor is still below `svn`. No
    /// new self-update may start: recording one would overwrite the session.
    AwaitingUpdateAgent {
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
    /// actuation failures the same, fail-secure.
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
        match effect {
            Effect::ReadFirmware(id) => self.stage_firmware(id).map(|_| None),
            Effect::VerifyFirmware(id) => self.verify_firmware(id).map(Some),
            Effect::ReleaseReset(id) => self.release_reset(id).map(|_| None),
            Effect::AssertReset(id) => self.assert_reset(id).map(|_| None),
            Effect::CommitSvnFloor(id) => self.commit_svn_floor(id).map(|_| None),
            // Reports carry no error, so they never reach the fail-secure
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
            // fail secure here instead of behind stub methods.
            Effect::SignAttestation | Effect::LatchLockdown => return Err(EffectError),
            // Emit is consumed by the orchestrator; receiving one is a
            // driver bug.
            Effect::Emit(_) => return Err(EffectError),
        }
        .map_err(|_| EffectError)
    }
}

/// Brings a platform up from one chain: the state machine that decides and the
/// driver that acts, built from the same entries so neither can be holding a
/// different list of components than the other.
///
/// Takes a [`ChainEntries`] returned by `orchestrator_config::chain_of`,
/// which validates the table at const time. Boards declare the result as a
/// `const` item, so an invalid table is a build error, not a runtime panic.
pub fn bring_up<B: BoardCapabilities, const N: usize, const E: usize>(
    chain_entries: &'static ChainEntries<N>,
    board: Board<B, N>,
    max_retry: u8,
) -> (Orchestrator<N, E>, PlatformDriver<B, N>) {
    let entries = chain_entries.entries();
    // chain_of validated: nonempty, at most u8::MAX, ids are positions,
    // deps strictly earlier. Chain::try_from rechecks the same invariants
    // at runtime, so the expect cannot fire.
    let chain: Chain<N> = heapless::Vec::from_slice(entries)
        .expect("same length as the capacity")
        .try_into()
        .expect("chain_of validated the entries");
    (
        Orchestrator::new(chain, max_retry),
        PlatformDriver::new(entries, board),
    )
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
///   run. Nothing here judges it. The update agent does, with
///   UpdateSecurityRevision, so the session is left alone.
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
            // Nothing here judges the trial. The update agent does.
            let SelfUpdateState::TrialPending { svn } = state else {
                // trial_outcome answers InProgress for TrialPending alone.
                // Fail secure rather than guess which SVN was recorded.
                return Err(SettleError::InconsistentSession(state));
            };
            Ok(SelfUpdateSettlement::AwaitingUpdateAgent { svn })
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
            Ok(SelfUpdateSettlement::AwaitingUpdateAgent { svn })
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

/// The connection between an update frontend and the SM: called (by the
/// event loop, on the frontend's behalf) once a complete candidate for
/// `target` sits in the staging region. Records the job first, then injects
/// [`Event::UpdateRequest`]; that order is load-bearing,
/// `AuthenticateStageUpdate` can never run without a target. On refusal no
/// event is injected and the frontend answers the requester over its own
/// protocol.
///
/// Every request gets one answer. `Ready` runs the update and the other
/// supervised states report it deferred, but an unsupervised machine drops
/// what it does not handle, so the request is refused here instead. The
/// check sits outside the state machine because giving `Locked` an arm that
/// emits a report would stop it being inert.
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
    orchestrator.dispatch(driver, Event::UpdateRequest(target));
    Ok(())
}
