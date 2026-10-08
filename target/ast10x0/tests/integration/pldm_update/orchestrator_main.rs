// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The RoT in the PLDM update scenario.
//!
//! Runs the shipped state machine and platform driver, and answers the
//! firmware device over IPC. The firmware device never decides anything: it
//! reports what the update agent asked for and what came of it, and this app
//! says whether the update may go on. That is the gatekeeper shape the design
//! calls for, and it only means anything across a process boundary, which is
//! why the orchestrator is its own app rather than a core inside the FD.
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
//! because in this flow the firmware device pulls its own bytes from the
//! update agent, out of the orchestrator's sight. Closing that is the next
//! step, and until then `activate` is the only side of the seam with a real
//! caller.

#![no_main]
#![no_std]

use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, Ordering};

use openprot_orchestrator_driver::{
    bring_up, Board, BoardCapabilities, ImageSource, Report, ReportSink, SvnFloorBinding, Verdict,
    Verifier,
};
// Only the accepting path records a job or pumps it, so the scenario that
// refuses every request needs neither.
#[cfg(not(refused_update))]
use openprot_orchestrator_driver::request_update;
use openprot_orchestrator_sm::{ComponentAttrs, ComponentId, Event, PowerOnResult};
// Only the accepting path checks the state it reached.
#[cfg(not(refused_update))]
use openprot_orchestrator_sm::State;
use orchestrator_capabilities::{
    BootControl, BootStatus, EvidenceReader, IncrementalVerifier, PollOutcome, StageProgress, Svn,
    SvnFloor, Updatable, UpdateError, VerifySession,
};
use orchestrator_checkpoint_walk::CheckpointWalk;
use orchestrator_config::{
    assert_retry_reaches_every_image, chain_of, BootCheckpoint, ChainEntries, DeviceConfig,
};
use pldm_api::wire::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use pldm_api::{FdStatus, RejectReason};
use pldm_client::{ClientError, FdIpcClient, Reply};
use util_io::{ByteReadError, ByteSource};

use userspace::syscall::Signals;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use util_ipc::{AsyncChannelTransport, IpcHandle};

use app_orchestrator::handle;

/// The PLDM IPC buffers this process lends the kernel, sized to the
/// protocol's own maxima.
static mut SEND_BUF: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];
static mut RECV_BUF: [u8; MAX_RESPONSE_SIZE] = [0; MAX_RESPONSE_SIZE];

type Fd = FdIpcClient<AsyncChannelTransport<IpcHandle>>;

/// The component the firmware device updates. One device, so its id is
/// fixed rather than carried in the messages.
const TARGET: ComponentId = ComponentId::new(0);

/// Chain capacity and the core's effect-sink cap (`E >= 2*N + 2`).
const N: usize = 1;
const E: usize = 2 * N + 2;
const MAX_RETRY: u8 = 3;

/// Size of the candidate the update agent hands over. Must match the
/// firmware device's image, because the staging region is sized from it.
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
static DEVICES: [DeviceConfig<u8, u8>; N] = [DeviceConfig::new(
    "device",
    0,
    &[BootCheckpoint::new(
        "ready",
        1,
        core::time::Duration::from_millis(1_000),
    )],
    None,
    ComponentAttrs::passive_required(),
)];

static CHAIN: ChainEntries<N> = chain_of(&DEVICES);

const _: () = assert_retry_reaches_every_image(MAX_RETRY, &DEVICES);

/// Set when the RoT's own verifier authenticates the candidate. The device
/// verifies its image too, and the update only goes through when both say
/// so, so this records one half of that.
static ROT_AUTHENTICATED: AtomicBool = AtomicBool::new(false);

/// Set when the driver actually calls `Updatable::activate`. The board sits
/// inside the driver, so this is how the run loop sees that activation
/// reached the device rather than only passing through the state machine.
static ACTIVATED: AtomicBool = AtomicBool::new(false);

/// One board fault, for every seam that has to name an error type.
#[derive(Debug)]
struct BoardFault;

impl core::fmt::Display for BoardFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("board fault")
    }
}

impl core::error::Error for BoardFault {}

/// The byte the candidate carries at `offset`. Offset-dependent, matching
/// the firmware device's own expectation, so a staging region that served
/// the wrong bytes would be caught rather than accepted.
fn expected_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

/// No reset line in this image: the device the update targets is the
/// firmware device's own, and nothing here holds it.
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

/// Signature checking is stubbed until the crypto service exists.
struct StubVerifier;

impl Verifier for StubVerifier {
    type Error = BoardFault;

    fn verify(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<Verdict, BoardFault> {
        Ok(Verdict::Authenticated {
            svn: Svn(1),
            // No hash engine on a test board: the stub verifier reads
            // nothing, so there is nothing to measure.
            measurement: None,
        })
    }
}

/// The running image, which the stub verifier never reads.
struct StubImage;

impl ImageSource for StubImage {
    type Error = BoardFault;

    fn open(&mut self) -> Result<(), BoardFault> {
        Ok(())
    }

    fn size(&self) -> Result<usize, BoardFault> {
        Ok(0)
    }

    fn read_at(&mut self, _offset: usize, _buf: &mut [u8]) -> Result<(), BoardFault> {
        Err(BoardFault)
    }
}

/// The device keeps its own floor: a PLDM firmware device commits
/// internally, so the eRoT holds no second one.
struct NoFloor;

impl SvnFloor for NoFloor {
    type Error = BoardFault;

    fn floor(&self) -> Result<Svn, BoardFault> {
        Err(BoardFault)
    }

    fn advance(&mut self, _to: Svn) -> Result<(), BoardFault> {
        Err(BoardFault)
    }
}

/// The firmware device as the driver sees it.
///
/// Staging is already done by the time the orchestrator hears about it: the
/// device pulled its own chunks from the update agent, which is what DSP0267
/// has it do. So `poll_stage` reports the payload is already in place rather
/// than pushing it, and `activate` is the one call that reaches the device
/// for real. Replacing this stub with an IPC call to the firmware device is
/// the next step; what it would prove is the push direction, which this
/// flow does not use.
struct FdUpdatable;

impl Updatable for FdUpdatable {
    fn poll_stage(&mut self, _payload: &dyn ByteSource) -> Result<StageProgress, UpdateError> {
        Ok(StageProgress::Ready)
    }

    fn abandon(&mut self) {}

    fn activate(&mut self) -> Result<(), UpdateError> {
        ACTIVATED.store(true, Ordering::Relaxed);
        Ok(())
    }
}

/// The staging region, as big as the candidate and carrying the same bytes
/// the firmware device expects. Nothing reads it while the device stages
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
/// pattern the update agent is expected to have sent. That makes the RoT's
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
        PollOutcome::Authenticated(PatternVerifier)
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
            _ => pw_log::info!("ORCH: report, unrecognised"),
        }
    }
}

struct UpdateBoard;

impl BoardCapabilities for UpdateBoard {
    type Image = StubImage;
    type Verifier = StubVerifier;
    type BootControl = NoReset;
    type BootWatch = CheckpointWalk<AlwaysBooted, u8>;
    type SvnFloor = NoFloor;
    type ReportSink = LogSink;
    type Updatable = FdUpdatable;
    type Recovery = ();
    type Staging = PatternStaging;
    type UpdateVerifier = PatternVerifier;
}

fn board() -> Board<UpdateBoard, N> {
    Board {
        images: [StubImage],
        verifier: StubVerifier,
        boot_controls: [NoReset],
        boot_watches: [CheckpointWalk::new(AlwaysBooted, &DEVICES[0])],
        svn_floors: [SvnFloorBinding::SelfManaged],
        report_sink: LogSink,
        updatables: [FdUpdatable],
        recovery: [()],
        update_staging: PatternStaging,
        update_verifier: Some(PatternVerifier),
        update_stall_budget_millis: 5_000,
    }
}

#[cfg(not(refused_update))]
fn now_millis() -> u64 {
    SystemClock::now().ticks() * 1000 / SystemClock::TICKS_PER_SEC
}

fn deadline_in(millis: u64) -> Instant {
    Instant::from_ticks(SystemClock::now().ticks() + millis * SystemClock::TICKS_PER_SEC / 1000)
}

type Core = openprot_orchestrator_sm::Orchestrator<N, E>;
type Driver = openprot_orchestrator_driver::PlatformDriver<UpdateBoard, N>;

/// The negative scenario refuses before looking at anything, which is the
/// one refusal that cannot be mistaken for a judgement about the candidate.
#[cfg(refused_update)]
fn accept_update(_core: &mut Core, _driver: &mut Driver, _candidate_len: u64) -> bool {
    pw_log::info!("ORCH: refusing the update, as this scenario asks");
    false
}

/// Records the job and walks it to authenticated.
///
/// The pump's `UpdateVerified` is not dispatched here. It is the RoT's half
/// of the verdict; the device's half arrives as `ApplyPending`, and the
/// update only goes through when both agree. Eight rounds is slack: the pump
/// takes three to get from Submitted to Authenticated.
#[cfg(not(refused_update))]
fn accept_update(core: &mut Core, driver: &mut Driver, candidate_len: u64) -> bool {
    if request_update(core, driver, TARGET, candidate_len).is_err() {
        pw_log::error!("ORCH: refused the update request");
        return false;
    }
    if core.state() != State::Updating(TARGET) {
        pw_log::error!("ORCH: the request did not reach Updating");
        return false;
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
                return false;
            }
            None => {}
        }
    }

    if !verified {
        pw_log::error!("ORCH: the pump never produced UpdateVerified");
        return false;
    }

    if !ROT_AUTHENTICATED.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: the candidate never authenticated");
        return false;
    }
    true
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

    let (mut core, mut driver) = bring_up::<UpdateBoard, N, E>(&CHAIN, board(), MAX_RETRY);

    // Power on and verify, which is what puts the machine in Ready. An
    // update request arriving before that is refused, which is the point of
    // the check rather than a race to avoid.
    core.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    core.dispatch(&mut driver, Event::VerificationPassed(TARGET));
    pw_log::info!("ORCH: supervising, waiting for the firmware device");

    // The firmware device declares the run's verdict, as it did before: the
    // runner greps one sentinel, and in a scenario where the RoT refuses,
    // the RoT reaching a failure is what the test passes on. So this logs
    // what it reached and parks.
    if run(&mut core, &mut driver, &mut fd) {
        pw_log::info!("ORCH: the update completed");
    } else {
        pw_log::error!("ORCH: the update did not complete");
    }
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
                return true;
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
    if !accept_update(core, driver, u64::from(total)) {
        let _ = fd.reject_offer(RejectReason::PolicyViolation);
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
    if !ACTIVATED.load(Ordering::Relaxed) {
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
    pw_log::info!("ORCH: update committed");
    true
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
        pw_log::error!("ORCH: could not send {}", name);
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
