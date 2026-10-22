// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The RoT side of the mock-downstream-device integration test, running the shipped
//! orchestrator rather than a hand-rolled sequence.
//!
//! `bring_up` builds the state machine and the platform driver from one
//! device table, so the chain the SM decides over and the arrays the driver
//! indexes cannot disagree. What this file supplies is the board: a reset
//! line that reaches the mock downstream device over IPC, a `CheckpointWalk` over a
//! latched evidence reader, and stubs for the capabilities a boot scenario
//! never touches.
//!
//! Scenario: power on, verify, release the device from reset, and wait for
//! it to report ready inside its checkpoint window. The run passes when the
//! machine reaches `Ready`.
//!
//! Evidence arrives as a request on `boot_evt` and is latched in a static.
//! The orchestrator never asks the device whether it is up, because
//! `EvidenceReader::read` is synchronous and may not block; reading a latch
//! is the only shape that satisfies that. The reset path clears the latch,
//! never the reader, so evidence from a previous attempt cannot satisfy the
//! next one.

#![no_main]
#![no_std]

use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use board_stubs::{BoardFault, StubImage};
// The shared stub authenticates everything and measures nothing. The
// `measured` and recovery cfgs wire a verifier of their own.
#[cfg(not(any(measured, recovery, recovery_exhausted)))]
use board_stubs::StubVerifier;
use openprot_orchestrator_driver::{
    bring_up, settle_self_update_at_boot, Board, BoardCapabilities, ComponentWiring, ErotUpdate,
    Report, ReportSink, SelfUpdateSettlement, SvnFloorBinding,
};
#[cfg(any(measured, recovery, recovery_exhausted))]
use openprot_orchestrator_driver::{ImageSource, Verdict, Verifier};
// Only the self-update build submits a job of its own; every other
// scenario here is a boot.
#[cfg(self_update)]
use openprot_orchestrator_driver::request_update;
#[cfg(any(measured, recovery, recovery_exhausted, self_update))]
use openprot_orchestrator_sm::ComponentId;
use openprot_orchestrator_sm::{ComponentAttrs, Event, PowerOnResult, State};
#[cfg(any(measured, recovery, recovery_exhausted, self_update))]
use orchestrator_capabilities::AuthenticatedImage;
#[cfg(measured)]
use orchestrator_capabilities::Measurement;
use orchestrator_capabilities::{
    BootControl, BootStatus, EvidenceReader, IncrementalVerifier, PollOutcome, RunningImage,
    SelfUpdate, SelfUpdateState, StageProgress, Svn, SvnFloor, Updatable, UpdateError,
    VerifySession,
};
#[cfg(self_update)]
use orchestrator_capabilities::{Progress, SelfReset};
// Only the recovery cfgs wire a restore source.
#[cfg(any(recovery, recovery_exhausted))]
use orchestrator_capabilities::{Recovery, RestoreOutcome};
use orchestrator_checkpoint_walk::CheckpointWalk;
use orchestrator_config::{
    assert_retry_reaches_every_image, chain_of, BootCheckpoint, ChainEntries, DeviceConfig,
};
use util_io::{ByteReadError, ByteSource};

use board_clock::{deadline_in, millis_to_ticks, now_millis};
use userspace::syscall::Signals;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use app_orchestrator::handle;

/// Chain capacity and the core's effect-sink cap (`E >= 2*N + 2`). `N`
/// counts the device table below, so the two cannot disagree.
const N: usize = DEVICES.len();
const E: usize = 2 * N + 2;
const MAX_RETRY: u8 = 3;

/// How long the device has to report ready. It boots in 50 ms, so this is
/// slack rather than a race.
const BOOT_WINDOW_MILLIS: u64 = 2_000;

/// How long the whole run may take before the test gives up. Larger than the
/// boot window so a walk timeout is reported by the orchestrator rather than
/// cut short here.
const RUN_BUDGET_MILLIS: u64 = 10_000;

/// Whether this build passes on a platform lock: the device never reports
/// (`device_hangs`), or its image is refused with no source left
/// (`recovery_exhausted`). Inverting the verdict here, rather than letting
/// the target fail, keeps a build error and a caught failure from looking
/// the same to the runner.
const EXPECT_LOCKDOWN: bool = cfg!(device_hangs) || cfg!(recovery_exhausted);

/// Whether this build starts with an image the RoT refuses. `recovery`
/// restores a good image and boots. `recovery_exhausted` restores an image
/// the verifier still refuses, then runs out of sources.
const EXPECT_RECOVERY: bool = cfg!(recovery) || cfg!(recovery_exhausted);

/// Whether the expected lockdown happens inside the `PowerGood` dispatch.
/// The verification, both restore calls and the gating are follow-ups of
/// `PowerGood`, so the state machine is already `Locked` when that
/// dispatch returns.
const LOCKS_AT_POWER_ON: bool = cfg!(recovery_exhausted);

/// How many times the run expects `restore` to be called. `recovery`
/// needs one. `recovery_exhausted` needs two: the first writes an image
/// the verifier still refuses, the second answers `SourceExhausted`.
const EXPECT_RESTORE_CALLS: u8 = if cfg!(recovery_exhausted) {
    2
} else if cfg!(recovery) {
    1
} else {
    0
};

/// Whether this build boots the trial image of the eRoT's own update.
///
/// The variant scenario starts the session at `TrialPending` and runs the
/// same boot. What it adds is the claim that bringing the chain up confirms
/// the session, so the Update Agent can advance the SVN floor afterwards.
#[cfg(not(self_trial))]
const EXPECT_SELF_TRIAL: bool = false;
#[cfg(self_trial)]
const EXPECT_SELF_TRIAL: bool = true;

/// The SVN the trial image carries, as the session recorded it before the
/// eRoT reset into it.
const TRIAL_SVN: Svn = Svn(7);

/// The SVN the candidate carries. Above the floor, which starts at zero,
/// and apart from `TRIAL_SVN` so the two self scenarios cannot be confused
/// for one another in a log.
#[cfg(self_update)]
const CANDIDATE_SVN: Svn = Svn(9);

/// The candidate's bytes. A real one arrives over PLDM, but nothing serves
/// the eRoT's own update yet, so this scenario is its own update source.
#[cfg(self_update)]
const CANDIDATE: [u8; 256] = [0xa5; 256];

/// How much of the candidate one `poll_stage` takes. Several steps, so the
/// pump is driven the way a real transfer drives it rather than completing
/// in one call.
#[cfg(self_update)]
const STAGE_CHUNK: usize = 64;

/// What settling the eRoT's own session has to answer in this build. A
/// self-trial build is mid-trial; any other boot has nothing in flight.
const EXPECT_SETTLEMENT: SelfUpdateSettlement = if EXPECT_SELF_TRIAL {
    SelfUpdateSettlement::TrialRunning { svn: TRIAL_SVN }
} else {
    SelfUpdateSettlement::Settled
};

/// Reset command bytes, read by the device as its reset line.
const RESET_ASSERT: u8 = 0;
const RESET_RELEASE: u8 = 1;

/// The device table. One checkpoint: the device either reports ready inside
/// the window or it does not. Probe 1 is what the evidence reader answers
/// `Booted` for once the latch is set.
/// Chain position 0 is the eRoT itself. It is never released, walked or
/// recovered. It is in the table so the orchestrator can address it, not so
/// it can supervise it. A boot scenario never updates the eRoT either, so
/// the entry carries no layout.
static DEVICES: [DeviceConfig<u8, u8>; 2] = [
    DeviceConfig::erot("erot", None),
    DeviceConfig::new(
        "downstream",
        0,
        &[BootCheckpoint::new(
            "ready",
            1,
            core::time::Duration::from_millis(BOOT_WINDOW_MILLIS),
        )],
        // A boot walk never looks at images, so no layout is legal here.
        None,
        ComponentAttrs::passive_required(),
    ),
];

static CHAIN: ChainEntries<N> = chain_of(&DEVICES);

const _: () = assert_retry_reaches_every_image(MAX_RETRY, &DEVICES);

/// The device's ready line, as the RoT sees it.
///
/// Set when the mock downstream device reports in, cleared by the reset path. A static
/// because the reader lives inside the walk, inside the board, inside the
/// driver, while the run loop that receives the report is outside all three.
static DEVICE_READY: AtomicBool = AtomicBool::new(false);

/// Drives the device's reset line. An undeliverable command is a wiring
/// failure, not a boot failure, and surfaces as an effect error.
fn drive_reset(level: u8) -> Result<(), BoardFault> {
    let mut ack = [0u8; 1];
    syscall::channel_transact(handle::RESET_CMD, &[level], &mut ack, deadline_in(1_000))
        .map(|_| ())
        .map_err(|_| BoardFault)
}

/// The device's reset line. Both directions clear the ready latch: the
/// device is restarting either way, so anything it reported before this
/// belongs to a previous attempt.
struct BmcReset;

impl BootControl for BmcReset {
    type Error = BoardFault;

    fn hold_in_reset(&mut self) -> Result<(), BoardFault> {
        DEVICE_READY.store(false, Ordering::Relaxed);
        drive_reset(RESET_ASSERT)
    }

    fn release(&mut self) -> Result<(), BoardFault> {
        DEVICE_READY.store(false, Ordering::Relaxed);
        drive_reset(RESET_RELEASE)
    }
}

/// Reads the ready latch. Probe 1 is the only checkpoint, so any probe the
/// walk asks about is answered from the same latch.
struct BmcEvidence;

impl EvidenceReader<u8> for BmcEvidence {
    type Error = Infallible;

    fn read(&mut self, _probe: &u8) -> Result<BootStatus, Infallible> {
        Ok(if DEVICE_READY.load(Ordering::Relaxed) {
            BootStatus::Booted
        } else {
            BootStatus::Booting
        })
    }
}

/// Whether the RoT accepts the device's image. Starts false; a restore
/// that writes a good image flips it.
#[cfg(any(recovery, recovery_exhausted))]
static IMAGE_GOOD: AtomicBool = AtomicBool::new(false);

/// Replaces the shared stub, which authenticates whatever it is given.
/// Reads `IMAGE_GOOD`, so the restore is what makes the image pass. The
/// walk skips the eRoT, so only the managed device is verified.
#[cfg(any(recovery, recovery_exhausted))]
struct LatchedVerifier;

#[cfg(any(recovery, recovery_exhausted))]
impl Verifier for LatchedVerifier {
    type Error = BoardFault;

    fn verify(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<Verdict, BoardFault> {
        if !IMAGE_GOOD.load(Ordering::Relaxed) {
            return Ok(Verdict::Rejected);
        }
        Ok(Verdict::Authenticated(AuthenticatedImage {
            // Like the shared stub: no manifest for an SVN, no engine to
            // measure with.
            svn: None,
            measurement: None,
        }))
    }
}

/// The verifier this scenario wires: the shared stub, unless the build
/// needs a verifier that refuses (the recovery cfgs) or one that measures
/// (`measured`).
#[cfg(not(any(measured, recovery, recovery_exhausted)))]
type ScenarioVerifier = StubVerifier;
#[cfg(any(recovery, recovery_exhausted))]
type ScenarioVerifier = LatchedVerifier;
#[cfg(measured)]
type ScenarioVerifier = MeasuringVerifier;

/// The `ScenarioVerifier` the board is built with.
#[cfg(not(any(measured, recovery, recovery_exhausted)))]
const SCENARIO_VERIFIER: ScenarioVerifier = StubVerifier;
#[cfg(any(recovery, recovery_exhausted))]
const SCENARIO_VERIFIER: ScenarioVerifier = LatchedVerifier;
#[cfg(measured)]
const SCENARIO_VERIFIER: ScenarioVerifier = MeasuringVerifier;

/// How many times `restore` has been called. The call that answers
/// `SourceExhausted` writes nothing and still counts.
static RESTORE_CALLS: AtomicU8 = AtomicU8::new(0);

/// Only the recovery cfgs wire a restore source. The rest keep `()`,
/// which answers `SourceExhausted` at once, so `device_hangs` still locks
/// instead of restoring.
#[cfg(not(any(recovery, recovery_exhausted)))]
type DeviceRecovery = ();
#[cfg(any(recovery, recovery_exhausted))]
type DeviceRecovery = RecoverySource;

/// The `DeviceRecovery` each component is built with.
#[cfg(not(any(recovery, recovery_exhausted)))]
const DEVICE_RECOVERY: DeviceRecovery = ();
#[cfg(any(recovery, recovery_exhausted))]
const DEVICE_RECOVERY: DeviceRecovery = RecoverySource;

/// The device's one recovery source: it writes an image over the active
/// image, and a second attempt answers `SourceExhausted`. The re-walk judges
/// the image this writes, so the `recovery_exhausted` cfg writes an image
/// the verifier refuses.
#[cfg(any(recovery, recovery_exhausted))]
struct RecoverySource;

#[cfg(any(recovery, recovery_exhausted))]
impl Recovery for RecoverySource {
    type Error = BoardFault;

    fn restore(&mut self, attempt: u8) -> Result<RestoreOutcome, BoardFault> {
        RESTORE_CALLS.fetch_add(1, Ordering::Relaxed);
        if attempt > 0 {
            pw_log::info!("no source left to restore the device from");
            return Ok(RestoreOutcome::SourceExhausted);
        }
        pw_log::info!("restoring the device's image from its recovery source");
        IMAGE_GOOD.store(cfg!(recovery), Ordering::Relaxed);
        Ok(RestoreOutcome::Restored)
    }
}

/// The managed device keeps its own floor, so the eRoT holds none for it.
/// The eRoT's own floor is a different thing and is held here, in RAM: this
/// scenario never resets the eRoT, so nothing has to survive a reset.
struct ErotFloor;

/// The eRoT's own anti-rollback floor. Starts at zero, which is below
/// `TRIAL_SVN`, so a confirmed trial still has the advance ahead of it.
static EROT_FLOOR: AtomicU32 = AtomicU32::new(0);

impl SvnFloor for ErotFloor {
    type Error = BoardFault;

    fn floor(&self) -> Result<Svn, BoardFault> {
        Ok(Svn(EROT_FLOOR.load(Ordering::Relaxed)))
    }

    fn advance(&mut self, to: Svn) -> Result<(), BoardFault> {
        EROT_FLOOR.fetch_max(to.0, Ordering::Relaxed);
        Ok(())
    }
}

/// The eRoT's own update session.
///
/// A real one lives in flash and survives the reset that decides the trial.
/// This one is two atomics, because the scenario never resets the eRoT: the
/// session is set up as the boot that follows an activation would find it,
/// and the claim is what the boot does about it.
///
/// The state lives in statics rather than in the struct so the run can read
/// it back. The driver takes the board at `bring_up` and hands out no
/// reference to it.
struct SelfSession;

/// The session's state, as `STATE_*` below.
static SESSION_STATE: AtomicU8 = AtomicU8::new(0);
/// The SVN the session recorded, read only when the state carries one.
static SESSION_SVN: AtomicU32 = AtomicU32::new(0);

const STATE_IDLE: u8 = 0;
const STATE_PREPARED: u8 = 1;
const STATE_TRIAL_PENDING: u8 = 2;
const STATE_COMMITTED: u8 = 3;

impl SelfSession {
    /// Sets the session up as the boot after an activation would find it.
    /// An ordinary boot leaves it `Idle`.
    fn install() {
        if EXPECT_SELF_TRIAL {
            SESSION_SVN.store(TRIAL_SVN.0, Ordering::Relaxed);
            SESSION_STATE.store(STATE_TRIAL_PENDING, Ordering::Relaxed);
        }
    }

    fn read() -> SelfUpdateState {
        let svn = Svn(SESSION_SVN.load(Ordering::Relaxed));
        match SESSION_STATE.load(Ordering::Relaxed) {
            STATE_IDLE => SelfUpdateState::Idle,
            STATE_PREPARED => SelfUpdateState::Prepared { svn },
            STATE_TRIAL_PENDING => SelfUpdateState::TrialPending { svn },
            STATE_COMMITTED => SelfUpdateState::Committed { svn },
            // Nothing outside this file writes SESSION_STATE. Reading an
            // unknown value back as `Idle` would make the write bug look
            // like a boot with no self-update in flight.
            _ => unreachable!("the session holds a state nothing wrote"),
        }
    }
}

impl SelfUpdate for SelfSession {
    type Error = BoardFault;

    fn state(&self) -> Result<SelfUpdateState, BoardFault> {
        Ok(Self::read())
    }

    fn running(&self) -> Result<RunningImage, BoardFault> {
        // Which image this boot runs is the boot selector's answer, not the
        // session's. Here it is the scenario's: a self-trial build is the
        // boot that runs the trial image.
        Ok(if EXPECT_SELF_TRIAL {
            RunningImage::Trial
        } else {
            RunningImage::Confirmed
        })
    }

    fn prepare(&mut self, svn: Svn) -> Result<(), BoardFault> {
        SESSION_SVN.store(svn.0, Ordering::Relaxed);
        SESSION_STATE.store(STATE_PREPARED, Ordering::Relaxed);
        Ok(())
    }

    fn set_trial_pending(&mut self) -> Result<(), BoardFault> {
        match Self::read() {
            // Marking a pending trial again is a retry, not a fault.
            SelfUpdateState::Prepared { .. } | SelfUpdateState::TrialPending { .. } => {
                SESSION_STATE.store(STATE_TRIAL_PENDING, Ordering::Relaxed);
                Ok(())
            }
            _ => Err(BoardFault),
        }
    }

    fn confirm(&mut self) -> Result<(), BoardFault> {
        match Self::read() {
            // Confirming a confirmed session is the same retry.
            SelfUpdateState::TrialPending { .. } | SelfUpdateState::Committed { .. } => {
                SESSION_STATE.store(STATE_COMMITTED, Ordering::Relaxed);
                Ok(())
            }
            _ => Err(BoardFault),
        }
    }

    fn complete(&mut self) -> Result<(), BoardFault> {
        match Self::read() {
            // Only a committed session closes, and closing a closed one is
            // a retry. A session still being judged is dropped with
            // `revert`, so completing it here would discard a pending
            // trial.
            SelfUpdateState::Committed { .. } | SelfUpdateState::Idle => {
                SESSION_STATE.store(STATE_IDLE, Ordering::Relaxed);
                Ok(())
            }
            _ => Err(BoardFault),
        }
    }

    fn revert(&mut self) -> Result<(), BoardFault> {
        SESSION_STATE.store(STATE_IDLE, Ordering::Relaxed);
        Ok(())
    }
}

/// No update path in a boot scenario. Every step errors, which is what the
/// board contract asks of a device that cannot be updated.
#[cfg(not(self_update))]
struct NoUpdate;

#[cfg(not(self_update))]
impl Updatable for NoUpdate {
    fn poll_stage(&mut self, _payload: &dyn ByteSource) -> Result<StageProgress, UpdateError> {
        Err(UpdateError::Device)
    }

    fn abandon(&mut self) {}

    fn activate(&mut self) -> Result<(), UpdateError> {
        Err(UpdateError::Device)
    }
}

/// No update is verified in a boot scenario, so being polled at all is a
/// wiring mistake rather than a verdict. It reports a fault, which the pump
/// turns into a rejection, instead of accepting an image nothing checked.
#[cfg(not(self_update))]
struct NeverVerifier;

#[cfg(not(self_update))]
struct NeverSession;

#[cfg(not(self_update))]
impl IncrementalVerifier for NeverVerifier {
    type Error = BoardFault;
    type Session = NeverSession;

    fn start(self) -> NeverSession {
        NeverSession
    }
}

#[cfg(not(self_update))]
impl VerifySession for NeverSession {
    type Verifier = NeverVerifier;
    type Error = BoardFault;

    fn poll(self, _payload: &dyn ByteSource) -> PollOutcome<Self> {
        PollOutcome::Fault(NeverVerifier, BoardFault)
    }

    fn abandon(self) -> NeverVerifier {
        NeverVerifier
    }
}

/// The eRoT's own slot, as the pump writes into it.
///
/// The slot writes nothing to flash: QEMU models one chip, so the image the
/// eRoT would boot is the one running. What the scenario needs from this
/// seam is that the pump ran to the end, so it counts the bytes it was
/// handed and reports `Ready` on the last one.
#[cfg(self_update)]
struct SelfSlot {
    written: u64,
}

/// Bytes the slot was handed, read at the end of the run. The slot itself
/// is inside the board, which the driver takes and hands no reference to.
/// A `u32` because this target has no 64-bit atomics, and
/// `CANDIDATE.len()` fits one.
#[cfg(self_update)]
static STAGED: AtomicU32 = AtomicU32::new(0);

#[cfg(self_update)]
impl Updatable for SelfSlot {
    fn poll_stage(&mut self, payload: &dyn ByteSource) -> Result<StageProgress, UpdateError> {
        let total = payload.len();
        if total == 0 {
            return Err(UpdateError::EmptyPayload);
        }
        let take = (total - self.written).min(STAGE_CHUNK as u64);
        let mut window = [0u8; STAGE_CHUNK];
        let window = &mut window[..take as usize];
        payload
            .read_at(self.written, window)
            .map_err(|_| UpdateError::Device)?;
        self.written += take;
        STAGED.store(self.written as u32, Ordering::Relaxed);
        if self.written == total {
            return Ok(StageProgress::Ready);
        }
        Ok(StageProgress::Transferring {
            progress: Progress {
                written: self.written,
                total,
            },
        })
    }

    fn abandon(&mut self) {
        self.written = 0;
        STAGED.store(0, Ordering::Relaxed);
    }

    /// Never called on this path: the eRoT's activation records a session
    /// and resets rather than commanding a device to run what it staged.
    fn activate(&mut self) -> Result<(), UpdateError> {
        Err(UpdateError::Device)
    }
}

/// Authenticates the staged candidate and reports the SVN it carries,
/// which is what the activation records on the session.
///
/// One pass, no chunking: the signature check is stubbed everywhere in
/// these scenarios, and what this seam has to supply here is a verdict
/// with an SVN in it.
#[cfg(self_update)]
struct SvnVerifier;

#[cfg(self_update)]
struct SvnSession;

#[cfg(self_update)]
impl IncrementalVerifier for SvnVerifier {
    type Error = BoardFault;
    type Session = SvnSession;

    fn start(self) -> SvnSession {
        SvnSession
    }
}

#[cfg(self_update)]
impl VerifySession for SvnSession {
    type Verifier = SvnVerifier;
    type Error = BoardFault;

    fn poll(self, _payload: &dyn ByteSource) -> PollOutcome<Self> {
        PollOutcome::Authenticated {
            verifier: SvnVerifier,
            image: AuthenticatedImage {
                svn: Some(CANDIDATE_SVN),
                measurement: None,
            },
        }
    }

    fn abandon(self) -> SvnVerifier {
        SvnVerifier
    }
}

/// The eRoT's reset, which is where this scenario's run ends.
///
/// Nothing on this target reboots the eRoT, and the driver does not come
/// back from here, so the claim is judged in place: the session has to
/// read `TrialPending` at the candidate's SVN, which is what the boot
/// after the reset would find. `self_trial` starts from exactly that
/// session and carries on from there.
#[cfg(self_update)]
struct EndOfRun;

#[cfg(self_update)]
impl SelfReset for EndOfRun {
    type Error = BoardFault;

    fn reset(&mut self) -> Result<core::convert::Infallible, BoardFault> {
        let staged = STAGED.load(Ordering::Relaxed);
        if staged != CANDIDATE.len() as u32 {
            // Checked first: a session marked pending for an image the
            // pump never finished writing is worse than an incomplete
            // staging alone.
            pw_log::error!(
                "the eRoT would reset into {} bytes of a {} byte candidate",
                staged as u32,
                CANDIDATE.len() as u32
            );
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        } else if SelfSession::read() == (SelfUpdateState::TrialPending { svn: CANDIDATE_SVN }) {
            pw_log::info!("the candidate is staged and the eRoT would reset into it now");
            let _ = syscall::debug_shutdown(Ok(()));
        } else {
            pw_log::error!("the activation requested a reset with no trial pending");
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        }
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

/// The candidate the pump pulls from.
#[cfg(self_update)]
struct CandidateBytes;

#[cfg(self_update)]
impl ByteSource for CandidateBytes {
    fn len(&self) -> u64 {
        CANDIDATE.len() as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
        let start = usize::try_from(offset).map_err(|_| ByteReadError::OutOfRange)?;
        let end = start
            .checked_add(buf.len())
            .ok_or(ByteReadError::OutOfRange)?;
        buf.copy_from_slice(CANDIDATE.get(start..end).ok_or(ByteReadError::OutOfRange)?);
        Ok(())
    }
}

/// No staging region, because nothing stages here.
#[cfg(not(self_update))]
struct NoStaging;

#[cfg(not(self_update))]
impl ByteSource for NoStaging {
    fn len(&self) -> u64 {
        0
    }

    fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> Result<(), ByteReadError> {
        Err(ByteReadError::OutOfRange)
    }
}

/// The digest the measuring build reports. A real one is a hash of the
/// image; this is a constant, because what the scenario claims is that a
/// measurement the verifier reported reaches the report sink, not that
/// anything hashed correctly.
#[cfg(measured)]
const DIGEST: Measurement = Measurement([0x5a; Measurement::LEN]);

/// Reports a constant digest in its verdict.
///
/// The shared stub authenticates and reports no digest, which is every
/// other scenario here, so the measurement path has never run:
/// `Report::Measured` is emitted only for a verdict carrying
/// `Some(measurement)`. The walk passes over the eRoT, so the only
/// component this is asked about is the managed device.
#[cfg(measured)]
struct MeasuringVerifier;

#[cfg(measured)]
impl Verifier for MeasuringVerifier {
    type Error = BoardFault;

    fn verify(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<Verdict, BoardFault> {
        Ok(Verdict::Authenticated(AuthenticatedImage {
            // Still nothing read: this build changes what the verifier
            // measures, not what it parses.
            svn: None,
            measurement: Some(DIGEST),
        }))
    }
}

/// What the sink was handed about the managed device's image: how many
/// measurements, and the last digest. A static because the sink lives
/// inside the board, inside the driver, while the run that judges it is
/// outside all three.
///
/// `Relaxed` on every access: the app is one thread, and the sink writes
/// these from the same run loop that reads them back, so there is no
/// second thread to order against.
static MEASUREMENTS: AtomicU32 = AtomicU32::new(0);
static MEASURED_ID: AtomicU8 = AtomicU8::new(u8::MAX);

/// The managed device's chain position. `chain_of` rejects a table that
/// does not start with the eRoT, and `CHAIN` runs it at compile time, so
/// position 1 is the first walked device whatever `DEVICES` holds.
const MANAGED_DEVICE: u8 = 1;

/// Reports go to the console. The test reads them the way an operator
/// would, and a scenario that ends badly says why in its own output.
struct LogSink;

impl ReportSink for LogSink {
    fn report(&mut self, report: Report) {
        match report {
            Report::Isolated(id) => pw_log::info!("report: component {} isolated", id.get() as u32),
            Report::RecoveryFailed(id) => {
                pw_log::info!(
                    "report: component {} out of recovery sources",
                    id.get() as u32
                )
            }
            Report::Measured { id, .. } => {
                MEASUREMENTS.fetch_add(1, Ordering::Relaxed);
                MEASURED_ID.store(id.get(), Ordering::Relaxed);
                pw_log::info!("report: component {} measured", id.get() as u32)
            }
            Report::BootFailed { id, checkpoint, .. } => pw_log::info!(
                "report: component {} failed at {}",
                id.get() as u32,
                checkpoint as &str
            ),
            _ => pw_log::info!("report: unrecognised"),
        }
    }
}

// The update wiring, named once per build so the `Board` carries no cfg.
// The self-update build wires the real pieces; every other build wires
// the stubs a boot scenario needs, which fault when polled so a wiring
// mistake cannot look like a verdict. `scenario_updatable` is a function
// rather than a constant because the self-update build's slot counts
// what it was handed. The reset differs the same way: an activation of
// the eRoT's own image supplies a reset of its own, and the `()` impl
// reports that it cannot drive one.

#[cfg(self_update)]
type ScenarioUpdatable = SelfSlot;
#[cfg(self_update)]
type ScenarioStaging = CandidateBytes;
#[cfg(self_update)]
type ScenarioUpdateVerifier = SvnVerifier;
#[cfg(self_update)]
type ScenarioSelfReset = EndOfRun;
#[cfg(self_update)]
fn scenario_updatable() -> ScenarioUpdatable {
    SelfSlot { written: 0 }
}
#[cfg(self_update)]
const SCENARIO_STAGING: ScenarioStaging = CandidateBytes;
#[cfg(self_update)]
const SCENARIO_UPDATE_VERIFIER: ScenarioUpdateVerifier = SvnVerifier;
#[cfg(self_update)]
const SCENARIO_SELF_RESET: ScenarioSelfReset = EndOfRun;

#[cfg(not(self_update))]
type ScenarioUpdatable = NoUpdate;
#[cfg(not(self_update))]
type ScenarioStaging = NoStaging;
#[cfg(not(self_update))]
type ScenarioUpdateVerifier = NeverVerifier;
#[cfg(not(self_update))]
type ScenarioSelfReset = ();
#[cfg(not(self_update))]
fn scenario_updatable() -> ScenarioUpdatable {
    NoUpdate
}
#[cfg(not(self_update))]
const SCENARIO_STAGING: ScenarioStaging = NoStaging;
#[cfg(not(self_update))]
const SCENARIO_UPDATE_VERIFIER: ScenarioUpdateVerifier = NeverVerifier;
#[cfg(not(self_update))]
const SCENARIO_SELF_RESET: ScenarioSelfReset = ();

struct MockDownstreamDeviceBoard;

impl BoardCapabilities for MockDownstreamDeviceBoard {
    type Image = StubImage;
    type Verifier = ScenarioVerifier;
    type BootControl = BmcReset;
    type BootWatch = CheckpointWalk<BmcEvidence, u8>;
    type SvnFloor = ErotFloor;
    type ReportSink = LogSink;
    type Updatable = ScenarioUpdatable;
    type Recovery = DeviceRecovery;
    type DeviceTrial = ();
    type Staging = ScenarioStaging;
    type UpdateVerifier = ScenarioUpdateVerifier;
    type SelfUpdate = SelfSession;
    type SelfReset = ScenarioSelfReset;
    type ResetSignal = u8;
    type Probe = u8;
}

fn board() -> Board<MockDownstreamDeviceBoard, N> {
    Board {
        // Built from the table index, so a component's parts cannot end
        // up wired to a different slot than the one they sit in.
        //
        // Index 0 is the eRoT. The walk passes over it, so nothing reads
        // or actuates that entry. Components are indexed by ComponentId,
        // so index 0 still needs a value.
        components: core::array::from_fn(|i| ComponentWiring {
            device: &DEVICES[i],
            image: StubImage,
            boot_control: BmcReset,
            boot_watch: CheckpointWalk::new(BmcEvidence, &DEVICES[i]),
            svn_floor: SvnFloorBinding::SelfManaged,
            updatable: scenario_updatable(),
            recovery: DEVICE_RECOVERY,
            trial: (),
        }),
        verifier: SCENARIO_VERIFIER,
        report_sink: LogSink,
        update_staging: SCENARIO_STAGING,
        update_verifier: Some(SCENARIO_UPDATE_VERIFIER),
        self_update: Some(ErotUpdate {
            session: SelfSession,
            floor: ErotFloor,
            reset: SCENARIO_SELF_RESET,
        }),
        update_stall_budget_millis: 1_000,
    }
}

/// The chain and the driver, as this scenario's update phase holds them.
#[cfg(self_update)]
type Core = openprot_orchestrator_sm::Orchestrator<N, E>;
#[cfg(self_update)]
type Driver = openprot_orchestrator_driver::PlatformDriver<MockDownstreamDeviceBoard, N>;

/// The eRoT, which is chain position 0 in every device table.
#[cfg(self_update)]
const EROT: ComponentId = ComponentId::new(0);

/// Stages a candidate into the eRoT's own slot and activates it.
///
/// This is the half `self_trial` does not cover. The two scenarios meet
/// at the reset: `self_update` ends where the eRoT would reboot, and
/// `self_trial` starts from the session that reset would have left
/// behind. One run cannot do both, because nothing on this target resets
/// the eRoT and the session lives in RAM.
///
/// Returns only when something went wrong. A successful activation records
/// the session, marks the trial pending and resets, and the reset is
/// [`EndOfRun`], which judges the run and shuts the target down.
#[cfg(self_update)]
fn self_update(core: &mut Core, driver: &mut Driver) -> Result<(), ()> {
    let len = CANDIDATE.len() as u64;
    if request_update(core, driver, EROT, len).is_err() {
        pw_log::error!("the eRoT's own update was refused");
        return Err(());
    }
    if core.state() != State::Updating(EROT) {
        pw_log::error!("the request did not reach Updating");
        return Err(());
    }
    pw_log::info!("staging {} bytes into the eRoT's own slot", len as u32);

    // The transfer steps, the authentication step, and the same again in
    // slack, so the bound can never cut a working transfer short. A run
    // that uses it up stopped making progress: a stalled pump returns no
    // event, so only this bound ends such a run.
    const UPDATE_ROUNDS: usize = 2 * (CANDIDATE.len() / STAGE_CHUNK + 1);
    for _ in 0..UPDATE_ROUNDS {
        let poll = driver.pump_update(now_millis());
        match poll.event {
            // The activation runs inside this dispatch and does not come
            // back from the reset.
            Some(Event::UpdateVerified) => {
                core.dispatch(driver, Event::UpdateVerified);
                pw_log::error!("the activation returned instead of resetting the eRoT");
                return Err(());
            }
            Some(event) => {
                pw_log::error!("the update pump gave up on the eRoT's own job");
                core.dispatch(driver, event);
                return Err(());
            }
            None => {}
        }
    }

    pw_log::error!("the pump never authenticated the eRoT's own candidate");
    Err(())
}

/// Drains whatever the mock downstream device has reported and latches a ready report.
/// Non-blocking: the run loop has already waited.
fn drain_reports() {
    let mut evt = [0u8; 1];
    while let Ok(len) = syscall::channel_read(handle::BOOT_EVT, 0usize, &mut evt) {
        let _ = syscall::channel_respond(handle::BOOT_EVT, &[0u8; 0]);
        if len == 1usize && evt[0] == 1 {
            DEVICE_READY.store(true, Ordering::Relaxed);
        }
    }
}

/// Whether the image measurement reached the report sink as this build
/// arranges: once, for the managed device, where the verifier reports a
/// digest, and never where it does not.
///
/// The second half is the half that bites. Every other scenario wires a
/// verifier that measures nothing, so a `Report::Measured` there would
/// mean a digest nobody computed.
fn measured_as_expected() -> bool {
    let expected = u32::from(cfg!(measured));
    let measurements = MEASUREMENTS.load(Ordering::Relaxed);
    if measurements != expected {
        pw_log::error!(
            "the sink was handed {} measurements, expected {}",
            measurements as u32,
            expected as u32
        );
        return false;
    }
    if cfg!(measured) && MEASURED_ID.load(Ordering::Relaxed) != MANAGED_DEVICE {
        pw_log::error!("the measurement named a component that is not the managed device");
        return false;
    }
    true
}

/// Whether `restore` was called exactly `EXPECT_RESTORE_CALLS` times.
/// The count is zero outside the recovery cfgs, so every other scenario
/// checks recovery never ran.
fn restore_calls_as_expected() -> bool {
    let calls = RESTORE_CALLS.load(Ordering::Relaxed);
    if calls == EXPECT_RESTORE_CALLS {
        return true;
    }
    pw_log::error!(
        "restore was called {} times, expected {}",
        calls as u32,
        EXPECT_RESTORE_CALLS as u32
    );
    false
}

fn run() -> Result<(), ()> {
    syscall::wait_group_add(handle::WG, handle::BOOT_EVT, Signals::READABLE, 0usize)
        .map_err(|_| ())?;

    SelfSession::install();

    let (mut core, mut driver) =
        bring_up::<MockDownstreamDeviceBoard, N, E>(&CHAIN, &DEVICES, board(), MAX_RETRY);

    // What the eRoT's last self-update was doing, read before anything
    // walks. A trial running right now has to be declared here, because
    // what judges it is this walk reaching `Ready`.
    let settlement = match settle_self_update_at_boot(&mut core, &mut driver) {
        Ok(settlement) => settlement,
        Err(_) => {
            pw_log::error!("the eRoT's own session could not be settled");
            return Err(());
        }
    };
    if settlement != EXPECT_SETTLEMENT {
        pw_log::error!("the eRoT's own session did not say what this boot is");
        return Err(());
    }

    // Power on and pass verification. The driver's release of the reset
    // line is what arms the walk, so nothing here touches the device
    // directly.
    core.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    if core.state() == State::Locked && !LOCKS_AT_POWER_ON {
        pw_log::error!("locked before the device was released");
        return Err(());
    }
    if LOCKS_AT_POWER_ON && core.state() != State::Locked {
        pw_log::error!("recovery did not run out of sources in the power-on dispatch");
        return Err(());
    }

    let give_up = deadline_in(RUN_BUDGET_MILLIS);

    // Reaching `Ready` is not the pass condition. A passive component is
    // released speculatively, so the machine is `Ready` as soon as the last
    // component verifies, whether or not the device ever came up. What
    // proves the boot is the walk's own verdict.
    let mut booted = false;

    loop {
        if core.state() == State::Locked {
            if !EXPECT_LOCKDOWN {
                pw_log::error!("orchestrator locked the platform");
                return Err(());
            }
            if !restore_calls_as_expected() || !measured_as_expected() {
                return Err(());
            }
            if EXPECT_RECOVERY {
                pw_log::info!("recovery ran out of sources and the platform locked");
            } else {
                pw_log::info!("the device never came up and the platform locked");
            }
            return Ok(());
        }
        if booted {
            if EXPECT_LOCKDOWN {
                pw_log::error!("the device reported ready when it should not have");
                return Err(());
            }
            // The chain is up. In a self-trial build that is the verdict on
            // the trial, and the session has to read `Committed`: without it
            // the UpdateSecurityRevision the Update Agent sends has nothing to
            // advance the floor for.
            if EXPECT_SELF_TRIAL
                && SelfSession::read() != (SelfUpdateState::Committed { svn: TRIAL_SVN })
            {
                pw_log::error!("the chain came up and the self trial was not confirmed");
                return Err(());
            }
            // Without the call count, `recovery` would pass on a verifier
            // that never refused the image.
            if !restore_calls_as_expected() || !measured_as_expected() {
                return Err(());
            }
            pw_log::info!("device booted and the orchestrator is in Ready");
            // Only `Ready` accepts an UpdateRequest; anywhere else the
            // state machine answers `ReportUpdateDeferred`. So the run
            // offers the eRoT its own update once the walk has finished.
            #[cfg(self_update)]
            return self_update(&mut core, &mut driver);
            #[cfg(not(self_update))]
            return Ok(());
        }

        let poll = driver.poll_boot_walks(now_millis());
        if let Some(event) = poll.event {
            match event {
                Event::Booted(_) => booted = true,
                // Released speculatively; not boot proof.
                Event::ComponentReady(_) => {}
                Event::BootFailed { checkpoint, .. } => {
                    pw_log::error!("device failed at checkpoint {}", checkpoint as &str);
                }
                _ => {}
            }
            core.dispatch(&mut driver, event);
            continue;
        }

        // Nothing to judge yet. Sleep until the walk's own deadline, or
        // until the device reports, whichever comes first. A lapsed
        // deadline is not an error here: the next poll is what judges it.
        let deadline = match poll.next_deadline_millis {
            Some(millis) => Instant::from_ticks(millis_to_ticks(millis)),
            None => give_up,
        };
        match syscall::object_wait(handle::WG, Signals::READABLE, deadline) {
            Ok(_) => drain_reports(),
            Err(pw_status::Error::DeadlineExceeded) => {}
            Err(_) => return Err(()),
        }

        if SystemClock::now() >= give_up {
            // Expecting lockdown and not getting one is a failure too: a
            // walk that never finished judged nothing.
            pw_log::error!("run budget expired with the machine still deciding");
            return Err(());
        }
    }
}

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(()) => {
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        }
    }
    #[expect(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
