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
    bring_up, request_update, Board, BoardCapabilities, ImageSource, Report, ReportSink,
    SvnFloorBinding, Verdict, Verifier,
};
use openprot_orchestrator_sm::{ComponentAttrs, ComponentId, Event, PowerOnResult, State};
use orchestrator_capabilities::{
    BootControl, BootStatus, EvidenceReader, StageProgress, Svn, SvnFloor, Updatable, UpdateError,
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

/// Message opcodes from the firmware device.
const MSG_UPDATE_REQUESTED: u8 = 1;
const MSG_VERIFY_OUTCOME: u8 = 2;
const MSG_ACTIVATED: u8 = 3;

/// Answers to those messages.
const REPLY_ACCEPTED: u8 = 1;
const REPLY_REFUSED: u8 = 0;

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
        update_stall_budget_millis: 5_000,
    }
}

fn now_millis() -> u64 {
    SystemClock::now().ticks() * 1000 / SystemClock::TICKS_PER_SEC
}

#[entry]
fn entry() {
    pw_log::info!("ORCH: app started");

    if syscall::wait_group_add(handle::WG, handle::UPDATE_EVT, Signals::READABLE, 0usize).is_err() {
        pw_log::error!("ORCH: wait group add failed");
        loop {}
    }

    let (mut core, mut driver) = bring_up::<UpdateBoard, N, E>(&CHAIN, board(), MAX_RETRY);

    // Power on and verify, which is what puts the machine in Ready. An
    // update request arriving before that is refused, which is the point of
    // the check rather than a race to avoid.
    core.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    core.dispatch(&mut driver, Event::VerificationPassed(TARGET));
    pw_log::info!("ORCH: supervising, waiting for the firmware device");

    let mut request = [0u8; 8];
    loop {
        if syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX).is_err() {
            continue;
        }

        let len = match syscall::channel_read(handle::UPDATE_EVT, 0usize, &mut request) {
            Ok(len) => len,
            Err(_) => continue,
        };
        if len == 0 {
            let _ = syscall::channel_respond(handle::UPDATE_EVT, &[REPLY_REFUSED]);
            continue;
        }

        let reply = match request[0] {
            MSG_UPDATE_REQUESTED => {
                let candidate_len = if len >= 5 {
                    u32::from_le_bytes([request[1], request[2], request[3], request[4]]) as u64
                } else {
                    0
                };
                handle_update_requested(&mut core, &mut driver, candidate_len)
            }
            MSG_VERIFY_OUTCOME => {
                let good = len >= 2 && request[1] == 1;
                handle_verify_outcome(&mut core, &mut driver, good)
            }
            MSG_ACTIVATED => handle_activated(&mut core, &mut driver),
            other => {
                pw_log::error!("ORCH: unknown opcode {}", other as u32);
                REPLY_REFUSED
            }
        };

        let _ = syscall::channel_respond(handle::UPDATE_EVT, &[reply]);
    }
}

type Core = openprot_orchestrator_sm::Orchestrator<N, E>;
type Driver = openprot_orchestrator_driver::PlatformDriver<UpdateBoard, N>;

/// Records the job and dispatches the request. Refusing here keeps the
/// machine out of `Updating` on a job that could never finish.
fn handle_update_requested(core: &mut Core, driver: &mut Driver, candidate_len: u64) -> u8 {
    if request_update(core, driver, TARGET, candidate_len).is_err() {
        pw_log::error!("ORCH: refused the update request");
        return REPLY_REFUSED;
    }
    if core.state() != State::Updating(TARGET) {
        pw_log::error!("ORCH: the request did not reach Updating");
        return REPLY_REFUSED;
    }

    // Walk the job to Staged. The device already holds the candidate, so
    // each step is a bookkeeping move rather than a transfer, but the phase
    // still has to advance or `ActivateUpdate` refuses an unstaged image.
    for _ in 0..4 {
        let poll = driver.pump_update(now_millis());
        if let Some(event) = poll.event {
            pw_log::error!("ORCH: the update pump gave up on the job");
            core.dispatch(driver, event);
            return REPLY_REFUSED;
        }
    }

    pw_log::info!("ORCH: update accepted, {} bytes", candidate_len as u32);
    REPLY_ACCEPTED
}

/// Turns the device's verdict into the event that lets the machine leave
/// `Updating`. PR #515 proposes the same mapping as a function on the PLDM
/// adapter; it is two lines here until that lands.
fn handle_verify_outcome(core: &mut Core, driver: &mut Driver, good: bool) -> u8 {
    if core.state() != State::Updating(TARGET) {
        pw_log::error!("ORCH: a verdict arrived with no update in flight");
        return REPLY_REFUSED;
    }

    let event = if good {
        Event::UpdateVerified
    } else {
        Event::UpdateRejected
    };
    core.dispatch(driver, event);

    if !good {
        pw_log::info!("ORCH: the device rejected the candidate, nothing activated");
        return REPLY_ACCEPTED;
    }
    if !ACTIVATED.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: the verdict did not reach the device");
        return REPLY_REFUSED;
    }

    pw_log::info!("ORCH: activated, awaiting the device's first boot");
    REPLY_ACCEPTED
}

/// The updated device came back. The floor advances here and nowhere
/// earlier: activation proposes an image, a healthy boot is what commits it.
fn handle_activated(core: &mut Core, driver: &mut Driver) -> u8 {
    // Nothing was activated, so there is no image whose first boot this
    // could be. Confirming one anyway would let a device that never got
    // past the gate report itself healthy.
    if !ACTIVATED.load(Ordering::Relaxed) {
        pw_log::error!("ORCH: a boot was reported for an update that never activated");
        return REPLY_REFUSED;
    }
    core.dispatch(driver, Event::BootConfirmed(TARGET));
    if core.state() != State::Ready {
        pw_log::error!("ORCH: the machine did not settle in Ready");
        return REPLY_REFUSED;
    }
    pw_log::info!("ORCH: update committed");
    REPLY_ACCEPTED
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
