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

use board_stubs::{BoardFault, StubImage, StubVerifier};
// A build that performs an activation of the eRoT's own image supplies a
// reset of its own, because the stub's job is to report that it cannot.
#[cfg(not(self_update))]
use board_stubs::StubSelfReset;
use openprot_orchestrator_driver::{
    bring_up, settle_self_image_at_boot, Board, BoardCapabilities, ComponentWiring, Report,
    ReportSink, SelfImage, SelfUpdateSettlement, SvnFloorBinding,
};
use openprot_orchestrator_sm::{ComponentAttrs, Event, PowerOnResult, State};
// Only the self-update build submits a job of its own; every other
// scenario here is a boot.
#[cfg(self_update)]
use openprot_orchestrator_driver::request_update;
#[cfg(self_update)]
use openprot_orchestrator_sm::ComponentId;
#[cfg(self_update)]
use orchestrator_capabilities::{AuthenticatedImage, Progress, SelfReset};
use orchestrator_capabilities::{
    BootControl, BootStatus, EvidenceReader, IncrementalVerifier, PollOutcome, RunningImage,
    SelfUpdate, SelfUpdateState, StageProgress, Svn, SvnFloor, Updatable, UpdateError,
    VerifySession,
};
use orchestrator_checkpoint_walk::CheckpointWalk;
use orchestrator_config::{
    assert_retry_reaches_every_image, chain_of, BootCheckpoint, ChainEntries, DeviceConfig,
};
use util_io::{ByteReadError, ByteSource};

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

/// Whether this build expects the device to come up.
///
/// The negative scenario is the same image with a device that never
/// reports, and it passes when the platform locks. Inverting the verdict
/// here rather than letting the target fail keeps a build error and a
/// caught failure from looking the same to the runner.
#[cfg(not(device_hangs))]
const EXPECT_LOCKDOWN: bool = false;
#[cfg(device_hangs)]
const EXPECT_LOCKDOWN: bool = true;

/// Whether this build boots the trial image of the eRoT's own update.
///
/// The variant scenario starts the session at `TrialPending` and runs the
/// same boot. What it adds is the claim that bringing the chain up confirms
/// the session, so the update agent can advance the floor afterwards.
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

fn millis_to_ticks(millis: u64) -> u64 {
    millis * SystemClock::TICKS_PER_SEC / 1000
}

fn ticks_to_millis(ticks: u64) -> u64 {
    ticks * 1000 / SystemClock::TICKS_PER_SEC
}

fn now_millis() -> u64 {
    ticks_to_millis(SystemClock::now().ticks())
}

fn deadline_in(millis: u64) -> Instant {
    Instant::from_ticks(SystemClock::now().ticks() + millis_to_ticks(millis))
}

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
/// Nothing is written anywhere: QEMU models one chip, so the image the
/// eRoT would boot is the one running. What the scenario needs from this
/// seam is that the pump ran to the end, so it counts the bytes it was
/// handed and reports `Ready` on the last one.
#[cfg(self_update)]
struct SelfSlot {
    written: u64,
}

/// Bytes the slot was handed, read at the end of the run. The slot itself
/// is inside the board, which the driver takes and hands no reference to.
/// A `u32` because this target has no 64-bit atomics, and the candidate is
/// two hundred and fifty-six bytes.
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
            // pump never finished writing is the worse failure of the two.
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
            pw_log::error!("the activation asked for a reset with no trial pending");
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
            Report::BootFailed { id, checkpoint, .. } => pw_log::info!(
                "report: component {} failed at {}",
                id.get() as u32,
                checkpoint as &str
            ),
            _ => pw_log::info!("report: unrecognised"),
        }
    }
}

/// The capabilities only the self-update build wires. Everywhere else the
/// board names the stubs a boot scenario needs, which fault when polled so
/// a wiring mistake cannot look like a verdict.
#[cfg(not(self_update))]
type ScenarioUpdatable = NoUpdate;
#[cfg(self_update)]
type ScenarioUpdatable = SelfSlot;

#[cfg(not(self_update))]
type ScenarioStaging = NoStaging;
#[cfg(self_update)]
type ScenarioStaging = CandidateBytes;

#[cfg(not(self_update))]
type ScenarioUpdateVerifier = NeverVerifier;
#[cfg(self_update)]
type ScenarioUpdateVerifier = SvnVerifier;

#[cfg(not(self_update))]
type ScenarioSelfReset = StubSelfReset;
#[cfg(self_update)]
type ScenarioSelfReset = EndOfRun;

/// One component's slot, fresh. A function rather than a constant because
/// the self-update build's slot counts what it was handed.
#[cfg(not(self_update))]
fn scenario_updatable() -> ScenarioUpdatable {
    NoUpdate
}
#[cfg(self_update)]
fn scenario_updatable() -> ScenarioUpdatable {
    SelfSlot { written: 0 }
}

#[cfg(not(self_update))]
const SCENARIO_STAGING: ScenarioStaging = NoStaging;
#[cfg(self_update)]
const SCENARIO_STAGING: ScenarioStaging = CandidateBytes;

#[cfg(not(self_update))]
const SCENARIO_UPDATE_VERIFIER: ScenarioUpdateVerifier = NeverVerifier;
#[cfg(self_update)]
const SCENARIO_UPDATE_VERIFIER: ScenarioUpdateVerifier = SvnVerifier;

#[cfg(not(self_update))]
const SCENARIO_SELF_RESET: ScenarioSelfReset = StubSelfReset;
#[cfg(self_update)]
const SCENARIO_SELF_RESET: ScenarioSelfReset = EndOfRun;

struct MockDownstreamDeviceBoard;

impl BoardCapabilities for MockDownstreamDeviceBoard {
    type Image = StubImage;
    type Verifier = StubVerifier;
    type BootControl = BmcReset;
    type BootWatch = CheckpointWalk<BmcEvidence, u8>;
    type SvnFloor = ErotFloor;
    type ReportSink = LogSink;
    type Updatable = ScenarioUpdatable;
    type Recovery = ();
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
            recovery: (),
        }),
        verifier: StubVerifier,
        report_sink: LogSink,
        update_staging: SCENARIO_STAGING,
        update_verifier: Some(SCENARIO_UPDATE_VERIFIER),
        self_image: Some(SelfImage {
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
/// This is the half `self_trial` does not cover. The two meet at the
/// reset: this ends where the eRoT would reboot, and `self_trial` starts
/// from the session that reset would have left behind. One run cannot do
/// both, because nothing on this target resets the eRoT and the session
/// lives in RAM.
///
/// Returns only when something went wrong. A successful activation records
/// the session, marks the trial pending and resets, and the reset is
/// [`EndOfRun`], which judges the run and shuts the target down.
///
/// Sixteen rounds is slack: the pump takes four to carry 256 bytes in
/// 64-byte steps and one more to authenticate.
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

    for _ in 0..16 {
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

fn run() -> Result<(), ()> {
    syscall::wait_group_add(handle::WG, handle::BOOT_EVT, Signals::READABLE, 0usize)
        .map_err(|_| ())?;

    SelfSession::install();

    let (mut core, mut driver) =
        bring_up::<MockDownstreamDeviceBoard, N, E>(&CHAIN, &DEVICES, board(), MAX_RETRY);

    // What the eRoT's last self-update was doing, read before anything
    // walks. A trial running right now has to be declared here, because
    // what judges it is this walk reaching `Ready`.
    let settlement = match settle_self_image_at_boot(&mut core, &mut driver) {
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
    if core.state() == State::Locked {
        pw_log::error!("locked before the device was released");
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
            if EXPECT_LOCKDOWN {
                pw_log::info!("the device never came up and the platform locked");
                return Ok(());
            }
            pw_log::error!("orchestrator locked the platform");
            return Err(());
        }
        if booted {
            if EXPECT_LOCKDOWN {
                pw_log::error!("the device reported ready when it should not have");
                return Err(());
            }
            // The chain is up. In a self-trial build that is the verdict on
            // the trial, and the session has to read `Committed`: without it
            // the update agent's UpdateSecurityRevision has nothing to
            // advance the floor for.
            if EXPECT_SELF_TRIAL
                && SelfSession::read() != (SelfUpdateState::Committed { svn: TRIAL_SVN })
            {
                pw_log::error!("the chain came up and the self trial was not confirmed");
                return Err(());
            }
            pw_log::info!("device booted and the orchestrator is in Ready");
            // The chain is up, which is what an update of the eRoT's own
            // image needs before it is offered one.
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
