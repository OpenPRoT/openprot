// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The RoT in the PLDM update scenario.
//!
//! Runs the shipped state machine and platform driver, and answers the Firmware
//! Device over IPC. The Firmware Device never decides anything: it reports what
//! the Update Agent requested and what came of it, and this app says whether
//! the update may go on. That is the gatekeeper shape the design calls for, and
//! it only means anything across a process boundary, which is why the
//! orchestrator is its own app rather than a core inside the FD.
//!
//! The exchange is three messages, each answered with one byte, 1 for
//! accepted and 0 for refused:
//!
//! 1. `UpdateRequested`, with the candidate's length. Accepted when the
//!    machine is supervised and the staging region can hold it.
//! 2. `VerifyOutcome`, carrying the device's verdict.
//! 3. `Activated`, which stands in for the updated device coming back up.
//!
//! What this does not yet prove: the driver's `Updatable` stages nothing,
//! because in this flow the Firmware Device pulls its own bytes from the
//! Update Agent, out of the orchestrator's sight. Closing that is the next
//! step, and until then `activate` is the only side of the seam with a real
//! caller.

#![no_main]
#![no_std]

use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(slot_resync)]
use core::sync::atomic::{AtomicU32, AtomicU8};

use openprot_orchestrator_driver::{
    bring_up, Board, BoardCapabilities, ComponentWiring, Report, ReportSink, SvnFloorBinding,
};
// Only the accepting path records a job or pumps it, so the scenario that
// refuses every request needs neither.
#[cfg(not(refused_update))]
use openprot_orchestrator_driver::request_update;
use openprot_orchestrator_sm::{
    ComponentAttrs, ComponentId, Effect, Event, Platform, PowerOnResult,
};
// Only the accepting path checks the state it reached.
#[cfg(not(slot_resync))]
use board_stubs::StubVerifier;
use board_stubs::{BoardFault, StubImage};
#[cfg(slot_resync)]
use openprot_orchestrator_driver::{ImageSource, Verdict, Verifier};
#[cfg(not(refused_update))]
use openprot_orchestrator_sm::State;
#[cfg(slot_resync)]
use orchestrator_capabilities::Progress;
use orchestrator_capabilities::{
    AuthenticatedImage, BootControl, BootStatus, EvidenceReader, IncrementalVerifier, PollOutcome,
    StageProgress, Svn, SvnFloor, Updatable, UpdateError, VerifySession,
};
use orchestrator_checkpoint_walk::CheckpointWalk;
use orchestrator_config::{
    assert_retry_reaches_every_image, chain_of, BootCheckpoint, ChainEntries, DeviceConfig,
};
use pldm_api::wire::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use pldm_api::{FdStatus, RejectReason};
use pldm_client::{ClientError, FdIpcClient, Reply};
use util_io::{ByteReadError, ByteSource};

use board_clock::deadline_in;
use userspace::syscall::Signals;
// Only the accepting path pumps the update, so only it reads the clock.
#[cfg(not(refused_update))]
use board_clock::now_millis;
use userspace::{entry, syscall};

use util_ipc::{AsyncChannelTransport, IpcHandle};

use app_orchestrator::handle;

/// The PLDM IPC buffers this process lends the kernel, sized to the
/// protocol's own maxima.
static mut SEND_BUF: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];
static mut RECV_BUF: [u8; MAX_RESPONSE_SIZE] = [0; MAX_RESPONSE_SIZE];

type Fd = FdIpcClient<AsyncChannelTransport<IpcHandle>>;

/// The component the Firmware Device updates. One device, so its id is
/// fixed rather than carried in the messages. Position 1 because the eRoT
/// holds position 0 in every device table.
const TARGET: ComponentId = ComponentId::new(1);

/// Whether the Update Agent sends UpdateSecurityRevision. Where it does
/// not, `activate` dispatches the event itself.
const AGENT_COMMITS_SVN: bool = cfg!(svn_commit) || cfg!(svn_commit_refused);

/// Chain capacity and the core's effect-sink cap (`E >= 2*N + 2`). `N`
/// counts the device table below, so the two cannot disagree.
const N: usize = DEVICES.len();
const E: usize = 2 * N + 2;
const MAX_RETRY: u8 = 3;

/// Size of the candidate the Update Agent hands over. Must match the image
/// the Firmware Device holds, because the staging region is sized from it.
const IMAGE_SIZE: usize = 1024;

/// Where the device is told to stage. It stages into its own flash and
/// ignores this, but the offer carries it, so the value has to be the one
/// the device actually uses or the message would be a lie.
const STAGING_BASE: u32 = 0x10_0000;

/// How long the device may take to answer a command. It serves its channel
/// whenever it is waiting on the RoT, so this bounds a device that stopped
/// serving, not one that is busy.
const DEVICE_TIMEOUT_MILLIS: u64 = 10_000;

/// How many device steps one update may take. A full run is four, so this
/// only catches a device that reports the same step forever.
const MAX_DEVICE_STEPS: usize = 16;

/// The device table. No device boots in this image, so the window is
/// nominal: what is being proven is the update path, not a boot walk.
/// Chain position 0 is the eRoT itself. It is addressable but never
/// supervised. This image does not update the eRoT, so the entry carries no
/// layout.
static DEVICES: [DeviceConfig<u8, u8>; 2] = [
    DeviceConfig::erot("erot", None),
    DeviceConfig::new(
        "device",
        0,
        &[BootCheckpoint::new(
            "ready",
            1,
            core::time::Duration::from_millis(1_000),
        )],
        None,
        ComponentAttrs::passive_required(),
    ),
];

static CHAIN: ChainEntries<N> = chain_of(&DEVICES);

const _: () = assert_retry_reaches_every_image(MAX_RETRY, &DEVICES);

/// Set when the RoT's own verifier authenticates the candidate. The device
/// verifies its image too, and the update only goes through when both say
/// so, so this records one half of that.
static ROT_AUTHENTICATED: AtomicBool = AtomicBool::new(false);

/// How many times the driver called `Updatable::activate`. The board sits
/// inside the driver, so this is how the run loop sees that activation
/// reached the device rather than only passing through the state machine.
/// A count rather than a flag because the spare-slot re-sync stages a
/// second time and must not activate again.
static ACTIVATIONS: AtomicUsize = AtomicUsize::new(0);

/// Set when the driver reported that the spare-slot re-sync pass did not
/// finish. A clean commit emits no report at all, so these three are how
/// the run tells a pass that worked from one that was merely quiet.
static RESYNC_FAILED: AtomicBool = AtomicBool::new(false);

/// Set when the SVN floor write failed after the pass had written the
/// spare slot.
static FLOOR_COMMIT_FAILED: AtomicBool = AtomicBool::new(false);

/// Set when the image the component runs carried no SVN, which leaves
/// the floor where it is.
static FLOOR_HELD: AtomicBool = AtomicBool::new(false);

/// The byte the candidate carries at `offset`. Offset-dependent, matching
/// what the Firmware Device expects, so a staging region that served the
/// wrong bytes would be caught rather than accepted.
fn expected_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

/// No reset line in this image: the update targets the device the Firmware
/// Device runs on, and nothing here holds it.
struct NoReset;

impl BootControl for NoReset {
    type Error = BoardFault;

    fn hold_in_reset(&mut self) -> Result<(), BoardFault> {
        Ok(())
    }

    fn release(&mut self) -> Result<(), BoardFault> {
        Ok(())
    }
}

/// No device reports boot evidence here, so the walk is satisfied at once.
/// A boot scenario is what proves the walk; this one proves the update.
struct AlwaysBooted;

impl EvidenceReader<u8> for AlwaysBooted {
    type Error = Infallible;

    fn read(&mut self, _probe: &u8) -> Result<BootStatus, Infallible> {
        Ok(BootStatus::Booted)
    }
}

/// The device keeps its own SVN floor: a PLDM Firmware Device commits
/// internally, so the eRoT holds no second one.
#[cfg(not(slot_resync))]
struct NoFloor;

#[cfg(not(slot_resync))]
impl SvnFloor for NoFloor {
    type Error = BoardFault;

    fn floor(&self) -> Result<Svn, BoardFault> {
        Err(BoardFault)
    }

    fn advance(&mut self, _to: Svn) -> Result<(), BoardFault> {
        Err(BoardFault)
    }
}

/// The SVN floor this scenario binds for the managed component.
///
/// A PLDM Firmware Device commits its revision internally, so every other
/// scenario here binds `SelfManaged` and names the floor that faults.
/// `slot_resync` is the one where the eRoT keeps the floor, which is what
/// makes the commit re-stage the spare slot.
#[cfg(not(slot_resync))]
type ScenarioFloor = NoFloor;
#[cfg(slot_resync)]
type ScenarioFloor = DeviceFloor;

/// The floor the eRoT keeps for the managed component, in RAM. The image
/// boots once, so nothing has to survive a reset; what the run needs is a
/// floor it can read back, which the capability alone does not offer.
#[cfg(slot_resync)]
struct DeviceFloor;

#[cfg(slot_resync)]
static DEVICE_FLOOR: AtomicU32 = AtomicU32::new(0);

#[cfg(slot_resync)]
impl SvnFloor for DeviceFloor {
    type Error = BoardFault;

    fn floor(&self) -> Result<Svn, BoardFault> {
        Ok(Svn(DEVICE_FLOOR.load(Ordering::Relaxed)))
    }

    fn advance(&mut self, to: Svn) -> Result<(), BoardFault> {
        DEVICE_FLOOR.fetch_max(to.0, Ordering::Relaxed);
        Ok(())
    }
}

/// Who keeps the SVN floor for the component at table index `i`.
///
/// Index 0 is the eRoT, whose own floor lives in `ErotUpdate` rather than
/// in this array, so it stays `SelfManaged` in both builds.
#[cfg(not(slot_resync))]
fn svn_floor_binding(_i: usize) -> SvnFloorBinding<ScenarioFloor> {
    SvnFloorBinding::SelfManaged
}

#[cfg(slot_resync)]
fn svn_floor_binding(i: usize) -> SvnFloorBinding<ScenarioFloor> {
    if i == TARGET.get() as usize {
        SvnFloorBinding::Erot(DeviceFloor)
    } else {
        SvnFloorBinding::SelfManaged
    }
}

/// The SVN the board verifier reads out of the image the component runs.
///
/// `commit_svn_floor` moves the floor to the SVN the chain walk recorded,
/// and an image with none makes it hold the advance and report
/// `SvnFloorHeld` instead. So the floor this run ends at says both that
/// the binding took effect and that the advance came from a verdict.
#[cfg(slot_resync)]
const RESYNC_SVN: Svn = Svn(9);

/// The board verifier for the re-sync run: the verdict the chain walk
/// makes, with an SVN on it. `StubVerifier` authenticates without one, which would
/// leave the floor where it is.
#[cfg(slot_resync)]
struct SvnVerifier;

#[cfg(slot_resync)]
impl Verifier for SvnVerifier {
    type Error = BoardFault;

    fn verify(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<Verdict, BoardFault> {
        Ok(Verdict::Authenticated(AuthenticatedImage {
            svn: Some(RESYNC_SVN),
            // No hash engine on a test board, so nothing to measure.
            measurement: None,
        }))
    }
}

#[cfg(not(slot_resync))]
type ScenarioVerifier = StubVerifier;
#[cfg(slot_resync)]
type ScenarioVerifier = SvnVerifier;

#[cfg(not(slot_resync))]
fn scenario_verifier() -> ScenarioVerifier {
    StubVerifier
}

#[cfg(slot_resync)]
fn scenario_verifier() -> ScenarioVerifier {
    SvnVerifier
}

/// The Firmware Device as the driver sees it.
///
/// Staging is already done by the time the orchestrator hears about it: the
/// device pulled its own chunks from the Update Agent, which is what DSP0267
/// has it do. So `poll_stage` reports the payload is already in place rather
/// than pushing it, and `activate` is the one call that reaches the device
/// for real. Replacing this stub with an IPC call to the Firmware Device is
/// the next step; what it would prove is the push direction, which this
/// flow does not use.
struct FdUpdatable;

#[cfg(not(slot_resync))]
impl Updatable for FdUpdatable {
    fn poll_stage(&mut self, _payload: &dyn ByteSource) -> Result<StageProgress, UpdateError> {
        Ok(StageProgress::Ready)
    }

    fn abandon(&mut self) {}

    fn activate(&mut self) -> Result<(), UpdateError> {
        ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// The same device, plus the slot the commit re-stages.
///
/// The first pass is what every other scenario sees: the device pulled
/// its own chunks from the Update Agent, so staging is already done.
/// After the activation the driver stages a second time, into the slot
/// the device stopped booting from, and that pass is the one the eRoT
/// performs. There is no PLDM operation that pushes bytes to a Firmware
/// Device, and only the Firmware Device process maps the flash, so the
/// spare slot is modelled here, in memory the RoT owns.
///
/// A chunk per call, so the pass takes several polls. A pass that ended
/// in one would say nothing about the SVN floor advance waiting behind
/// it.
#[cfg(slot_resync)]
impl Updatable for FdUpdatable {
    fn poll_stage(&mut self, payload: &dyn ByteSource) -> Result<StageProgress, UpdateError> {
        if ACTIVATIONS.load(Ordering::Relaxed) == 0 {
            return Ok(StageProgress::Ready);
        }

        // Read inside the pass, where the SVN floor must still hold the
        // value it had before the commit.
        if DEVICE_FLOOR.load(Ordering::Relaxed) != 0 {
            FLOOR_ADVANCED_MID_PASS.store(true, Ordering::Relaxed);
        }

        let total = payload.len();
        let written = SPARE_WRITTEN.load(Ordering::Relaxed) as u64;
        let take = core::cmp::min(RESYNC_CHUNK as u64, total - written) as usize;
        let mut chunk = [0u8; RESYNC_CHUNK];
        payload
            .read_at(written, &mut chunk[..take])
            .map_err(UpdateError::Payload)?;
        for (i, byte) in chunk[..take].iter().enumerate() {
            SPARE_SLOT[written as usize + i].store(*byte, Ordering::Relaxed);
        }
        let written = written + take as u64;
        SPARE_WRITTEN.store(written as usize, Ordering::Relaxed);

        if written < total {
            Ok(StageProgress::Transferring {
                progress: Progress { written, total },
            })
        } else {
            Ok(StageProgress::Ready)
        }
    }

    fn abandon(&mut self) {
        SPARE_WRITTEN.store(0, Ordering::Relaxed);
    }

    fn activate(&mut self) -> Result<(), UpdateError> {
        ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// The slot the device stopped booting from, as the `Updatable` the RoT
/// wires fills it. Checked byte for byte at the end of the run, so a pass that
/// wrote a pattern of its own rather than the staged bytes is caught.
#[cfg(slot_resync)]
static SPARE_SLOT: [AtomicU8; IMAGE_SIZE] = [const { AtomicU8::new(0) }; IMAGE_SIZE];

/// How much of the spare slot the pass has written.
#[cfg(slot_resync)]
static SPARE_WRITTEN: AtomicUsize = AtomicUsize::new(0);

/// Set when the SVN floor moved while the re-sync pass was still running.
#[cfg(slot_resync)]
static FLOOR_ADVANCED_MID_PASS: AtomicBool = AtomicBool::new(false);

/// Bytes one re-sync poll writes. Small enough that the pass takes more
/// than one call, which is what gives the held advance a window to be
/// observed in.
#[cfg(slot_resync)]
const RESYNC_CHUNK: usize = 64;

/// How many polls the re-sync pass may take. The image needs
/// `IMAGE_SIZE / RESYNC_CHUNK` of them, and this leaves room for the
/// steps the driver takes around them.
#[cfg(slot_resync)]
const MAX_RESYNC_PUMPS: usize = 4 * (IMAGE_SIZE / RESYNC_CHUNK);

/// The staging region, as big as the candidate and carrying the same bytes
/// the Firmware Device expects. Nothing reads it while the device stages
/// itself, but its length is what bounds the candidate at submit time, so a
/// candidate too large for the board is refused before an update starts.
struct PatternStaging;

impl ByteSource for PatternStaging {
    fn len(&self) -> u64 {
        IMAGE_SIZE as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
        let end = offset
            .checked_add(buf.len() as u64)
            .ok_or(ByteReadError::OutOfRange)?;
        if end > self.len() {
            return Err(ByteReadError::OutOfRange);
        }
        for (i, slot) in buf.iter_mut().enumerate() {
            *slot = expected_byte(offset as usize + i);
        }
        Ok(())
    }
}

/// The RoT's own check of the staged candidate.
///
/// Signature checking belongs to the crypto service and is stubbed until
/// the service exists, so this reads the staging region and compares it with the
/// pattern the Update Agent is expected to have sent. That makes the RoT's
/// verdict depend on the bytes rather than on nothing, which is what keeps
/// the authenticated path from passing vacuously.
///
/// One poll does the whole image. The trait asks for bounded steps so a long
/// check cannot stall the loop; at a kilobyte that does not arise.
struct PatternVerifier;

struct PatternSession;

impl IncrementalVerifier for PatternVerifier {
    type Error = BoardFault;
    type Session = PatternSession;

    fn start(self) -> PatternSession {
        PatternSession
    }
}

impl VerifySession for PatternSession {
    type Verifier = PatternVerifier;
    type Error = BoardFault;

    fn poll(self, payload: &dyn ByteSource) -> PollOutcome<Self> {
        let len = payload.len() as usize;
        if len == 0 {
            // An empty payload is a fault, never a vacuous pass.
            return PollOutcome::Fault(PatternVerifier, BoardFault);
        }

        let mut chunk = [0u8; 64];
        let mut offset = 0usize;
        while offset < len {
            let take = core::cmp::min(chunk.len(), len - offset);
            if payload.read_at(offset as u64, &mut chunk[..take]).is_err() {
                return PollOutcome::Fault(PatternVerifier, BoardFault);
            }
            for (i, byte) in chunk[..take].iter().enumerate() {
                if *byte != expected_byte(offset + i) {
                    return PollOutcome::Rejected(PatternVerifier);
                }
            }
            offset += take;
        }

        ROT_AUTHENTICATED.store(true, Ordering::Relaxed);
        PollOutcome::Authenticated {
            verifier: PatternVerifier,
            image: AuthenticatedImage {
                // The pattern check reads a synthetic image: no manifest
                // and no digest.
                svn: None,
                measurement: None,
            },
        }
    }

    fn abandon(self) -> PatternVerifier {
        PatternVerifier
    }
}

/// Reports go to the console, where the test reads them as an operator would.
struct LogSink;

impl ReportSink for LogSink {
    fn report(&mut self, report: Report) {
        match report {
            Report::Isolated(id) => {
                pw_log::info!("ORCH: report, component {} isolated", id.get() as u32)
            }
            Report::RecoveryFailed(id) => pw_log::info!(
                "ORCH: report, component {} out of recovery sources",
                id.get() as u32
            ),
            Report::BootFailed { id, checkpoint, .. } => pw_log::info!(
                "ORCH: report, component {} failed at {}",
                id.get() as u32,
                checkpoint as &str
            ),
            Report::SlotResyncFailed(id) => {
                RESYNC_FAILED.store(true, Ordering::Relaxed);
                pw_log::error!(
                    "ORCH: report, the spare slot for component {} was not re-synced",
                    id.get() as u32
                )
            }
            Report::SvnFloorCommitFailed(id) => {
                FLOOR_COMMIT_FAILED.store(true, Ordering::Relaxed);
                pw_log::error!(
                    "ORCH: report, the SVN floor for component {} was not written",
                    id.get() as u32
                )
            }
            Report::SvnFloorHeld(id) => {
                FLOOR_HELD.store(true, Ordering::Relaxed);
                pw_log::error!(
                    "ORCH: report, component {} carried no SVN to move its floor to",
                    id.get() as u32
                )
            }
            _ => pw_log::info!("ORCH: report, unrecognised"),
        }
    }
}

struct UpdateBoard;

impl BoardCapabilities for UpdateBoard {
    type Image = StubImage;
    type Verifier = ScenarioVerifier;
    type BootControl = NoReset;
    type BootWatch = CheckpointWalk<AlwaysBooted, u8>;
    type SvnFloor = ScenarioFloor;
    type ReportSink = LogSink;
    type Updatable = FdUpdatable;
    type Recovery = ();
    type DeviceTrial = ();
    type Staging = PatternStaging;
    type UpdateVerifier = PatternVerifier;
    type SelfUpdate = ();
    type SelfReset = ();
    type ResetSignal = u8;
    type Probe = u8;
}

fn board() -> Board<UpdateBoard, N> {
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
            boot_control: NoReset,
            boot_watch: CheckpointWalk::new(AlwaysBooted, &DEVICES[i]),
            svn_floor: svn_floor_binding(i),
            updatable: FdUpdatable,
            recovery: (),
            trial: (),
        }),
        verifier: scenario_verifier(),
        report_sink: LogSink,
        update_staging: PatternStaging,
        update_verifier: Some(PatternVerifier),
        self_update: None,
        update_stall_budget_millis: 5_000,
    }
}

type Core = openprot_orchestrator_sm::Orchestrator<N, E>;
type Driver = openprot_orchestrator_driver::PlatformDriver<UpdateBoard, N>;

/// The negative scenario refuses before looking at anything, which is the
/// one refusal that cannot be mistaken for a judgement about the candidate.
#[cfg(refused_update)]
fn accept_update(
    _core: &mut Core,
    _driver: &mut Driver,
    _candidate_len: u64,
) -> Option<RejectReason> {
    pw_log::info!("ORCH: refusing the update, as this scenario asks");
    Some(RejectReason::PolicyViolation)
}

/// Records the job and walks it to authenticated.
///
/// The pump's `UpdateVerified` is not dispatched here. It is the RoT's half
/// of the verdict; the device's half arrives as `ApplyPending`, and the
/// update only goes through when both agree. Eight rounds is slack: the pump
/// takes three to get from Submitted to Authenticated.
#[cfg(not(refused_update))]
fn accept_update(core: &mut Core, driver: &mut Driver, candidate_len: u64) -> Option<RejectReason> {
    // No updates until the platform knows what it has. Busy, not
    // PolicyViolation: this says "not now", not "not this image".
    if !core.state().is_supervised() {
        pw_log::error!("ORCH: an update was offered while the platform was not supervising");
        return Some(RejectReason::Busy);
    }
    if request_update(core, driver, TARGET, candidate_len).is_err() {
        pw_log::error!("ORCH: refused the update request");
        return Some(RejectReason::PolicyViolation);
    }
    if core.state() != State::Updating(TARGET) {
        pw_log::error!("ORCH: the request did not reach Updating");
        return Some(RejectReason::PolicyViolation);
    }

    let mut verified = false;
    for _ in 0..8 {
        let poll = driver.pump_update(now_millis());
        match poll.event {
            Some(Event::UpdateVerified) => {
                verified = true;
                break;
            }
            Some(event) => {
                pw_log::error!("ORCH: the update pump gave up on the job");
                core.dispatch(driver, event);
                return Some(RejectReason::PolicyViolation);
            }
            None => {}
        }
    }

    if !verified {
        pw_log::error!("ORCH: the pump never produced UpdateVerified");
        return Some(RejectReason::PolicyViolation);
    }

    if !ROT_AUTHENTICATED.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: the candidate never authenticated");
        return Some(RejectReason::PolicyViolation);
    }
    None
}

#[entry]
fn entry() {
    pw_log::info!("ORCH: app started");

    // SAFETY: taken once, at this process's entry point, and handed
    // straight to the transport that owns them from here on.
    let send = unsafe { &mut *core::ptr::addr_of_mut!(SEND_BUF) };
    let recv = unsafe { &mut *core::ptr::addr_of_mut!(RECV_BUF) };
    let transport = AsyncChannelTransport::new(IpcHandle::new(handle::FD), send, recv);
    let mut fd = FdIpcClient::new(transport);

    let (mut core, mut driver) =
        bring_up::<UpdateBoard, N, E>(&CHAIN, &DEVICES, board(), MAX_RETRY);

    // Power on and verify, which is what puts the machine in Ready. An
    // update request arriving before that is refused, which is the point of
    // the check rather than a race to avoid.
    if cfg!(offer_before_supervising) {
        // Skip PowerGood and VerificationPassed, so the machine is still
        // unsupervised when the offer arrives.
        pw_log::info!("ORCH: not supervising yet, waiting for the firmware device");
    } else {
        core.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
        core.dispatch(&mut driver, Event::VerificationPassed(TARGET));
        pw_log::info!("ORCH: supervising, waiting for the firmware device");
    }

    // The Firmware Device declares the run's verdict, as it did before: the
    // runner greps one sentinel, and in a scenario where the RoT refuses,
    // the RoT reaching a failure is what the test passes on. So this logs
    // what it reached and parks.
    let completed = run(&mut core, &mut driver, &mut fd);
    if completed {
        pw_log::info!("ORCH: the update completed");
    } else {
        pw_log::error!("ORCH: the update did not complete");
    }
    // The claim here is about the SVN floor the RoT keeps, which the
    // Firmware Device cannot see, so the RoT declares this run. The
    // Firmware Device is built with `rot_owns_verdict` and only logs.
    #[cfg(slot_resync)]
    let _ = if completed {
        syscall::debug_shutdown(Ok(()))
    } else {
        syscall::debug_shutdown(Err(pw_status::Error::Internal))
    };
    #[expect(clippy::empty_loop)]
    loop {}
}

/// Drives one update, device step by device step.
///
/// Asks what the device is waiting for, decides, commands, and asks again.
/// The device raises `Signals::USER` when it starts waiting, which this loop
/// does not need: a `QueryStatus` sits in the channel until the device next
/// serves it, and this orchestrator has nothing else to do meanwhile. An
/// orchestrator that did would wait on the signal instead of parking here.
fn run(core: &mut Core, driver: &mut Driver, fd: &mut Fd) -> bool {
    for _ in 0..MAX_DEVICE_STEPS {
        let Some(status) = query(fd) else {
            return false;
        };

        match status {
            FdStatus::OfferPending { total, .. } => {
                if !offer(core, driver, fd, total) {
                    return false;
                }
            }
            // Consent to start verifying, not a verdict: the device has
            // not read its image back yet.
            FdStatus::VerifyPending => {
                if !command(fd, "PerformVerify", |fd| fd.perform_verify()) {
                    return false;
                }
            }
            // The device only asks for apply once it has read the image
            // back and found it whole, so this is its verdict.
            FdStatus::ApplyPending => {
                if !verified(core, driver, fd) {
                    return false;
                }
            }
            FdStatus::ActivationPending => {
                if !activate(core, driver, fd) {
                    return false;
                }
                // The agent may still ask for the security revision to be
                // committed, which DSP0267 only takes once the device is
                // idle. Nothing else follows an activation.
                if !AGENT_COMMITS_SVN {
                    return true;
                }
            }
            // Committing the revision is a decision of its own: the image
            // is installed either way, and this is what stops the device
            // being rolled back to what it replaced.
            FdStatus::SvnCommitPending { component } => {
                pw_log::info!(
                    "ORCH: the agent asked to commit component {}'s security revision",
                    component as u32
                );
                match svn_commit(core, driver, fd) {
                    Commit::Failed => return false,
                    // The commit queued a second staging pass, and the
                    // SVN floor advance waits behind it.
                    #[cfg(slot_resync)]
                    Commit::Granted => return resync(core, driver),
                    // `svn_commit` sends one request and is done.
                    #[cfg(not(slot_resync))]
                    Commit::Granted if !cfg!(svn_commit_refused) => return true,
                    #[cfg(not(slot_resync))]
                    Commit::Granted => {}
                    // The second request in `svn_commit_refused`.
                    Commit::Refused if cfg!(svn_commit_refused) => return true,
                    Commit::Refused => {
                        pw_log::error!("ORCH: the revision commit was refused");
                        return false;
                    }
                }
            }
            FdStatus::PhaseFailed { phase, result_code } => {
                pw_log::error!(
                    "ORCH: the device failed phase {} with code {}",
                    phase as u32,
                    result_code as u32
                );
                core.dispatch(driver, Event::UpdateRejected);
                return false;
            }
            // The requester withdrew. Drop staged work, ack the cancel.
            FdStatus::Cancelled => {
                pw_log::info!("ORCH: the update was withdrawn");
                core.dispatch(driver, Event::UpdateCancelled);
                // State is not imported when refused_update is set.
                #[cfg(not(refused_update))]
                if core.state() != State::Ready {
                    pw_log::error!("ORCH: the machine did not settle after the withdrawal");
                    return false;
                }
                if !command(fd, "AckCancel", |fd| fd.ack_cancel()) {
                    return false;
                }
                return false;
            }
            FdStatus::Idle { .. } => {
                pw_log::error!("ORCH: the device went idle mid-update");
                return false;
            }
            _ => {
                pw_log::error!("ORCH: the device reported a status this scenario does not drive");
                return false;
            }
        }
    }

    pw_log::error!("ORCH: the device never finished");
    false
}

/// Records the job, walks it to authenticated, and answers the offer.
fn offer(core: &mut Core, driver: &mut Driver, fd: &mut Fd, total: u32) -> bool {
    if let Some(reason) = accept_update(core, driver, u64::from(total)) {
        let _ = fd.reject_offer(reason);
        let _ = settle(fd);
        pw_log::error!("ORCH: refused the offer");
        return false;
    }
    pw_log::info!("ORCH: update accepted, {} bytes", total as u32);
    command(fd, "AcceptOffer", |fd| fd.accept_offer(STAGING_BASE))
}

/// The device says it verified. The update goes on only if the RoT's own
/// verifier agreed too.
fn verified(core: &mut Core, driver: &mut Driver, fd: &mut Fd) -> bool {
    if !ROT_AUTHENTICATED.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: the device accepted a candidate the RoT did not");
        core.dispatch(driver, Event::UpdateRejected);
        let _ = fd.reject_apply(RejectReason::PolicyViolation);
        let _ = settle(fd);
        return false;
    }

    core.dispatch(driver, Event::UpdateVerified);
    if ACTIVATIONS.load(Ordering::Relaxed) == 0 {
        pw_log::error!("ORCH: the verdict did not reach the device");
        return false;
    }
    pw_log::info!("ORCH: both verdicts agree, activating");
    command(fd, "PerformApply", |fd| fd.perform_apply())
}

/// Commands the activation and closes the commit window.
fn activate(core: &mut Core, driver: &mut Driver, fd: &mut Fd) -> bool {
    if !command(fd, "PerformActivate", |fd| fd.perform_activate()) {
        return false;
    }
    // Stands in for the updated device coming back up. Nothing in this
    // image reboots, so the acknowledged activation is what this scenario
    // treats as a healthy first boot.
    core.dispatch(driver, Event::BootConfirmed(TARGET));
    // The SVN floor moves on the request the Update Agent sends, not on the
    // boot. Where the Update Agent sends no UpdateSecurityRevision,
    // `Event::SvnCommitRequest` stands in for it, as `BootConfirmed` above
    // stands in for the reboot.
    if !AGENT_COMMITS_SVN {
        core.dispatch(driver, Event::SvnCommitRequest(TARGET));
    }
    pw_log::info!("ORCH: update committed");
    true
}

/// Grants or refuses the request the Update Agent sends to commit the
/// device's security revision.
///
/// The machine drops `SvnCommitRequest` for a component with nothing
/// outstanding, and a dropped request emits no effect. So the grant waits
/// on `Effect::CommitSvnFloor` actually arriving: answering `perform`
/// without it would tell the Update Agent the RoT granted something it
/// never saw.
///
/// `svn_commit_refused` sends a second request with nothing outstanding,
/// so the three answers have to be told apart.
fn svn_commit(core: &mut Core, driver: &mut Driver, fd: &mut Fd) -> Commit {
    let mut granted = false;
    core.dispatch_with(Event::SvnCommitRequest(TARGET), |effect| {
        if effect == Effect::CommitSvnFloor(TARGET) {
            granted = true;
        }
        driver.execute(effect)
    });
    if !granted {
        // Info, not error: `svn_commit_refused` arranges this refusal.
        pw_log::info!("ORCH: nothing outstanding to commit a revision for");
        let _ = fd.reject_svn_commit(RejectReason::PolicyViolation);
        let _ = settle(fd);
        return Commit::Refused;
    }
    if !command(fd, "PerformSvnCommit", |fd| fd.perform_svn_commit()) {
        return Commit::Failed;
    }
    Commit::Granted
}

/// Runs the spare-slot re-sync the granted commit queued, and checks that
/// the SVN floor advance waited for it.
///
/// `commit_svn_floor` answers the effect before the pass starts, so the
/// Update Agent has already been told the commit succeeded. What is left
/// to show is the order the driver keeps: the floor keeps the old SVN
/// while the slot the device stopped booting from is still behind, and
/// takes the new one once that slot holds the image.
#[cfg(slot_resync)]
fn resync(core: &mut Core, driver: &mut Driver) -> bool {
    if DEVICE_FLOOR.load(Ordering::Relaxed) != 0 {
        pw_log::error!("ORCH: the SVN floor moved before the spare slot was re-staged");
        return false;
    }
    if driver.pending_update() != Some(TARGET) {
        pw_log::error!("ORCH: the commit queued no re-sync for the spare slot");
        return false;
    }

    // The pass belongs to the driver alone: nothing is told, and no
    // requester is waiting on it, so it reports no event and no progress.
    let mut done = false;
    for _ in 0..MAX_RESYNC_PUMPS {
        let poll = driver.pump_update(now_millis());
        if poll.event.is_some() {
            pw_log::error!("ORCH: the re-sync told the state machine something");
            return false;
        }
        if poll.progress.is_some() {
            pw_log::error!("ORCH: the re-sync reported progress to a requester");
            return false;
        }
        if driver.pending_update().is_none() {
            done = true;
            break;
        }
    }
    if !done {
        pw_log::error!("ORCH: the re-sync pass never finished");
        return false;
    }

    if FLOOR_ADVANCED_MID_PASS.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: the SVN floor moved while the spare slot was still behind");
        return false;
    }
    let written = SPARE_WRITTEN.load(Ordering::Relaxed);
    if written != IMAGE_SIZE {
        pw_log::error!(
            "ORCH: the spare slot took {} bytes of the image",
            written as u32
        );
        return false;
    }
    for (offset, byte) in SPARE_SLOT.iter().enumerate() {
        if byte.load(Ordering::Relaxed) != expected_byte(offset) {
            pw_log::error!(
                "ORCH: the spare slot holds the wrong byte at offset {}",
                offset as u32
            );
            return false;
        }
    }
    if DEVICE_FLOOR.load(Ordering::Relaxed) != RESYNC_SVN.0 {
        pw_log::error!(
            "ORCH: the SVN floor reads {} after the re-sync",
            DEVICE_FLOOR.load(Ordering::Relaxed) as u32
        );
        return false;
    }
    if RESYNC_FAILED.load(Ordering::Relaxed)
        || FLOOR_COMMIT_FAILED.load(Ordering::Relaxed)
        || FLOOR_HELD.load(Ordering::Relaxed)
    {
        pw_log::error!("ORCH: the re-sync reported a failure");
        return false;
    }
    // The device keeps running what it committed: the pass stages, and
    // nothing activates behind it.
    let activations = ACTIVATIONS.load(Ordering::Relaxed);
    if activations != 1 {
        pw_log::error!("ORCH: the image was activated {} times", activations as u32);
        return false;
    }
    if core.state() != State::Ready {
        pw_log::error!("ORCH: the machine left Ready while the spare slot was re-staged");
        return false;
    }
    pw_log::info!(
        "ORCH: the spare slot holds the committed image and the SVN floor reads {}",
        DEVICE_FLOOR.load(Ordering::Relaxed) as u32
    );
    true
}

/// What the RoT answered one UpdateSecurityRevision request with.
enum Commit {
    /// The floor took the SVN, and the RoT sent PerformSvnCommit.
    Granted,
    /// The state machine emitted no `Effect::CommitSvnFloor`, so the RoT
    /// sent RejectSvnCommit.
    Refused,
    /// The exchange with the device did not complete.
    Failed,
}

/// Asks the device what it is waiting for.
fn query(fd: &mut Fd) -> Option<FdStatus> {
    if fd.query_status().is_err() {
        pw_log::error!("ORCH: could not ask the device for its status");
        return None;
    }
    match settle(fd) {
        Some(Reply::Status(status)) => Some(status),
        Some(Reply::Acked) => {
            pw_log::error!("ORCH: QueryStatus was acknowledged instead of answered");
            None
        }
        None => None,
    }
}

/// Sends one command and waits for the device to take it.
fn command(
    fd: &mut Fd,
    name: &str,
    start: impl FnOnce(&mut Fd) -> Result<(), ClientError>,
) -> bool {
    if start(fd).is_err() {
        pw_log::error!("ORCH: could not send {}", name as &str);
        return false;
    }
    match settle(fd) {
        Some(Reply::Acked) => true,
        Some(Reply::Status(_)) => {
            pw_log::error!("ORCH: a command was answered with a status");
            false
        }
        None => false,
    }
}

/// Waits for the answer to whatever is in flight.
///
/// The deadline is the one that matters in this scenario: a device that
/// stops serving its channel leaves this loop with a log line rather than
/// stalling the run until the harness times out.
fn settle(fd: &mut Fd) -> Option<Reply> {
    let deadline = deadline_in(DEVICE_TIMEOUT_MILLIS);
    loop {
        match fd.poll() {
            Ok(Some(reply)) => return Some(reply),
            Ok(None) => {}
            Err(_) => {
                pw_log::error!("ORCH: the device's answer did not decode");
                return None;
            }
        }
        if syscall::object_wait(handle::FD, Signals::READABLE, deadline).is_err() {
            pw_log::error!("ORCH: the device stopped answering");
            return None;
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
