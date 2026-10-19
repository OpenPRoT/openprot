// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`PlatformDriver`]: one executor method per [`Effect`] variant, routed from
//! the SM through the [`Platform`] impl.

use openprot_orchestrator_sm::{
    BootFailureKind, Chain, ComponentAttrs, ComponentId, ComponentKind, Effect, EffectError, Event,
    Orchestrator, Platform,
};

use orchestrator_config::{ChainEntries, DeviceConfig};

use crate::board::{
    Board, BoardCapabilities, ComponentWiring, ImageSource, Report, ReportSink, SvnFloorBinding,
    Verdict, Verifier,
};
use orchestrator_capabilities::{
    AuthenticatedImage, BootControl, BootWatch, FailureCause, IncrementalVerifier, Progress,
    Recovery, RestoreOutcome, RunningImage, SelfUpdate, SelfUpdateState, Svn, SvnFloor,
    TrialOutcome, WalkVerdict,
};

/// Why the driver could not carry out an effect.
///
/// Effects are what the state machine tells the driver to do. The event
/// loop turns these errors into `Event::EffectFailed`, which sends the
/// machine to `Locked`, so any error here locks the platform. A refusal
/// that should not lock belongs earlier, where the request arrives, before
/// the state machine emits an effect at all.
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
    /// A self-update effect ran on a board that left `Board::self_image`
    /// `None`. The event that opens the trial is read from that field, so
    /// the board and the machine have drifted apart.
    NoSelfImage,
    /// The eRoT's own update session could not be written.
    SelfUpdateFault,
    /// The eRoT's own candidate reached the activation with no manifest
    /// SVN, so the session has no SVN to record.
    ///
    /// The name says `SelfActivate`: what failed is the activation, while
    /// the image is versionless rather than faulty.
    SelfActivateNoSvn,
    /// The reset that boots the eRoT's trial image did not happen. The
    /// session is already marked pending, so the next reset from any
    /// source still boots the trial. The board's `SelfReset::Error` says
    /// why the reset failed; the conversion to this variant discards that
    /// error, so no caller and no report ever sees it.
    SelfResetFault,
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
            DriverError::NoSelfImage => "board wired no self-image",
            DriverError::SelfUpdateFault => "self-update session could not be written",
            DriverError::SelfActivateNoSvn => {
                "the eRoT's own candidate reached the activation with no manifest SVN"
            }
            DriverError::SelfResetFault => "the reset into the trial image did not happen",
            DriverError::UpdateNotCommanded => "staging ran before the SM commanded it",
        })
    }
}

impl core::error::Error for DriverError {}

mod update;

pub use update::UpdatePoll;

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
    /// `verified_svn[i]` is what the last verification of `ComponentId(i)`
    /// recorded, the only reading a floor commit may trust. Reset to
    /// [`VerifiedSvn::Unverified`] on rejection.
    verified_svn: [VerifiedSvn; N],
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
    /// The verifier accepted the candidate; activation is next. Carries
    /// the `AuthenticatedImage`, so the candidate's SVN cannot go missing
    /// between the verdict and the activation that records it.
    Authenticated(AuthenticatedImage),
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
            verified_svn: [VerifiedSvn::Unverified; N],
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

    /// `id`'s image source. Takes the array rather than `&mut self` so the
    /// caller can borrow `board.verifier` alongside the returned image.
    fn source(
        components: &mut [ComponentWiring<B>; N],
        id: ComponentId,
    ) -> Result<&mut B::Image, DriverError> {
        components
            .get_mut(id.get() as usize)
            .map(|component| &mut component.image)
            .ok_or(DriverError::UnknownComponent)
    }

    /// Stage `id`'s image: open its source so
    /// [`verify_firmware`](Self::verify_firmware) can read it.
    pub fn stage_firmware(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.staged = None;
        let source = Self::source(&mut self.board.components, id)?;
        source.open().map_err(|_| DriverError::ImageUnavailable)?;
        self.staged = Some(id);
        Ok(())
    }

    /// Judge the staged image via the [`Verifier`] and return the verdict:
    /// `Event::VerificationPassed(id)` or `Event::VerificationFailed(id)`.
    pub fn verify_firmware(&mut self, id: ComponentId) -> Result<Event, DriverError> {
        // Id validity first: an unknown component is UnknownComponent even
        // though it can never be staged.
        let source = Self::source(&mut self.board.components, id)?;
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
            Verdict::Authenticated(image) => {
                self.verified_svn[idx] = match image.svn {
                    Some(svn) => VerifiedSvn::Svn(svn),
                    None => VerifiedSvn::Versionless,
                };
                if let Some(measurement) = image.measurement {
                    self.report(Report::Measured { id, measurement });
                }
                Event::VerificationPassed(id)
            }
            Verdict::Rejected => {
                self.verified_svn[idx] = VerifiedSvn::Unverified;
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
            .components
            .get(idx)
            .map(|component| &component.svn_floor)
            .ok_or(DriverError::UnknownComponent)?
        else {
            // The component tracks its own SVN, so there is no floor
            // here to move and no slot of ours to bring up to date.
            return Ok(());
        };
        let svn = match self.verified_svn[idx] {
            VerifiedSvn::Svn(svn) => svn,
            // The image authenticated and carried no version to move the
            // floor to. Nothing is wrong with the image, so the commit
            // succeeds and the floor stays where it is; the board asked
            // for anti-rollback it cannot get, and the report is what
            // says so. Refusing here would turn a legal image into a
            // lockdown. The spare still gets the new image: the re-sync
            // keeps the two slots holding the same firmware, which is
            // wanted whether or not a floor moves behind it.
            VerifiedSvn::Versionless => {
                self.queue_slot_resync(id);
                self.report(Report::SvnFloorHeld(id));
                return Ok(());
            }
            // No verdict recorded at all. The commit follows a walk that
            // verifies, so this means the driver and the machine have
            // drifted apart, and it fails secure.
            VerifiedSvn::Unverified => return Err(DriverError::NoVerifiedImage),
        };
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
        let Some(SvnFloorBinding::Erot(floor)) = self
            .board
            .components
            .get_mut(idx)
            .map(|component| &mut component.svn_floor)
        else {
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

    /// Whether `id` is the eRoT: the platform acting on itself rather
    /// than on a device it manages. The chain gives every component a
    /// kind, `Erot`, `Active` or `Passive`, and `chain_of` admits exactly
    /// one `Erot`, so exactly one id answers true.
    fn is_self(&self, id: ComponentId) -> bool {
        self.kinds.get(id.get() as usize) == Some(&ComponentKind::Erot)
    }

    /// `id`'s reset actuator.
    fn boot_control(&mut self, id: ComponentId) -> Result<&mut B::BootControl, DriverError> {
        self.board
            .components
            .get_mut(id.get() as usize)
            .map(|component| &mut component.boot_control)
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
        self.board.components[idx].boot_watch.start();
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
            match self.board.components[idx].boot_watch.poll(now_millis) {
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
                        // The eRoT is never released, so it is never watched
                        // and no walk finishes for it. Keep polling. The
                        // deadline so far covers only the components before
                        // this one, so returning here would give the caller
                        // too long a wait.
                        ComponentKind::Erot => continue,
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
            .components
            .get_mut(id.get() as usize)
            .map(|component| &mut component.recovery)
            .ok_or(DriverError::UnknownComponent)?;
        match recovery.restore(attempt) {
            Ok(RestoreOutcome::Restored) => Ok(Event::Restored(id)),
            Ok(RestoreOutcome::SourceExhausted) => Ok(Event::RecoveryUnavailable(id)),
            Err(_) => Err(DriverError::RecoveryFault),
        }
    }

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
    fn confirm_self_trial(&mut self) -> Result<(), DriverError> {
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

    /// Hands one report to the board's sink. Cannot fail, so reporting stays
    /// off the fail-secure path; reports arrive in the order the SM emitted
    /// them.
    pub fn report(&mut self, report: Report) {
        self.board.report_sink.report(report);
    }
}

/// The error type of the board's self-update session, spelled once so
/// [`PlatformDriver::settle_self_image`] fits on a line.
type SelfUpdateError<B> = <<B as BoardCapabilities>::SelfUpdate as SelfUpdate>::Error;

/// The error type of the board's anti-rollback floor, for the same reason.
type SvnFloorError<B> = <<B as BoardCapabilities>::SvnFloor as SvnFloor>::Error;

/// What the last verification of one component recorded about its image.
///
/// Three states, not an `Option<Svn>`, because "no verdict yet" and
/// "authenticated with no version in its manifest" cost different things
/// at commit time: the first is drift between the driver and the machine,
/// the second is an ordinary image on a board that cannot do anti-rollback
/// for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VerifiedSvn {
    /// No verification has passed, or the last one failed.
    Unverified,
    /// The image authenticated and its manifest carried no SVN.
    Versionless,
    /// The SVN the authenticated manifest carried.
    Svn(Svn),
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
            Effect::ConfirmSelfTrial => self.confirm_self_trial().map(|_| None),
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
///
/// `chain_entries` and `devices` are expected to come from the same table.
/// Nothing checks that, so one board's chain passed with another's table
/// would describe every component wrongly.
///
/// # Panics
///
/// Panics if a component's wiring does not name its own row of `devices`.
/// The wiring holds hardware handles, so it cannot be checked at const time
/// the way `chain_of` checks the table; the assert catches a miswired board
/// at first bring-up, before it drives another component's reset line.
///
/// The table has to be a `static` item. A `const` table is promoted afresh
/// at each use, so the row a wiring names and the row here would be two
/// addresses and the check could not hold on a correctly wired board.
pub fn bring_up<B: BoardCapabilities, const N: usize, const E: usize>(
    chain_entries: &'static ChainEntries<N>,
    devices: &'static [DeviceConfig<B::ResetSignal, B::Probe>; N],
    board: Board<B, N>,
    max_retry: u8,
) -> (Orchestrator<N, E>, PlatformDriver<B, N>) {
    for (component, device) in board.components.iter().zip(devices.iter()) {
        assert!(
            core::ptr::eq(component.device, device),
            "a component's wiring must name its own device table entry, \
             and the table has to be a static item"
        );
    }
    let entries = chain_entries.entries();
    // chain_of validated: nonempty, at most u8::MAX, ids are positions,
    // deps strictly earlier, eRoT at position 0 and nowhere else.
    // Chain::try_from rechecks all of that except the eRoT rule, so the
    // expect cannot fire. Only chain_of knows that rule, which is what lets
    // the unit tests and the QEMU runtime test build a bare chain through
    // Chain::try_from that starts with any kind.
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
