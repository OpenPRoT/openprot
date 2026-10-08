// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The RoT in the full update scenario: it supervises the managed device
//! and commands the firmware device, and the point of the scenario is that
//! it does both in one run.
//!
//! The claim is an ordering. The device boots under supervision and the
//! walk completes. An update arrives over PLDM, the RoT accepts it, the
//! device stages and verifies it, the RoT activates it. Activation only
//! proposes the image, so the state machine re-walks: the device is reset,
//! boots the image it was just given, and reports ready a second time.
//! Only then is the image proven, and only then does the floor commit.
//!
//! Each of those steps has its own scenario elsewhere. What cannot be
//! tested apart is that they happen in that order, with the second ready
//! report caused by the reset that followed the activation rather than
//! left over from the first boot.

#![no_main]
#![no_std]

use core::convert::Infallible;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use openprot_orchestrator_driver::{
    bring_up, request_update, Board, BoardCapabilities, ImageSource, Report, ReportSink,
    SvnFloorBinding, Verdict, Verifier,
};
use openprot_orchestrator_sm::{ComponentAttrs, ComponentId, Event, PowerOnResult, State};
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
use util_ipc::{AsyncChannelTransport, IpcHandle};

use userspace::syscall::Signals;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use app_orchestrator::handle;

/// The component this RoT supervises and updates.
const TARGET: ComponentId = ComponentId::new(0);

/// Chain capacity and the core's effect-sink cap (`E >= 2*N + 2`).
const N: usize = 1;
const E: usize = 2 * N + 2;
const MAX_RETRY: u8 = 3;

/// Bytes in the image the agent hands over, and where the device stages it.
const IMAGE_SIZE: usize = 1024;
const STAGING_BASE: u32 = 0x10_0000;

/// How long the managed device has to report ready after a reset. It boots
/// in 50 ms, so this is slack rather than a race.
const BOOT_WINDOW_MILLIS: u64 = 2_000;

/// How long one supervised boot may take before the run gives up. Larger
/// than the window so a late device is judged by the walk, not by this.
const BOOT_BUDGET_MILLIS: u64 = 10_000;

/// How long the firmware device may take to answer a command.
const DEVICE_TIMEOUT_MILLIS: u64 = 10_000;

/// How many device steps one update may take. A full run is four.
const MAX_DEVICE_STEPS: usize = 16;

/// Reset command bytes, read by the managed device as its reset line.
const RESET_ASSERT: u8 = 0;
const RESET_RELEASE: u8 = 1;

/// The PLDM IPC buffers this process lends the kernel.
static mut SEND_BUF: [u8; MAX_REQUEST_SIZE] = [0; MAX_REQUEST_SIZE];
static mut RECV_BUF: [u8; MAX_RESPONSE_SIZE] = [0; MAX_RESPONSE_SIZE];

type Fd = FdIpcClient<AsyncChannelTransport<IpcHandle>>;
type Core = openprot_orchestrator_sm::Orchestrator<N, E>;
type Driver = openprot_orchestrator_driver::PlatformDriver<UpdateBoard, N>;

/// The device table. One checkpoint: the device reports ready inside the
/// window or it does not.
static DEVICES: [DeviceConfig<u8, u8>; N] = [DeviceConfig::new(
    "bmc",
    0,
    &[BootCheckpoint::new(
        "ready",
        1,
        core::time::Duration::from_millis(BOOT_WINDOW_MILLIS),
    )],
    None,
    ComponentAttrs::passive_required(),
)];

static CHAIN: ChainEntries<N> = chain_of(&DEVICES);

const _: () = assert_retry_reaches_every_image(MAX_RETRY, &DEVICES);

/// The managed device's ready line, as the RoT sees it. Cleared by the
/// reset path so a report from one boot cannot satisfy the next walk.
static DEVICE_READY: AtomicBool = AtomicBool::new(false);

/// How many times the device has been let out of reset. The scenario's
/// ordering claim rests on this going up across the activation.
static RESETS: AtomicU32 = AtomicU32::new(0);

/// Set when the RoT's own verifier authenticates the candidate.
static ROT_AUTHENTICATED: AtomicBool = AtomicBool::new(false);

/// Set when the driver calls `Updatable::activate`.
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

fn now_millis() -> u64 {
    SystemClock::now().ticks() * 1000 / SystemClock::TICKS_PER_SEC
}

fn deadline_in(millis: u64) -> Instant {
    Instant::from_ticks(SystemClock::now().ticks() + millis * SystemClock::TICKS_PER_SEC / 1000)
}

/// The byte the agent sends at `offset`, which is also what the staging
/// region serves and what the device checks.
fn expected_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

/// Drives the managed device's reset line. An undeliverable command is a
/// wiring failure, not a boot failure.
fn drive_reset(level: u8) -> Result<(), BoardFault> {
    let mut ack = [0u8; 1];
    syscall::channel_transact(handle::RESET_CMD, &[level], &mut ack, deadline_in(1_000))
        .map(|_| ())
        .map_err(|_| BoardFault)
}

/// The managed device's reset line. Both directions clear the ready latch:
/// the device is restarting either way, so anything it reported before
/// belongs to an earlier attempt.
struct BmcReset;

impl BootControl for BmcReset {
    type Error = BoardFault;

    fn hold_in_reset(&mut self) -> Result<(), BoardFault> {
        DEVICE_READY.store(false, Ordering::Relaxed);
        drive_reset(RESET_ASSERT)
    }

    fn release(&mut self) -> Result<(), BoardFault> {
        DEVICE_READY.store(false, Ordering::Relaxed);
        RESETS.fetch_add(1, Ordering::Relaxed);
        drive_reset(RESET_RELEASE)
    }
}

/// Reads the ready latch. `EvidenceReader::read` is synchronous and may not
/// block, so the latch is the only shape that fits.
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

/// Signature checking is stubbed until the crypto service exists.
struct StubVerifier;

impl Verifier for StubVerifier {
    type Error = BoardFault;

    fn verify(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<Verdict, BoardFault> {
        Ok(Verdict::Authenticated { svn: Svn(1) })
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

/// The firmware device as the driver sees it. Staging is already done by
/// the time the RoT hears about it, because the device pulls its own
/// chunks from the agent, so `activate` is the one call that reaches it.
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

/// The staging region, as big as the candidate and carrying the bytes the
/// device is expected to have staged.
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

/// The RoT's own check of the staged candidate. A content check, because
/// signature checking belongs to the crypto service and is stubbed.
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

/// Reports go to the console, where the test reads them as an operator
/// would.
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
    type BootControl = BmcReset;
    type BootWatch = CheckpointWalk<BmcEvidence, u8>;
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
        boot_controls: [BmcReset],
        boot_watches: [CheckpointWalk::new(BmcEvidence, &DEVICES[0])],
        svn_floors: [SvnFloorBinding::SelfManaged],
        report_sink: LogSink,
        updatables: [FdUpdatable],
        recovery: [()],
        update_staging: PatternStaging,
        update_verifier: Some(PatternVerifier),
        update_stall_budget_millis: 5_000,
    }
}

/// Drains whatever the managed device has reported and latches a ready
/// report. Non-blocking: the caller has already waited.
fn drain_reports() {
    let mut evt = [0u8; 1];
    while let Ok(len) = syscall::channel_read(handle::BOOT_EVT, 0usize, &mut evt) {
        let _ = syscall::channel_respond(handle::BOOT_EVT, &[0u8; 0]);
        if len == 1usize && evt[0] == 1 {
            DEVICE_READY.store(true, Ordering::Relaxed);
        }
    }
}

#[entry]
fn entry() {
    pw_log::info!("ORCH: app started");

    if syscall::wait_group_add(handle::WG, handle::BOOT_EVT, Signals::READABLE, 0usize).is_err() {
        pw_log::error!("ORCH: wait group add failed");
        loop {}
    }

    // SAFETY: taken once, at this process's entry point, and handed
    // straight to the transport that owns them from here on.
    let send = unsafe { &mut *core::ptr::addr_of_mut!(SEND_BUF) };
    let recv = unsafe { &mut *core::ptr::addr_of_mut!(RECV_BUF) };
    let transport = AsyncChannelTransport::new(IpcHandle::new(handle::FD), send, recv);
    let mut fd = FdIpcClient::new(transport);

    let (mut core, mut driver) = bring_up::<UpdateBoard, N, E>(&CHAIN, board(), MAX_RETRY);

    // The RoT declares this run. Its claim is the ordering, and only it
    // sees the whole of it: the firmware device's flow ends at activation
    // and tells it nothing about the reset that has to follow.
    if run(&mut core, &mut driver, &mut fd) {
        pw_log::info!("ORCH: the device booted the image it was given");
        let _ = syscall::debug_shutdown(Ok(()));
    } else {
        pw_log::error!("ORCH: the run did not reach a committed update");
        let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
    }
    #[expect(clippy::empty_loop)]
    loop {}
}

/// The whole scenario, in the order the claim is about.
fn run(core: &mut Core, driver: &mut Driver, fd: &mut Fd) -> bool {
    // Power on. The driver releases the device, which arms its walk.
    core.dispatch(driver, Event::PowerGood(PowerOnResult::Provisioned));
    if !supervise_boot(core, driver, "first") {
        return false;
    }
    if core.state() != State::Ready {
        pw_log::error!("ORCH: not supervising after the first boot");
        return false;
    }

    let resets_before = RESETS.load(Ordering::Relaxed);

    if !drive_update(core, driver, fd) {
        return false;
    }

    // Activation only proposed the image. The state machine re-walks, and
    // that walk is what resets the device into what it just activated.
    let resets_after = RESETS.load(Ordering::Relaxed);
    if resets_after <= resets_before {
        pw_log::error!("ORCH: the device was never reset after the activation");
        return false;
    }

    if !supervise_boot(core, driver, "second") {
        return false;
    }

    // Only now has the image proved it can run.
    core.dispatch(driver, Event::BootConfirmed(TARGET));
    if core.state() != State::Ready {
        pw_log::error!("ORCH: the machine did not settle after the second boot");
        return false;
    }
    pw_log::info!("ORCH: update committed after the device booted it");
    true
}

/// Waits for the managed device's walk to complete.
///
/// The walk's verdict is the pass condition, not the machine's state: a
/// passive component is released speculatively, so `Ready` is reached
/// whether or not the device ever came up.
fn supervise_boot(core: &mut Core, driver: &mut Driver, which: &str) -> bool {
    let give_up = deadline_in(BOOT_BUDGET_MILLIS);

    loop {
        if core.state() == State::Locked {
            pw_log::error!("ORCH: the platform locked during a boot");
            return false;
        }

        let poll = driver.poll_boot_walks(now_millis());
        match poll.event {
            Some(Event::Booted(_)) | Some(Event::ComponentReady(_)) => {
                let event = poll.event.expect("just matched");
                core.dispatch(driver, event);
                pw_log::info!("ORCH: the device reported ready, {} boot", which as &str);
                return true;
            }
            Some(Event::BootFailed { checkpoint, .. }) => {
                pw_log::error!(
                    "ORCH: the device failed at checkpoint {}",
                    checkpoint as &str
                );
                core.dispatch(driver, poll.event.expect("just matched"));
                return false;
            }
            Some(event) => {
                core.dispatch(driver, event);
                continue;
            }
            None => {}
        }

        let deadline = match poll.next_deadline_millis {
            Some(millis) => Instant::from_ticks(millis * SystemClock::TICKS_PER_SEC / 1000),
            None => give_up,
        };
        match syscall::object_wait(handle::WG, Signals::READABLE, deadline) {
            Ok(_) => drain_reports(),
            Err(pw_status::Error::DeadlineExceeded) => {}
            Err(_) => return false,
        }

        if SystemClock::now() >= give_up {
            pw_log::error!("ORCH: gave up waiting for the device to boot");
            return false;
        }
    }
}

/// Drives one update, device step by device step, up to activation.
fn drive_update(core: &mut Core, driver: &mut Driver, fd: &mut Fd) -> bool {
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
            FdStatus::VerifyPending => {
                if !command(fd, |fd| fd.perform_verify()) {
                    return false;
                }
            }
            FdStatus::ApplyPending => {
                if !verified(core, driver, fd) {
                    return false;
                }
            }
            FdStatus::ActivationPending => {
                return command(fd, |fd| fd.perform_activate());
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
    if request_update(core, driver, TARGET, u64::from(total)).is_err() {
        pw_log::error!("ORCH: refused the update request");
        let _ = fd.reject_offer(RejectReason::PolicyViolation);
        let _ = settle(fd);
        return false;
    }
    if core.state() != State::Updating(TARGET) {
        pw_log::error!("ORCH: the request did not reach Updating");
        return false;
    }

    // Walk the job to authenticated. The device already holds the
    // candidate, so staging is a bookkeeping move, but the RoT's own
    // verifier still reads the staging region and activation refuses a
    // candidate that did not reach it.
    for _ in 0..8 {
        let poll = driver.pump_update(now_millis());
        match poll.event {
            Some(Event::UpdateVerified) => break,
            Some(event) => {
                pw_log::error!("ORCH: the update pump gave up on the job");
                core.dispatch(driver, event);
                return false;
            }
            None => {}
        }
    }

    if !ROT_AUTHENTICATED.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: the candidate never authenticated");
        return false;
    }

    pw_log::info!("ORCH: update accepted, {} bytes", total as u32);
    command(fd, |fd| fd.accept_offer(STAGING_BASE))
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
    command(fd, |fd| fd.perform_apply())
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
fn command(fd: &mut Fd, start: impl FnOnce(&mut Fd) -> Result<(), ClientError>) -> bool {
    if start(fd).is_err() {
        pw_log::error!("ORCH: could not send a command");
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

/// Waits for the answer to whatever is in flight. The deadline is what
/// turns a device that stopped serving into a log line rather than a stall.
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
