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

use openprot_orchestrator_driver::{
    bring_up, settle_self_image_at_boot, Board, BoardCapabilities, ImageSource, Report, ReportSink,
    SelfImage, SvnFloorBinding, Verdict, Verifier,
};
use openprot_orchestrator_sm::{ComponentAttrs, ComponentId, Event, PowerOnResult, State};
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

/// One board fault, for every seam that has to name an error type. The
/// scenario cares whether an actuation failed, never which one.
#[derive(Debug)]
struct BoardFault;

impl core::fmt::Display for BoardFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("board fault")
    }
}

impl core::error::Error for BoardFault {}

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

/// Signature checking is stubbed until the crypto service exists, so every
/// image authenticates. The SVN is a fixed value: nothing in a boot
/// scenario moves the floor.
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

/// A fixed-size image of zeros. The stub verifier never reads it; it exists
/// because the board has to name an image for every component.
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

/// The managed device keeps its own floor, so the eRoT holds none for it.
/// The eRoT's own floor is a different thing and is held here, in RAM: this
/// scenario never resets the eRoT, so nothing has to survive a reset.
struct ErotFloor;

/// The eRoT's own anti-rollback floor. Starts at zero: no scenario here
/// advances it, and a session that recorded an SVN still has the advance
/// ahead of it.
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
        // session's. No scenario here activates an image for the eRoT, so
        // this boot is always running the confirmed one.
        Ok(RunningImage::Confirmed)
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
struct NoUpdate;

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
struct NeverVerifier;

struct NeverSession;

impl IncrementalVerifier for NeverVerifier {
    type Error = BoardFault;
    type Session = NeverSession;

    fn start(self) -> NeverSession {
        NeverSession
    }
}

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

/// No staging region, because nothing stages here.
struct NoStaging;

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

struct MockDownstreamDeviceBoard;

impl BoardCapabilities for MockDownstreamDeviceBoard {
    type Image = StubImage;
    type Verifier = StubVerifier;
    type BootControl = BmcReset;
    type BootWatch = CheckpointWalk<BmcEvidence, u8>;
    type SvnFloor = ErotFloor;
    type ReportSink = LogSink;
    type Updatable = NoUpdate;
    type Recovery = ();
    type Staging = NoStaging;
    type UpdateVerifier = NeverVerifier;
    type SelfUpdate = SelfSession;
}

fn board() -> Board<MockDownstreamDeviceBoard, N> {
    Board {
        // Index 0 is the eRoT. The walk passes over it, so nothing reads
        // or actuates these entries. The arrays are indexed by ComponentId,
        // so index 0 still needs a value.
        images: [StubImage, StubImage],
        verifier: StubVerifier,
        boot_controls: [BmcReset, BmcReset],
        boot_watches: [
            CheckpointWalk::new(BmcEvidence, &DEVICES[0]),
            CheckpointWalk::new(BmcEvidence, &DEVICES[1]),
        ],
        svn_floors: [SvnFloorBinding::SelfManaged, SvnFloorBinding::SelfManaged],
        report_sink: LogSink,
        updatables: [NoUpdate, NoUpdate],
        recovery: [(), ()],
        update_staging: NoStaging,
        update_verifier: Some(NeverVerifier),
        self_image: Some(SelfImage {
            session: SelfSession,
            floor: ErotFloor,
        }),
        update_stall_budget_millis: 1_000,
    }
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

    let (mut core, mut driver) =
        bring_up::<MockDownstreamDeviceBoard, N, E>(&CHAIN, board(), MAX_RETRY);

    // What the eRoT's last self-update was doing, read before anything
    // walks. A trial running right now has to be declared here, because
    // what judges it is this walk reaching `Ready`.
    if settle_self_image_at_boot(&mut core, &mut driver).is_err() {
        pw_log::error!("the eRoT's own session could not be settled");
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
            pw_log::info!("device booted and the orchestrator is in Ready");
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
