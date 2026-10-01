// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use crate::*;
use openprot_orchestrator_sm::{
    BootFailureKind, ComponentAttrs, ComponentId, ComponentKind, Effect, Event, Orchestrator,
    Platform, PowerOnResult, State,
};
use orchestrator_capabilities::{
    BootWatch, FailureCause, Progress, Recovery, RestoreOutcome, Svn, SvnFloor, WalkVerdict,
};
use util_io::{ByteReadError, ByteSource};

const C0: ComponentId = ComponentId::new(0);

// Test image convention, shared with the fwmanager itests: 4 magic bytes,
// payload, final byte makes the XOR over the image zero. A board-side
// stand-in for signature + SVN verification.
const IMAGE_MAGIC: [u8; 4] = *b"OPRT";
const IMAGE_LEN: usize = 16;

fn valid_image() -> std::vec::Vec<u8> {
    let mut image = std::vec![0u8; IMAGE_LEN];
    image[..4].copy_from_slice(&IMAGE_MAGIC);
    image[4..IMAGE_LEN - 1].fill(0xAB);
    image[IMAGE_LEN - 1] = image[..IMAGE_LEN - 1].iter().fold(0, |acc, b| acc ^ b);
    image
}

#[derive(Debug)]
struct MemFault;

impl core::fmt::Display for MemFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("mem image fault")
    }
}

impl core::error::Error for MemFault {}

/// RAM-backed image source — the seam satisfied without a HAL. A re-flash
/// can be queued: it lands on the first re-open (a re-stage), the way real
/// flash changes underneath a source between stagings — never on the
/// initial staging.
struct MemImage {
    data: std::vec::Vec<u8>,
    reflash: Option<std::vec::Vec<u8>>,
    opened: bool,
    fail_open: bool,
    fail_read: bool,
}

impl MemImage {
    fn holding(data: std::vec::Vec<u8>) -> Self {
        Self {
            data,
            reflash: None,
            opened: false,
            fail_open: false,
            fail_read: false,
        }
    }

    /// Queue a re-flash: the first re-open stages `data` instead.
    fn reflash_on_reopen(mut self, data: std::vec::Vec<u8>) -> Self {
        self.reflash = Some(data);
        self
    }
}

impl ImageSource for MemImage {
    type Error = MemFault;

    fn open(&mut self) -> Result<(), MemFault> {
        if self.fail_open {
            return Err(MemFault);
        }
        if let Some(data) = self.reflash.take_if(|_| self.opened) {
            self.data = data;
        }
        self.opened = true;
        Ok(())
    }

    fn size(&self) -> Result<usize, MemFault> {
        Ok(self.data.len())
    }

    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), MemFault> {
        if self.fail_read {
            return Err(MemFault);
        }
        buf.copy_from_slice(&self.data[offset..offset + buf.len()]);
        Ok(())
    }
}

#[derive(Debug)]
struct VerifierError;

impl core::fmt::Display for VerifierError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("verifier broken")
    }
}

impl core::error::Error for VerifierError {}

/// The magic + XOR-zero check as a board-supplied verifier, reading in
/// chunks.
struct XorVerifier {
    fault: bool,
    /// The manifest SVN this verifier reports for an authenticated image.
    /// A real verifier reads it from the signed manifest of the image it
    /// just checked; the test image format has no manifest, so tests pin it.
    svn: u32,
}

impl Verifier for XorVerifier {
    type Error = VerifierError;

    fn verify(
        &mut self,
        _id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, VerifierError> {
        if self.fault {
            return Err(VerifierError);
        }
        let len = image.size().map_err(|_| VerifierError)?;
        let mut magic = [0u8; 4];
        let mut xor = 0u8;
        let mut offset = 0;
        let mut chunk = [0u8; 4];
        while offset < len {
            let take = chunk.len().min(len - offset);
            image
                .read_at(offset, &mut chunk[..take])
                .map_err(|_| VerifierError)?;
            if offset == 0 && take >= 4 {
                magic.copy_from_slice(&chunk[..4]);
            }
            xor = chunk[..take].iter().fold(xor, |acc, b| acc ^ b);
            offset += take;
        }
        let ok = len > IMAGE_MAGIC.len() && magic == IMAGE_MAGIC && xor == 0;
        Ok(if ok {
            Verdict::Authenticated { svn: Svn(self.svn) }
        } else {
            Verdict::Rejected
        })
    }
}

#[derive(Debug)]
struct ResetFault;

impl core::fmt::Display for ResetFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("reset line fault")
    }
}

impl core::error::Error for ResetFault {}

/// Reset actuation without a HAL; `held` is shared so tests can observe the
/// line after the control moves into the driver.
struct MockReset {
    held: std::rc::Rc<core::cell::Cell<bool>>,
    fail: bool,
}

impl MockReset {
    fn new() -> Self {
        Self {
            held: std::rc::Rc::new(core::cell::Cell::new(true)),
            fail: false,
        }
    }
}

impl orchestrator_capabilities::BootControl for MockReset {
    type Error = ResetFault;

    fn hold_in_reset(&mut self) -> Result<(), ResetFault> {
        if self.fail {
            return Err(ResetFault);
        }
        self.held.set(true);
        Ok(())
    }

    fn release(&mut self) -> Result<(), ResetFault> {
        if self.fail {
            return Err(ResetFault);
        }
        self.held.set(false);
        Ok(())
    }
}

/// Boot walk without a device; scripted verdicts. An exhausted script
/// holds its last verdict; an empty script waits forever. `arm` rewinds
/// to the script start, so a fresh attempt is observable from the
/// verdicts alone — no poll or arm counters needed.
struct MockWalk {
    verdicts: std::vec::Vec<WalkVerdict>,
    next: usize,
}

const IDLE_DEADLINE: u64 = 60_000;

impl MockWalk {
    fn scripted(verdicts: std::vec::Vec<WalkVerdict>) -> Self {
        Self { verdicts, next: 0 }
    }

    /// A walk that reports "still waiting" forever.
    fn idle() -> Self {
        Self::scripted(std::vec::Vec::new())
    }
}

impl BootWatch for MockWalk {
    fn arm(&mut self) {
        self.next = 0;
    }

    fn poll(&mut self, _now_millis: u64) -> WalkVerdict {
        match self.verdicts.get(self.next) {
            Some(v) => {
                self.next += 1;
                *v
            }
            // Exhausted: repeat the last verdict, like a real finished
            // walk. A driver bug that re-polls one then shows up as a
            // duplicate event in the exactly-once assertions instead of
            // panicking here.
            None => self
                .verdicts
                .last()
                .copied()
                .unwrap_or(WalkVerdict::Waiting {
                    deadline_millis: IDLE_DEADLINE,
                }),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FloorFaultInjected;

impl core::fmt::Display for FloorFaultInjected {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("floor fault injected")
    }
}

impl core::error::Error for FloorFaultInjected {}

/// An SVN floor without storage; tests read it back through the
/// capability's own `floor()` via `PlatformDriver::board`.
struct MockFloor {
    floor: u32,
    fail: bool,
}

impl MockFloor {
    fn new() -> Self {
        Self {
            floor: 0,
            fail: false,
        }
    }
}

/// Chain entries for an all-passive chain of `N` components, which is what
/// most of these tests want. Ids are positions, as the driver requires.
fn passive_entries<const N: usize>() -> [(ComponentId, ComponentAttrs); N] {
    core::array::from_fn(|i| {
        (
            ComponentId::new(i as u8),
            ComponentAttrs::passive_required(),
        )
    })
}

/// The same, with the kinds a test cares about.
fn entries_with_kinds<const N: usize>(
    kinds: [ComponentKind; N],
) -> [(ComponentId, ComponentAttrs); N] {
    core::array::from_fn(|i| {
        let mut attrs = ComponentAttrs::passive_required();
        attrs.kind = kinds[i];
        (ComponentId::new(i as u8), attrs)
    })
}

/// Update adapter without a HAL. Wiring-only for now: it stages the whole
/// payload in one step. The update pump replaces it with a stepping mock
/// when the executors land.
struct MockUpdatable {
    ready: bool,
    active: bool,
    abandons: usize,
    /// Transferring steps before the device reports Ready.
    steps: usize,
    written: u64,
    /// Answers Transferring forever without moving `written`: the stall
    /// the pump's budget exists for.
    stalls: bool,
    /// Fails every step.
    faults: bool,
}

/// Bytes one staging step writes.
const STAGE_CHUNK: u64 = 8;

impl MockUpdatable {
    /// Reports Ready on the first step.
    fn new() -> Self {
        Self {
            ready: false,
            active: false,
            abandons: 0,
            steps: 0,
            written: 0,
            stalls: false,
            faults: false,
        }
    }

    /// Transfers in `steps` steps before reporting Ready.
    fn stepping(steps: usize) -> Self {
        Self {
            steps,
            ..Self::new()
        }
    }

    fn stalling() -> Self {
        Self {
            stalls: true,
            ..Self::new()
        }
    }

    fn faulting() -> Self {
        Self {
            faults: true,
            ..Self::new()
        }
    }
}

/// The staging region: a fixed buffer an update source would have
/// written into.
struct MemStaging {
    bytes: [u8; STAGING_LEN],
}

/// Room for the candidate plus slack, so a test can tell the region's
/// length from the candidate's.
const STAGING_LEN: usize = 64;

/// What `mock_board` declares as its candidate length: shorter than the
/// region, so the job carries the actual occupied range.
const CANDIDATE_LEN: u64 = 32;

impl MemStaging {
    fn new() -> Self {
        Self {
            bytes: core::array::from_fn(|i| i as u8),
        }
    }
}

impl ByteSource for MemStaging {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
        let start = offset as usize;
        let end = start
            .checked_add(buf.len())
            .ok_or(ByteReadError::OutOfRange)?;
        if end > self.bytes.len() {
            return Err(ByteReadError::OutOfRange);
        }
        buf.copy_from_slice(&self.bytes[start..end]);
        Ok(())
    }
}

impl orchestrator_capabilities::SvnFloor for MockFloor {
    type Error = FloorFaultInjected;

    fn floor(&self) -> Result<Svn, FloorFaultInjected> {
        if self.fail {
            return Err(FloorFaultInjected);
        }
        Ok(Svn(self.floor))
    }

    fn advance(&mut self, to: Svn) -> Result<(), FloorFaultInjected> {
        if self.fail {
            return Err(FloorFaultInjected);
        }
        self.floor = self.floor.max(to.0);
        Ok(())
    }
}

impl orchestrator_capabilities::Updatable for MockUpdatable {
    fn poll_stage(
        &mut self,
        payload: &dyn util_io::ByteSource,
    ) -> Result<orchestrator_capabilities::StageProgress, orchestrator_capabilities::UpdateError>
    {
        if self.faults {
            return Err(orchestrator_capabilities::UpdateError::Device);
        }
        let total = payload.len();
        if self.stalls {
            return Ok(orchestrator_capabilities::StageProgress::Transferring {
                progress: Progress {
                    written: self.written,
                    total,
                },
            });
        }
        if self.steps > 0 {
            self.steps -= 1;
            self.written = (self.written + STAGE_CHUNK).min(total);
            return Ok(orchestrator_capabilities::StageProgress::Transferring {
                progress: Progress {
                    written: self.written,
                    total,
                },
            });
        }
        self.ready = true;
        Ok(orchestrator_capabilities::StageProgress::Ready)
    }

    fn abandon(&mut self) {
        self.ready = false;
        self.abandons += 1;
    }

    fn activate(&mut self) -> Result<(), orchestrator_capabilities::UpdateError> {
        if !self.ready {
            return Err(orchestrator_capabilities::UpdateError::NothingStaged);
        }
        self.active = true;
        Ok(())
    }
}

/// The test board's type choices.
struct MockBoard;

impl BoardCapabilities for MockBoard {
    type Image = MemImage;
    type Verifier = XorVerifier;
    type BootControl = MockReset;
    type BootWatch = MockWalk;
    type SvnFloor = MockFloor;
    type ReportSink = RecordingSink;
    type Updatable = MockUpdatable;
    type Recovery = ();
    type Staging = MemStaging;
}

/// The SVN `mock_board`'s verifier vouches for. Tests that read the floor
/// back assert against it.
const MOCK_SVN: u32 = 5;

/// Happy-path wiring for `N` components. Tests override the one field
/// they exercise with `..mock_board()`.
/// How long a job may make no progress before the pump abandons it.
const STALL_BUDGET_MILLIS: u64 = 1_000;

fn mock_board<const N: usize>() -> Board<MockBoard, N> {
    Board {
        images: core::array::from_fn(|_| MemImage::holding(valid_image())),
        verifier: XorVerifier {
            fault: false,
            svn: MOCK_SVN,
        },
        boot_controls: core::array::from_fn(|_| MockReset::new()),
        boot_watches: core::array::from_fn(|_| MockWalk::idle()),
        svn_floors: core::array::from_fn(|_| SvnFloorBinding::Erot(MockFloor::new())),
        report_sink: RecordingSink::new(),
        updatables: core::array::from_fn(|_| MockUpdatable::new()),
        recovery: core::array::from_fn(|_| ()),
        update_staging: MemStaging::new(),
        update_stall_budget_millis: STALL_BUDGET_MILLIS,
    }
}

fn driver(images: [MemImage; 1]) -> PlatformDriver<MockBoard, 1> {
    PlatformDriver::new(
        &passive_entries(),
        Board {
            images,
            ..mock_board()
        },
    )
}

fn orchestrator() -> Orchestrator<1, 4> {
    let mut chain = heapless::Vec::<_, 1>::new();
    chain
        .push((C0, ComponentAttrs::passive_required()))
        .unwrap();
    Orchestrator::new(chain.try_into().unwrap(), 3)
}

// PowerGood drives ReadFirmware + VerifyFirmware into the driver; the
// verdict returns through execute and the SM settles it in the same
// dispatch run, carrying a passive component all the way to Ready.
#[test]
fn boot_verifies_the_first_component() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Ready);
}

#[test]
fn corrupt_image_fails_verification() {
    let mut corrupt = valid_image();
    corrupt[7] ^= 0x01;
    let mut driver = driver([MemImage::holding(corrupt)]);

    driver.stage_firmware(C0).unwrap();

    assert_eq!(
        driver.verify_firmware(C0),
        Ok(Event::VerificationFailed(C0))
    );
}

#[test]
fn verify_without_read_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.verify_firmware(C0), Err(DriverError::NotStaged));
}

// An unopenable source is a failed actuation, not a verdict: the SM
// latches Locked instead of getting a forged VerificationFailed.
#[test]
fn unopenable_source_fails_closed() {
    let mut orch = orchestrator();
    let mut image = MemImage::holding(valid_image());
    image.fail_open = true;
    let mut driver = driver([image]);

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
}

// A source that opens but cannot be read fails the same way, via the
// verifier's error.
#[test]
fn unreadable_source_fails_closed() {
    let mut orch = orchestrator();
    let mut image = MemImage::holding(valid_image());
    image.fail_read = true;
    let mut driver = driver([image]);

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
}

// So does a verifier that cannot run its check.
#[test]
fn verifier_fault_fails_closed() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            verifier: XorVerifier {
                fault: true,
                svn: MOCK_SVN,
            },
            ..mock_board()
        },
    );

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
}

const C1: ComponentId = ComponentId::new(1);

#[test]
fn verify_for_a_different_component_is_refused() {
    let mut driver = PlatformDriver::<MockBoard, 2>::new(&passive_entries(), mock_board());

    driver.stage_firmware(C0).unwrap();

    assert_eq!(driver.verify_firmware(C1), Err(DriverError::NotStaged));
}

#[test]
fn unknown_component_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.stage_firmware(ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );
}

// An unknown id is reported as such even though it is also never staged.
#[test]
fn verify_of_unknown_component_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.verify_firmware(ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );
}

#[test]
fn reset_release_and_assert_reach_the_boot_control() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(&passive_entries(), mock_board());

    driver.release_reset(C0).unwrap();
    assert!(!driver.board().boot_controls[0].held.get());

    driver.assert_reset(C0).unwrap();
    assert!(driver.board().boot_controls[0].held.get());
}

#[test]
fn reset_of_unknown_component_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.release_reset(ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );
    assert_eq!(
        driver.assert_reset(ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );
}

#[test]
fn reset_line_fault_is_reported() {
    let mut control = MockReset::new();
    control.fail = true;
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            boot_controls: [control],
            ..mock_board()
        },
    );

    assert_eq!(driver.release_reset(C0), Err(DriverError::BootControlFault));
    assert_eq!(driver.assert_reset(C0), Err(DriverError::BootControlFault));
}

// Effect::Emit is the orchestrator's internal channel and must never reach
// a Platform; the driver refuses it rather than acting on it.
#[test]
fn emit_is_refused() {
    use openprot_orchestrator_sm::{Effect, EffectError, Platform};

    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.execute(Effect::Emit(Event::UpdateRequest(C0))),
        Err(EffectError)
    );
}

// The verdict is the returned event of the VerifyFirmware effect.
#[test]
fn execute_returns_the_verdict_event() {
    use openprot_orchestrator_sm::{Effect, Platform};

    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.execute(Effect::ReadFirmware(C0)), Ok(None));
    assert_eq!(
        driver.execute(Effect::VerifyFirmware(C0)),
        Ok(Some(Event::VerificationPassed(C0)))
    );
}

/// Wraps [`XorVerifier`] and snapshots the reset line as the check runs,
/// so the test can see the line state inside the verification window.
struct LineWatchingVerifier {
    inner: XorVerifier,
    line: std::rc::Rc<core::cell::Cell<bool>>,
    held_during_verify: std::rc::Rc<core::cell::Cell<bool>>,
}

impl Verifier for LineWatchingVerifier {
    type Error = VerifierError;

    fn verify(
        &mut self,
        id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, VerifierError> {
        self.held_during_verify.set(self.line.get());
        self.inner.verify(id, image)
    }
}

struct WatchBoard;

impl BoardCapabilities for WatchBoard {
    type Image = MemImage;
    type Verifier = LineWatchingVerifier;
    type BootControl = MockReset;
    type BootWatch = MockWalk;
    type SvnFloor = MockFloor;
    // A board with nothing to tell: exercises the no-op sink.
    type ReportSink = ();
    type Updatable = MockUpdatable;
    type Recovery = ();
    type Staging = MemStaging;
}

// The at-rest guarantee end to end: the component is still held while its
// image is verified, and the line is released only on the passing verdict.
#[test]
fn release_follows_verification() {
    let control = MockReset::new();
    let held = control.held.clone();
    let held_during_verify = std::rc::Rc::new(core::cell::Cell::new(false));
    let mut driver = PlatformDriver::<WatchBoard, 1>::new(
        &passive_entries(),
        Board {
            images: [MemImage::holding(valid_image())],
            verifier: LineWatchingVerifier {
                inner: XorVerifier {
                    fault: false,
                    svn: MOCK_SVN,
                },
                line: held.clone(),
                held_during_verify: held_during_verify.clone(),
            },
            boot_controls: [control],
            boot_watches: [MockWalk::idle()],
            svn_floors: [SvnFloorBinding::Erot(MockFloor::new())],
            report_sink: (),
            updatables: [MockUpdatable::new()],
            recovery: [()],
            update_staging: MemStaging::new(),
            update_stall_budget_millis: STALL_BUDGET_MILLIS,
        },
    );
    let mut orch = orchestrator();

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Ready);
    assert!(
        held_during_verify.get(),
        "held while its image was verified"
    );
    assert!(!held.get(), "released after the verdict");
}

// A dead reset line is a failed actuation, not a verdict: the SM fails
// closed and the component stays quiesced.
#[test]
fn failed_release_fails_closed() {
    let mut control = MockReset::new();
    control.fail = true;
    let held = control.held.clone();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            boot_controls: [control],
            ..mock_board()
        },
    );
    let mut orch = orchestrator();

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
    assert!(held.get(), "never left reset");
}

// ---------------------------------------------------------------------------
// Boot-walk supervision.
// ---------------------------------------------------------------------------

/// A 2-component driver with per-component scripted walks and kinds;
/// everything else is the happy-path mock.
fn walk_driver(
    walks: [MockWalk; 2],
    component_kinds: [ComponentKind; 2],
) -> PlatformDriver<MockBoard, 2> {
    PlatformDriver::new(
        &entries_with_kinds(component_kinds),
        Board {
            boot_watches: walks,
            ..mock_board()
        },
    )
}

// A completed walk becomes ComponentReady for Active, Booted for Passive.
// One event per call, drained in index order; a finished walk never
// reports twice.
#[test]
fn completed_walks_report_by_kind() {
    let mut driver = walk_driver(
        [
            MockWalk::scripted(std::vec![WalkVerdict::Complete]),
            MockWalk::scripted(std::vec![WalkVerdict::Complete]),
        ],
        [ComponentKind::Active, ComponentKind::Passive],
    );
    driver.release_reset(C0).unwrap();
    driver.release_reset(C1).unwrap();

    assert_eq!(
        driver.poll_boot_walks(0).event,
        Some(Event::ComponentReady(C0))
    );
    assert_eq!(driver.poll_boot_walks(0).event, Some(Event::Booted(C1)));

    let quiet = driver.poll_boot_walks(0);
    assert_eq!(quiet.event, None, "verdicts are delivered exactly once");
    assert_eq!(quiet.next_deadline_millis, None, "no walk left waiting");
}

// A failed walk becomes BootFailed, preserving the checkpoint name and
// classified cause so the SM can differentiate retry decisions later.
#[test]
fn failed_walks_map_to_boot_failed() {
    let mut driver = walk_driver(
        [
            MockWalk::scripted(std::vec![WalkVerdict::Failed {
                checkpoint: "heartbeat",
                cause: FailureCause::TimedOut,
            }]),
            MockWalk::scripted(std::vec![WalkVerdict::Failed {
                checkpoint: "self-test",
                cause: FailureCause::DeviceFatal,
            }]),
        ],
        [ComponentKind::Active, ComponentKind::Passive],
    );
    driver.release_reset(C0).unwrap();
    driver.release_reset(C1).unwrap();

    assert_eq!(
        driver.poll_boot_walks(0).event,
        Some(Event::BootFailed {
            id: C0,
            checkpoint: "heartbeat",
            kind: BootFailureKind::TimedOut,
        })
    );
    assert_eq!(
        driver.poll_boot_walks(0).event,
        Some(Event::BootFailed {
            id: C1,
            checkpoint: "self-test",
            kind: BootFailureKind::DeviceFatal,
        })
    );
    assert_eq!(driver.poll_boot_walks(0).event, None);
}

// An event-carrying poll returns before visiting later walks, so its
// deadline is partial and must not be trusted; the drain's final,
// event-free poll visits every remaining walk and reports the earliest
// deadline.
#[test]
fn deadline_is_authoritative_only_when_no_event() {
    let mut driver = walk_driver(
        [
            MockWalk::scripted(std::vec![WalkVerdict::Complete]),
            MockWalk::scripted(std::vec![WalkVerdict::Waiting {
                deadline_millis: 1_000,
            }]),
        ],
        [ComponentKind::Passive, ComponentKind::Passive],
    );
    driver.release_reset(C0).unwrap();
    driver.release_reset(C1).unwrap();

    let first = driver.poll_boot_walks(0);
    assert_eq!(first.event, Some(Event::Booted(C0)));
    assert_eq!(
        first.next_deadline_millis, None,
        "returned before the waiting walk was visited"
    );

    let last = driver.poll_boot_walks(0);
    assert_eq!(last.event, None);
    assert_eq!(last.next_deadline_millis, Some(1_000));
}

// While every watched walk waits, the poll carries the earliest deadline
// as the run loop's next wake-up.
#[test]
fn waiting_walks_report_the_earliest_deadline() {
    let mut driver = walk_driver(
        [
            MockWalk::scripted(std::vec![WalkVerdict::Waiting {
                deadline_millis: 9_000,
            }]),
            MockWalk::scripted(std::vec![WalkVerdict::Waiting {
                deadline_millis: 4_000,
            }]),
        ],
        [ComponentKind::Passive, ComponentKind::Passive],
    );
    driver.release_reset(C0).unwrap();
    driver.release_reset(C1).unwrap();

    let poll = driver.poll_boot_walks(0);
    assert_eq!(poll.event, None);
    assert_eq!(poll.next_deadline_millis, Some(4_000));
}

// A walk is watched only between release and terminal verdict. The script's
// terminal verdict would surface as an event if the gate were missing: no
// event before release, no stale event after assert_reset, and the verdict
// still arrives once the device is actually released.
#[test]
fn only_released_components_are_watched() {
    let mut driver = walk_driver(
        [
            MockWalk::scripted(std::vec![WalkVerdict::Complete]),
            MockWalk::idle(),
        ],
        [ComponentKind::Passive, ComponentKind::Passive],
    );

    assert_eq!(
        driver.poll_boot_walks(0).event,
        None,
        "unreleased: no event"
    );

    driver.release_reset(C0).unwrap();
    driver.assert_reset(C0).unwrap();
    assert_eq!(
        driver.poll_boot_walks(0).event,
        None,
        "back in reset: no stale event"
    );

    driver.release_reset(C0).unwrap();
    assert_eq!(driver.poll_boot_walks(0).event, Some(Event::Booted(C0)));
}

// Every release re-arms the walk: a retry judges a new attempt from the
// first checkpoint, not the failed one resumed. With a script of
// [Failed, Complete], a resumed walk would report Complete on the second
// attempt; a fresh one reports Failed again.
#[test]
fn rerelease_arms_a_fresh_walk() {
    let mut driver = walk_driver(
        [
            MockWalk::scripted(std::vec![
                WalkVerdict::Failed {
                    checkpoint: "heartbeat",
                    cause: FailureCause::TimedOut,
                },
                WalkVerdict::Complete,
            ]),
            MockWalk::idle(),
        ],
        [ComponentKind::Passive, ComponentKind::Passive],
    );

    driver.release_reset(C0).unwrap();
    assert_eq!(
        driver.poll_boot_walks(0).event,
        Some(Event::BootFailed {
            id: C0,
            checkpoint: "heartbeat",
            kind: BootFailureKind::TimedOut,
        })
    );

    driver.release_reset(C0).unwrap();
    assert_eq!(
        driver.poll_boot_walks(0).event,
        Some(Event::BootFailed {
            id: C0,
            checkpoint: "heartbeat",
            kind: BootFailureKind::TimedOut,
        }),
        "fresh attempt from the first checkpoint, not the old walk resumed"
    );
}

// End to end: a passive component is released speculatively (Ready), its
// walk completes, and the Booted event settles cleanly.
#[test]
fn booted_walk_settles_in_ready() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            boot_watches: [MockWalk::scripted(std::vec![WalkVerdict::Complete])],
            ..mock_board()
        },
    );

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    let event = driver.poll_boot_walks(0).event.expect("walk completed");
    assert_eq!(event, Event::Booted(C0));
    orch.dispatch(&mut driver, event);

    assert_eq!(orch.state(), State::Ready);
}

// End to end, failure path: the released component's walk fails, its
// BootFailed enters recovery. The `()` recovery reports source
// exhaustion immediately, and a Required component locks down.
#[test]
fn boot_failure_locks_when_recovery_is_exhausted() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            boot_watches: [MockWalk::scripted(std::vec![WalkVerdict::Failed {
                checkpoint: "heartbeat",
                cause: FailureCause::TimedOut,
            }])],
            ..mock_board()
        },
    );

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    let event = driver.poll_boot_walks(0).event.expect("walk failed");
    assert_eq!(
        event,
        Event::BootFailed {
            id: C0,
            checkpoint: "heartbeat",
            kind: BootFailureKind::TimedOut,
        }
    );
    orch.dispatch(&mut driver, event);

    assert_eq!(orch.state(), State::Locked);
    assert!(
        driver
            .board()
            .report_sink
            .seen
            .contains(&Report::RecoveryFailed(C0)),
        "lock came via exhaust_recovery, not a raw actuation fault"
    );
}

// ---------------------------------------------------------------------------
// Recovery executor.
// ---------------------------------------------------------------------------

struct MockRecovery {
    sources: u8,
    fail_on: Option<u8>,
}

#[derive(Debug)]
struct RecoveryActuationFault;

impl core::fmt::Display for RecoveryActuationFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("recovery actuation fault")
    }
}

impl core::error::Error for RecoveryActuationFault {}

impl Recovery for MockRecovery {
    type Error = RecoveryActuationFault;

    fn restore(&mut self, attempt: u8) -> Result<RestoreOutcome, Self::Error> {
        if self.fail_on == Some(attempt) {
            return Err(RecoveryActuationFault);
        }
        if attempt >= self.sources {
            return Ok(RestoreOutcome::SourceExhausted);
        }
        Ok(RestoreOutcome::Restored)
    }
}

struct RecoverableBoard;

impl BoardCapabilities for RecoverableBoard {
    type Image = MemImage;
    type Verifier = XorVerifier;
    type BootControl = MockReset;
    type BootWatch = MockWalk;
    type SvnFloor = MockFloor;
    type ReportSink = RecordingSink;
    type Updatable = MockUpdatable;
    type Recovery = MockRecovery;
    type Staging = MemStaging;
}

fn recoverable_board<const N: usize>(sources: u8) -> Board<RecoverableBoard, N> {
    Board {
        images: core::array::from_fn(|_| MemImage::holding(valid_image())),
        verifier: XorVerifier {
            fault: false,
            svn: MOCK_SVN,
        },
        boot_controls: core::array::from_fn(|_| MockReset::new()),
        boot_watches: core::array::from_fn(|_| MockWalk::idle()),
        svn_floors: core::array::from_fn(|_| SvnFloorBinding::Erot(MockFloor::new())),
        report_sink: RecordingSink::new(),
        updatables: core::array::from_fn(|_| MockUpdatable::new()),
        recovery: core::array::from_fn(|_| MockRecovery {
            sources,
            fail_on: None,
        }),
        update_staging: MemStaging::new(),
        update_stall_budget_millis: STALL_BUDGET_MILLIS,
    }
}

// RecoverComponent with a successful restore returns Restored(id).
#[test]
fn recover_component_returns_restored() {
    let mut driver =
        PlatformDriver::<RecoverableBoard, 1>::new(&passive_entries(), recoverable_board(2));

    assert_eq!(driver.recover_component(C0, 0), Ok(Event::Restored(C0)));
    assert_eq!(driver.recover_component(C0, 1), Ok(Event::Restored(C0)));
}

// RecoverComponent past the last source returns RecoveryUnavailable(id).
#[test]
fn recover_component_returns_unavailable_when_exhausted() {
    let mut driver =
        PlatformDriver::<RecoverableBoard, 1>::new(&passive_entries(), recoverable_board(1));

    assert_eq!(driver.recover_component(C0, 0), Ok(Event::Restored(C0)));
    assert_eq!(
        driver.recover_component(C0, 1),
        Ok(Event::RecoveryUnavailable(C0))
    );
}

// A faulting mechanism returns RecoveryFault; the next attempt still
// succeeds, so a fault does not poison the path.
#[test]
fn recovery_fault_is_reported() {
    let mut driver = PlatformDriver::<RecoverableBoard, 1>::new(
        &passive_entries(),
        Board {
            recovery: [MockRecovery {
                sources: 2,
                fail_on: Some(0),
            }],
            ..recoverable_board(2)
        },
    );

    assert_eq!(
        driver.recover_component(C0, 0),
        Err(DriverError::RecoveryFault)
    );
    // The next attempt succeeds: a fault does not poison the path.
    assert_eq!(driver.recover_component(C0, 1), Ok(Event::Restored(C0)));
}

// An unknown component is refused before the mechanism is consulted.
#[test]
fn recover_unknown_component_is_refused() {
    let mut driver =
        PlatformDriver::<RecoverableBoard, 1>::new(&passive_entries(), recoverable_board(2));

    assert_eq!(
        driver.recover_component(ComponentId::new(9), 0),
        Err(DriverError::UnknownComponent)
    );
}

// The `()` impl reports exhaustion on every attempt: no sources exist.
#[test]
fn unit_recovery_always_exhausted() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(&passive_entries(), mock_board());

    assert_eq!(
        driver.recover_component(C0, 0),
        Ok(Event::RecoveryUnavailable(C0))
    );
}

// RecoverComponent through the Platform seam: a successful restore returns
// Some(Restored), a faulting mechanism returns EffectError.
#[test]
fn execute_routes_recover_component() {
    use openprot_orchestrator_sm::{Effect, EffectError, Platform};

    let mut driver =
        PlatformDriver::<RecoverableBoard, 1>::new(&passive_entries(), recoverable_board(2));

    assert_eq!(
        driver.execute(Effect::RecoverComponent { id: C0, attempt: 0 }),
        Ok(Some(Event::Restored(C0)))
    );

    let mut faulting_driver = PlatformDriver::<RecoverableBoard, 1>::new(
        &passive_entries(),
        Board {
            recovery: [MockRecovery {
                sources: 2,
                fail_on: Some(0),
            }],
            ..recoverable_board(2)
        },
    );

    assert_eq!(
        faulting_driver.execute(Effect::RecoverComponent { id: C0, attempt: 0 }),
        Err(EffectError)
    );
}

// ---------------------------------------------------------------------------
// Report sink.
// ---------------------------------------------------------------------------

/// Records what it is handed: the seam satisfied without a management
/// transport. Tests read `seen` back through `PlatformDriver::board`.
#[derive(Default)]
struct RecordingSink {
    seen: std::vec::Vec<Report>,
}

impl RecordingSink {
    fn new() -> Self {
        Self::default()
    }
}

impl ReportSink for RecordingSink {
    fn report(&mut self, report: Report) {
        self.seen.push(report);
    }
}

/// One of each report, so a test covers the whole enum.
fn every_report() -> [Report; 5] {
    [
        Report::Isolated(C0),
        Report::RecoveryFailed(C0),
        Report::BootFailed {
            id: C0,
            checkpoint: "self-test",
            kind: BootFailureKind::DeviceFatal,
        },
        Report::UpdateDeferred,
        Report::UpdateAborted,
    ]
}

// Every report is deliverable through the seam alone, in the order handed
// over; the unit sink is a wiring choice and satisfies the same caller.
#[test]
fn every_report_reaches_a_sink() {
    fn tell<S: ReportSink>(sink: &mut S, reports: [Report; 5]) {
        for report in reports {
            sink.report(report);
        }
    }

    let mut recording = RecordingSink::new();
    tell(&mut recording, every_report());
    assert_eq!(recording.seen, every_report());

    tell(&mut (), every_report());
}

// ---------------------------------------------------------------------------
// Anti-rollback floor commits.
// ---------------------------------------------------------------------------

// The floor may only move to the SVN the verifier vouched for, and only
// after a verification has passed — the two halves of the commit contract.
#[test]
fn commit_advances_the_floor_to_the_verified_svn() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(&passive_entries(), mock_board());

    driver
        .execute(Effect::ReadFirmware(C0))
        .expect("stage failed");
    driver
        .execute(Effect::VerifyFirmware(C0))
        .expect("verify failed");

    assert_eq!(driver.execute(Effect::CommitSvnFloor(C0)), Ok(None));
    // Read back through the capability's own seam.
    let SvnFloorBinding::Erot(floor) = &driver.board().svn_floors[0] else {
        panic!("C0 is wired with an eRoT floor");
    };
    assert_eq!(
        floor.floor(),
        Ok(Svn(MOCK_SVN)),
        "floor advanced to the verifier's SVN"
    );
}

// A component wired without an eRoT floor tracks its own SVN. The commit
// succeeds even with no verified image cached: there is no floor to
// mis-advance.
#[test]
fn commit_without_an_erot_floor_is_a_no_op() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            svn_floors: [SvnFloorBinding::SelfManaged],
            ..mock_board()
        },
    );

    assert_eq!(driver.execute(Effect::CommitSvnFloor(C0)), Ok(None));
}

#[test]
fn commit_without_a_verified_image_fails_closed() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.commit_svn_floor(C0),
        Err(DriverError::NoVerifiedImage)
    );
}

// A rejection must clear the cached SVN, or the floor could commit against
// an image that is no longer the authenticated one.
#[test]
fn rejected_image_clears_the_verified_svn() {
    let mut corrupt = valid_image();
    corrupt[7] ^= 0x01;
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            images: [MemImage::holding(valid_image()).reflash_on_reopen(corrupt)],
            ..mock_board()
        },
    );

    driver.stage_firmware(C0).expect("stage failed");
    assert_eq!(
        driver.verify_firmware(C0),
        Ok(Event::VerificationPassed(C0))
    );

    // The queued re-flash lands on the re-stage; the rejection must take
    // the cached SVN with it.
    driver.stage_firmware(C0).expect("re-stage failed");
    assert_eq!(
        driver.verify_firmware(C0),
        Ok(Event::VerificationFailed(C0))
    );

    assert_eq!(
        driver.commit_svn_floor(C0),
        Err(DriverError::NoVerifiedImage)
    );
}

#[test]
fn floor_fault_is_reported() {
    let mut mock = MockFloor::new();
    mock.fail = true;
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            svn_floors: [SvnFloorBinding::Erot(mock)],
            ..mock_board()
        },
    );

    driver.stage_firmware(C0).expect("stage failed");
    driver.verify_firmware(C0).expect("verify failed");

    assert_eq!(driver.commit_svn_floor(C0), Err(DriverError::SvnFloorFault));
}

// Every report effect reaches the board's sink, in emission order, and none
// hands back an error for the SM to fail closed on.
#[test]
fn reports_reach_the_board_sink() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            verifier: XorVerifier {
                fault: false,
                svn: 0,
            },
            ..mock_board()
        },
    );

    for effect in [
        Effect::ReportIsolated(C0),
        Effect::ReportRecoveryFailed(C0),
        Effect::ReportBootFailed {
            id: C0,
            checkpoint: "self-test",
            kind: BootFailureKind::DeviceFatal,
        },
        Effect::ReportUpdateDeferred,
        Effect::ReportUpdateAborted,
    ] {
        assert_eq!(driver.execute(effect), Ok(None));
    }

    assert_eq!(driver.board().report_sink.seen, every_report());
}

// An Isolable component is contained and reported, and the platform keeps
// running: executing a report returns no error, so it never reaches the
// fail-closed path.
#[test]
fn reporting_an_isolated_component_does_not_lock_the_platform() {
    let mut driver = PlatformDriver::<MockBoard, 2>::new(
        &passive_entries(),
        Board {
            verifier: XorVerifier {
                fault: false,
                svn: 0,
            },
            ..mock_board()
        },
    );
    let mut chain = heapless::Vec::<_, 2>::new();
    chain
        .push((C0, ComponentAttrs::passive_required()))
        .unwrap();
    chain
        .push((C1, ComponentAttrs::passive_isolable()))
        .unwrap();
    let mut orch = Orchestrator::<2, 6>::new(chain.try_into().unwrap(), 3);

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready, "both components verified");

    orch.dispatch(&mut driver, Event::CorruptionDetected(C1));

    assert_eq!(orch.state(), State::Ready, "contained, not locked");
    assert_eq!(driver.board().report_sink.seen, [Report::Isolated(C1)]);
}

#[test]
fn submit_update_refuses_an_unknown_component() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.submit_update(ComponentId::new(9), CANDIDATE_LEN),
        Err(DriverError::UnknownComponent)
    );
    assert_eq!(driver.pending_update(), None);
}

// Single update in flight by construction: a second submit is refused and
// the first job's target survives untouched.
#[test]
fn submit_update_refuses_a_second_in_flight() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    driver.submit_update(C0, CANDIDATE_LEN).unwrap();
    assert_eq!(
        driver.submit_update(C0, CANDIDATE_LEN),
        Err(DriverError::UpdateBusy)
    );
    assert_eq!(driver.pending_update(), Some(C0));
}

// The frontend connection end to end: request_update records the job and
// the SM receives UpdateRequest. Ready accepts it and enters Updating,
// whose sole entry effect AuthenticateStageUpdate has an executor, so
// the SM stays in Updating (no latch).
#[test]
fn request_update_reaches_the_sm() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN).unwrap();

    assert_eq!(driver.pending_update(), Some(C0));
    assert_eq!(orch.state(), State::Updating(C0));
}

// A refused submit injects nothing: no job, no event, the SM stays Ready.
#[test]
fn refused_request_update_injects_no_event() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(
        request_update(&mut orch, &mut driver, ComponentId::new(9), CANDIDATE_LEN),
        Err(DriverError::UnknownComponent)
    );

    assert_eq!(driver.pending_update(), None);
    assert_eq!(orch.state(), State::Ready);
}

// The candidate is described by the offer, not by the region it sits
// in: a length past the end of the staging region is refused before
// anything is recorded.
#[test]
fn submit_update_refuses_a_candidate_past_the_staging_region() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.submit_update(C0, STAGING_LEN as u64 + 1),
        Err(DriverError::CandidateOutOfRange)
    );
    assert_eq!(driver.pending_update(), None);
}

// The job is recorded by the frontend before the SM emits anything, so a
// prepare with no job means the two have drifted apart.
#[test]
fn prepare_update_without_a_job_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.prepare_update(), Err(DriverError::NoUpdateJob));
}

// DiscardStaged is the SM's way back to Ready: the job is gone, the
// device dropped what it staged, and the next update can proceed.
#[test]
fn discard_staged_clears_the_job_and_abandons_the_device() {
    let mut driver = driver([MemImage::holding(valid_image())]);
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();

    driver.discard_staged().expect("discard failed");

    assert_eq!(driver.pending_update(), None);
    assert_eq!(driver.board().updatables[0].abandons, 1);
}

// Same drift as the authenticate case: the SM only emits DiscardStaged
// with an update in flight.
#[test]
fn discard_staged_without_a_job_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.discard_staged(), Err(DriverError::NoUpdateJob));
}

// ReportUpdateDeferred clears pending_update so the next request is not
// permanently blocked.
#[test]
fn deferred_report_clears_pending_update() {
    let mut driver = driver([MemImage::holding(valid_image())]);
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();
    assert_eq!(driver.pending_update(), Some(C0));

    driver.execute(Effect::ReportUpdateDeferred).unwrap();

    assert_eq!(
        driver.pending_update(),
        None,
        "deferred report must clear the pending job"
    );

    // A subsequent submit succeeds: the slot is free.
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();
    assert_eq!(driver.pending_update(), Some(C0));
}

// ReportUpdateAborted clears pending_update so an update superseded by
// recovery does not block future requests.
#[test]
fn aborted_update_clears_pending_update() {
    let mut driver = driver([MemImage::holding(valid_image())]);
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();

    driver.execute(Effect::ReportUpdateAborted).unwrap();

    assert_eq!(
        driver.pending_update(),
        None,
        "aborted report must clear the pending job"
    );

    driver.submit_update(C0, CANDIDATE_LEN).unwrap();
    assert_eq!(driver.pending_update(), Some(C0));
}

fn update_driver(updatable: MockUpdatable) -> PlatformDriver<MockBoard, 1> {
    PlatformDriver::new(
        &passive_entries(),
        Board {
            updatables: [updatable],
            ..mock_board()
        },
    )
}

/// Submits a job and runs the entry executor, as entry to `Updating`
/// does.
fn updating(driver: &mut PlatformDriver<MockBoard, 1>) {
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();
    driver.prepare_update().unwrap();
}

#[test]
fn pumping_without_a_job_is_idle() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    let poll = driver.pump_update(0);

    assert_eq!(poll.event, None);
    assert_eq!(poll.progress, None);
}

#[test]
fn pumping_a_submitted_job_is_idle() {
    let mut driver = driver([MemImage::holding(valid_image())]);
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();

    assert_eq!(driver.pump_update(0).event, None);
}

// A device that stages in one step parks at Staged. No UpdateVerified
// is emitted until the crypto verify-client is wired.
#[test]
fn a_pumped_job_parks_at_staged() {
    let mut driver = update_driver(MockUpdatable::new());
    updating(&mut driver);

    for tick in 0..8 {
        assert_eq!(driver.pump_update(tick).event, None);
    }

    assert!(driver.board().updatables[0].ready);
    assert!(driver.pending_update().is_some(), "job survives at Staged");
}

// Staging is polled too: a device that takes four steps is not waited
// out inside one pump call.
#[test]
fn staging_advances_one_bounded_step_per_pump() {
    let mut driver = update_driver(MockUpdatable::stepping(4));
    updating(&mut driver);

    // Tick 0: Submitted -> Staging transition, device not staged yet.
    driver.pump_update(0);
    assert!(!driver.board().updatables[0].ready, "tick 0: still staging");

    // Ticks 1..4: one transfer step each.
    for tick in 1..5 {
        driver.pump_update(tick);
        assert!(
            !driver.board().updatables[0].ready,
            "tick {tick}: still staging"
        );
    }

    // Tick 5: final step stages the device.
    driver.pump_update(5);
    assert!(driver.board().updatables[0].ready, "tick 5: device staged");

    // Parked at Staged, no event emitted.
    assert_eq!(driver.pump_update(6), UpdatePoll::idle());
}

// A device that fails a staging step ends the job.
#[test]
fn a_device_fault_rejects_the_update() {
    let mut driver = update_driver(MockUpdatable::faulting());
    updating(&mut driver);

    let mut event = None;
    for tick in 0..8 {
        if let Some(e) = driver.pump_update(tick).event {
            event = Some(e);
            break;
        }
    }

    assert_eq!(event, Some(Event::UpdateRejected));
    assert_eq!(driver.board().updatables[0].abandons, 1);
}

// A transfer that stops moving is abandoned once the budget is spent,
// rather than held open for a device that stopped answering.
#[test]
fn a_stalled_transfer_is_rejected_at_the_budget() {
    let mut driver = update_driver(MockUpdatable::stalling());
    updating(&mut driver);

    // The device answers Transferring without moving.
    assert_eq!(driver.pump_update(0).event, None);
    assert_eq!(driver.pump_update(STALL_BUDGET_MILLIS - 1).event, None);

    let poll = driver.pump_update(STALL_BUDGET_MILLIS);

    assert_eq!(poll.event, Some(Event::UpdateRejected));
    assert_eq!(driver.board().updatables[0].abandons, 1);
}

// Progress restarts the budget: a slow transfer that keeps moving is not
// a stalled one.
#[test]
fn progress_restarts_the_stall_budget() {
    let mut driver = update_driver(MockUpdatable::stepping(4));
    updating(&mut driver);

    // One step per budget window: every call moves `written`, so the
    // budget never runs out even though each step takes nearly all of it.
    let mut tick = 0;
    for _ in 0..12 {
        tick += STALL_BUDGET_MILLIS - 1;
        let poll = driver.pump_update(tick);
        assert_eq!(poll.event, None, "no rejection, no verdict");
    }

    assert!(driver.board().updatables[0].ready, "device staged");
}

// The SM emits AuthenticateStageUpdate on entry to Updating. A job
// missing the prepare flag means the two sides drifted, so the pump
// refuses rather than pushing bytes the SM never commanded.
#[test]
fn a_job_the_sm_never_commanded_stays_idle() {
    let mut driver = update_driver(MockUpdatable::new());
    driver.submit_update(C0, CANDIDATE_LEN).unwrap();
    // No prepare_update call.

    let mut event = None;
    for tick in 0..8 {
        if let Some(e) = driver.pump_update(tick).event {
            event = Some(e);
            break;
        }
    }

    // Submitted phase returns idle, not rejected.
    assert_eq!(event, None);
}

// Same drift as the other update executors: the SM only emits
// ActivateUpdate with an update in flight.
#[test]
fn activate_update_without_a_job_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.activate_update(), Err(DriverError::NoUpdateJob));
}

// Activation is the SM's answer to UpdateVerified, and it only applies
// to a job the device has actually staged.
#[test]
fn activate_update_requires_a_staged_job() {
    let mut driver = update_driver(MockUpdatable::new());
    updating(&mut driver);

    assert_eq!(
        driver.activate_update(),
        Err(DriverError::CandidateNotStaged)
    );
    assert_eq!(
        driver.pending_update(),
        Some(C0),
        "the job survives, so DiscardStaged still finds it"
    );
}

// The job ends at activation: the device runs the candidate on its next
// boot, tentatively, and the driver is free for the next update.
#[test]
fn activate_update_ends_the_job() {
    let mut driver = update_driver(MockUpdatable::new());
    updating(&mut driver);
    for tick in 0..8 {
        if driver.pump_update(tick).event.is_some() {
            break;
        }
    }

    driver.activate_update().expect("activate failed");

    assert!(driver.board().updatables[0].active);
    assert_eq!(driver.pending_update(), None);
}

// The path through the SM up to Staged: a request, the pump, staging
// completes, and the pump parks. The full path through activation
// requires the crypto verify-client (emitting UpdateVerified).
#[test]
fn an_update_stages_through_the_sm() {
    let mut orch = orchestrator();
    let mut driver = update_driver(MockUpdatable::stepping(2));
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN).unwrap();
    assert_eq!(orch.state(), State::Updating(C0));

    for tick in 0..16 {
        if let Some(event) = driver.pump_update(tick).event {
            orch.dispatch(&mut driver, event);
            break;
        }
    }

    // Pump parked at Staged, no event emitted. SM stays in Updating.
    assert_eq!(orch.state(), State::Updating(C0));
    assert!(driver.board().updatables[0].ready);
    assert!(driver.pending_update().is_some());
}

// The rejection path through the SM: DiscardStaged runs and the platform
// is back in Ready.
#[test]
fn a_rejected_update_returns_the_platform_to_ready() {
    let mut orch = orchestrator();
    let mut driver = update_driver(MockUpdatable::faulting());
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN).unwrap();

    for tick in 0..16 {
        if let Some(event) = driver.pump_update(tick).event {
            orch.dispatch(&mut driver, event);
            break;
        }
    }

    assert_eq!(orch.state(), State::Ready);
    assert_eq!(driver.pending_update(), None, "DiscardStaged cleared it");
    assert!(!driver.board().updatables[0].active);
}

// bring_up takes a ChainEntries, the validated output of chain_of. The
// machine and the driver cannot be holding different component lists
// because the entries come from one const-validated device table.
#[test]
fn bring_up_builds_both_halves_from_one_chain() {
    use core::time::Duration;
    use orchestrator_config::{chain_of, BootCheckpoint, ChainEntries, DeviceConfig};

    const CP: BootCheckpoint<u8> = BootCheckpoint::new("up", 1, Duration::from_millis(10));
    const fn dev(attrs: ComponentAttrs) -> DeviceConfig<u8, u8> {
        DeviceConfig::new("dev", 0, &[CP], None, attrs)
    }
    const TABLE: [DeviceConfig<u8, u8>; 1] = [dev(ComponentAttrs::active_required())];
    const CHAIN: ChainEntries<1> = chain_of(&TABLE);

    let (orch, driver) = bring_up::<MockBoard, 1, 4>(&CHAIN, mock_board(), 3);

    assert_eq!(orch.state(), State::PowerOnReset);
    assert!(driver.pending_update().is_none());
}

// The driver indexes every per-component array by id, so an entry out of
// position would address the wrong component's reset line and flash.
#[test]
#[should_panic(expected = "id must be its position")]
fn an_entry_out_of_position_is_refused() {
    let entries = [(ComponentId::new(3), ComponentAttrs::passive_required())];
    let _ = PlatformDriver::<MockBoard, 1>::new(&entries, mock_board());
}

// Before the platform is in service nothing would report the request
// deferred, so it is refused and no job is recorded.
#[test]
fn a_request_before_the_platform_is_in_service_is_refused() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(orch.state(), State::PowerOnReset);
    assert_eq!(
        request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN),
        Err(DriverError::Unsupervised)
    );
    assert_eq!(driver.pending_update(), None);
}

// Same for a locked platform, which can neither run the update nor report
// it deferred.
#[test]
fn a_request_to_a_locked_platform_is_refused() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(
        &passive_entries(),
        Board {
            verifier: XorVerifier {
                fault: true,
                svn: MOCK_SVN,
            },
            ..mock_board()
        },
    );
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Locked);

    assert_eq!(
        request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN),
        Err(DriverError::Unsupervised)
    );
    assert_eq!(driver.pending_update(), None);
}

// The driver refuses a second request itself: its single-job rule answers
// before the state machine is asked, and the running update is left alone.
#[test]
fn a_second_request_while_an_update_runs_is_refused_as_busy() {
    let mut orch = orchestrator();
    let mut driver = update_driver(MockUpdatable::stepping(2));
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN).unwrap();
    assert_eq!(orch.state(), State::Updating(C0));

    assert_eq!(
        request_update(&mut orch, &mut driver, C0, CANDIDATE_LEN),
        Err(DriverError::UpdateBusy)
    );
    assert!(
        driver.pending_update().is_some(),
        "the running job survives"
    );
}
