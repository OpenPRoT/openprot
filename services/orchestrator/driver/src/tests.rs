// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use crate::*;
use openprot_orchestrator_sm::{
    BootFailureKind, ComponentAttrs, ComponentId, ComponentKind, Effect, Event, Orchestrator,
    Platform, PowerOnResult, State,
};
use orchestrator_capabilities::{BootWatch, FailureCause, Svn, SvnFloor, WalkVerdict};

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

#[derive(Debug, PartialEq)]
struct VerifierError;

impl core::fmt::Display for VerifierError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("verifier broken")
    }
}

impl core::error::Error for VerifierError {}

/// A check that runs to completion in one call, as the test fakes below do.
trait SyncCheck {
    type Error: core::error::Error;

    fn verify(
        &mut self,
        id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, Self::Error>;
}

/// Runs a [`SyncCheck`] as a [`Verifier`]: the check happens inside `start`
/// and the first `poll` returns the verdict. A real verifier that is fast and
/// local is written the same way.
struct Immediate<V: SyncCheck> {
    inner: V,
    done: Option<Verdict>,
}

impl<V: SyncCheck> Immediate<V> {
    fn new(inner: V) -> Self {
        Self { inner, done: None }
    }
}

impl<V: SyncCheck> Verifier for Immediate<V> {
    type Error = V::Error;

    fn start(&mut self, id: ComponentId, image: &mut impl ImageSource) -> Result<(), Self::Error> {
        self.done = None;
        let verdict = self.inner.verify(id, image)?;
        self.done = Some(verdict);
        Ok(())
    }

    fn poll(&mut self, _now_millis: u64) -> Result<VerifyStatus, Self::Error> {
        Ok(match self.done.take() {
            Some(verdict) => VerifyStatus::Done(verdict),
            None => VerifyStatus::Pending {
                deadline_millis: None,
            },
        })
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.done = None;
        Ok(())
    }
}

/// The magic + XOR-zero check as a board-supplied verifier, reading in
/// chunks.
struct XorVerifier {
    fault: bool,
    /// The manifest SVN this verifier reports for an authenticated image.
    /// A real verifier reads it from the signed manifest of the image it
    /// just checked; the test image format has no manifest, so tests pin it.
    svn: u32,
}

impl SyncCheck for XorVerifier {
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

/// The test board's type choices.
struct MockBoard;

impl BoardCapabilities for MockBoard {
    type Image = MemImage;
    type Verifier = Immediate<XorVerifier>;
    type BootControl = MockReset;
    type BootWatch = MockWalk;
    type SvnFloor = MockFloor;
    type ReportSink = RecordingSink;
}

/// The SVN `mock_board`'s verifier vouches for. Tests that read the floor
/// back assert against it.
const MOCK_SVN: u32 = 5;

/// Happy-path wiring for `N` components. Tests override the one field
/// they exercise with `..mock_board()`.
fn mock_board<const N: usize>() -> Board<MockBoard, N> {
    Board {
        images: core::array::from_fn(|_| MemImage::holding(valid_image())),
        verifier: Immediate::new(XorVerifier {
            fault: false,
            svn: MOCK_SVN,
        }),
        boot_controls: core::array::from_fn(|_| MockReset::new()),
        boot_watches: core::array::from_fn(|_| MockWalk::idle()),
        component_kinds: core::array::from_fn(|_| ComponentKind::Passive),
        svn_floors: core::array::from_fn(|_| SvnFloorBinding::Erot(MockFloor::new())),
        report_sink: RecordingSink::new(),
    }
}

fn driver(images: [MemImage; 1]) -> PlatformDriver<MockBoard, 1> {
    PlatformDriver::new(Board {
        images,
        ..mock_board()
    })
}

fn orchestrator() -> Orchestrator<1, 4> {
    let mut chain = heapless::Vec::<_, 1>::new();
    chain
        .push((C0, ComponentAttrs::passive_required()))
        .unwrap();
    Orchestrator::new(chain.try_into().unwrap(), 3)
}

/// Start the check and collect its verdict, for a verifier that answers on the
/// first poll (`Immediate`).
fn verify_now<B: BoardCapabilities, const N: usize>(
    driver: &mut PlatformDriver<B, N>,
    id: ComponentId,
) -> Result<Event, DriverError> {
    driver.start_verify(id)?;
    Ok(driver
        .poll_verify(0)?
        .expect("an immediate verifier answers on the first poll"))
}

/// Dispatch `event`, then deliver verification verdicts as the run loop does
/// until none is left. A poll error is dispatched as `EffectFailed`.
fn dispatch_settled<B: BoardCapabilities, const N: usize, const E: usize>(
    orch: &mut Orchestrator<N, E>,
    driver: &mut PlatformDriver<B, N>,
    event: Event,
) {
    orch.dispatch(driver, event);
    loop {
        match driver.poll_verify(0) {
            Ok(Some(ev)) => orch.dispatch(driver, ev),
            Ok(None) => break,
            Err(_) => {
                orch.dispatch(driver, Event::EffectFailed);
                break;
            }
        }
    }
}

// PowerGood drives ReadFirmware + VerifyFirmware into the driver; the
// verdict is polled back and dispatched, carrying a passive component all
// the way to Ready.
#[test]
fn boot_verifies_the_first_component() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

    assert_eq!(orch.state(), State::Ready);
}

#[test]
fn corrupt_image_fails_verification() {
    let mut corrupt = valid_image();
    corrupt[7] ^= 0x01;
    let mut driver = driver([MemImage::holding(corrupt)]);

    driver.stage_firmware(C0).unwrap();

    assert_eq!(
        verify_now(&mut driver, C0),
        Ok(Event::VerificationFailed(C0))
    );
}

#[test]
fn verify_without_read_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(verify_now(&mut driver, C0), Err(DriverError::NotStaged));
}

// An unopenable source is a failed actuation, not a verdict: the SM
// latches Locked instead of getting a forged VerificationFailed.
#[test]
fn unopenable_source_fails_closed() {
    let mut orch = orchestrator();
    let mut image = MemImage::holding(valid_image());
    image.fail_open = true;
    let mut driver = driver([image]);

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

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

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

    assert_eq!(orch.state(), State::Locked);
}

// So does a verifier that cannot run its check.
#[test]
fn verifier_fault_fails_closed() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        verifier: Immediate::new(XorVerifier {
            fault: true,
            svn: MOCK_SVN,
        }),
        ..mock_board()
    });

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

    assert_eq!(orch.state(), State::Locked);
}

const C1: ComponentId = ComponentId::new(1);

#[test]
fn verify_for_a_different_component_is_refused() {
    let mut driver = PlatformDriver::<MockBoard, 2>::new(mock_board());

    driver.stage_firmware(C0).unwrap();

    assert_eq!(verify_now(&mut driver, C1), Err(DriverError::NotStaged));
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
        verify_now(&mut driver, ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );
}

#[test]
fn reset_release_and_assert_reach_the_boot_control() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(mock_board());

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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        boot_controls: [control],
        ..mock_board()
    });

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
        driver.execute(Effect::Emit(Event::UpdateRequest)),
        Err(EffectError)
    );
}

// VerifyFirmware only starts the check: execute returns at once with no event,
// and the verdict is collected from poll_verify.
#[test]
fn execute_starts_the_check_and_the_verdict_is_polled() {
    use openprot_orchestrator_sm::{Effect, Platform};

    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.execute(Effect::ReadFirmware(C0)), Ok(None));
    assert_eq!(driver.execute(Effect::VerifyFirmware(C0)), Ok(None));
    assert_eq!(
        driver.poll_verify(0),
        Ok(Some(Event::VerificationPassed(C0)))
    );
    // Delivered once.
    assert_eq!(driver.poll_verify(0), Ok(None));
}

/// Wraps [`XorVerifier`] and snapshots the reset line as the check runs,
/// so the test can see the line state inside the verification window.
struct LineWatchingVerifier {
    inner: XorVerifier,
    line: std::rc::Rc<core::cell::Cell<bool>>,
    held_during_verify: std::rc::Rc<core::cell::Cell<bool>>,
}

impl SyncCheck for LineWatchingVerifier {
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
    type Verifier = Immediate<LineWatchingVerifier>;
    type BootControl = MockReset;
    type BootWatch = MockWalk;
    type SvnFloor = MockFloor;
    // A board with nothing to tell: exercises the no-op sink.
    type ReportSink = ();
}

// The at-rest guarantee end to end: the component is still held while its
// image is verified, and the line is released only on the passing verdict.
#[test]
fn release_follows_verification() {
    let control = MockReset::new();
    let held = control.held.clone();
    let held_during_verify = std::rc::Rc::new(core::cell::Cell::new(false));
    let mut driver = PlatformDriver::<WatchBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: Immediate::new(LineWatchingVerifier {
            inner: XorVerifier {
                fault: false,
                svn: MOCK_SVN,
            },
            line: held.clone(),
            held_during_verify: held_during_verify.clone(),
        }),
        boot_controls: [control],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        svn_floors: [SvnFloorBinding::Erot(MockFloor::new())],
        report_sink: (),
    });
    let mut orch = orchestrator();

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        boot_controls: [control],
        ..mock_board()
    });
    let mut orch = orchestrator();

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );

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
    PlatformDriver::new(Board {
        boot_watches: walks,
        component_kinds,
        ..mock_board()
    })
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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        boot_watches: [MockWalk::scripted(std::vec![WalkVerdict::Complete])],
        ..mock_board()
    });

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );
    assert_eq!(orch.state(), State::Ready);

    let event = driver.poll_boot_walks(0).event.expect("walk completed");
    assert_eq!(event, Event::Booted(C0));
    orch.dispatch(&mut driver, event);

    assert_eq!(orch.state(), State::Ready);
}

// End to end, failure path: the released component's walk fails, its
// BootFailed enters recovery, and with no recovery capability composed
// yet the machine fails closed.
#[test]
fn boot_failure_fails_closed_without_recovery() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        boot_watches: [MockWalk::scripted(std::vec![WalkVerdict::Failed {
            checkpoint: "heartbeat",
            cause: FailureCause::TimedOut,
        }])],
        ..mock_board()
    });

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );
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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(mock_board());

    driver
        .execute(Effect::ReadFirmware(C0))
        .expect("stage failed");
    driver
        .execute(Effect::VerifyFirmware(C0))
        .expect("verify failed");
    driver.poll_verify(0).expect("poll failed");

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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        svn_floors: [SvnFloorBinding::SelfManaged],
        ..mock_board()
    });

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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image()).reflash_on_reopen(corrupt)],
        ..mock_board()
    });

    driver.stage_firmware(C0).expect("stage failed");
    assert_eq!(
        verify_now(&mut driver, C0),
        Ok(Event::VerificationPassed(C0))
    );

    // The queued re-flash lands on the re-stage; the rejection must take
    // the cached SVN with it.
    driver.stage_firmware(C0).expect("re-stage failed");
    assert_eq!(
        verify_now(&mut driver, C0),
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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        svn_floors: [SvnFloorBinding::Erot(mock)],
        ..mock_board()
    });

    driver.stage_firmware(C0).expect("stage failed");
    verify_now(&mut driver, C0).expect("verify failed");

    assert_eq!(driver.commit_svn_floor(C0), Err(DriverError::SvnFloorFault));
}

// Every report effect reaches the board's sink, in emission order, and none
// hands back an error for the SM to fail closed on.
#[test]
fn reports_reach_the_board_sink() {
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        verifier: Immediate::new(XorVerifier {
            fault: false,
            svn: 0,
        }),
        ..mock_board()
    });

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
    let mut driver = PlatformDriver::<MockBoard, 2>::new(Board {
        verifier: Immediate::new(XorVerifier {
            fault: false,
            svn: 0,
        }),
        ..mock_board()
    });
    let mut chain = heapless::Vec::<_, 2>::new();
    chain
        .push((C0, ComponentAttrs::passive_required()))
        .unwrap();
    chain
        .push((C1, ComponentAttrs::passive_isolable()))
        .unwrap();
    let mut orch = Orchestrator::<2, 6>::new(chain.try_into().unwrap(), 3);

    dispatch_settled(
        &mut orch,
        &mut driver,
        Event::PowerGood(PowerOnResult::Provisioned),
    );
    assert_eq!(orch.state(), State::Ready, "both components verified");

    orch.dispatch(&mut driver, Event::CorruptionDetected(C1));

    assert_eq!(orch.state(), State::Ready, "contained, not locked");
    assert_eq!(driver.board().report_sink.seen, [Report::Isolated(C1)]);
}

// ---------------------------------------------------------------------------
// Asynchronous verification.
// ---------------------------------------------------------------------------

/// What a [`ScriptedVerifier`] answers once its pending polls run out.
#[derive(Clone, Copy)]
enum Scripted {
    Done(Verdict),
    TimedOut,
    Fault,
}

/// An [`Verifier`] the test drives: `Pending` for a set number of polls,
/// then the scripted outcome. Records starts and cancels.
struct ScriptedVerifier {
    pending: u32,
    outcome: Scripted,
    running: bool,
    starts: u32,
    cancels: u32,
    /// When a check that is still pending gives up, in milliseconds.
    deadline: Option<u64>,
    /// Starts after this many have been made fail.
    fail_after: Option<u32>,
}

impl ScriptedVerifier {
    fn new(pending: u32, outcome: Scripted) -> Self {
        Self {
            pending,
            outcome,
            running: false,
            starts: 0,
            cancels: 0,
            deadline: None,
            fail_after: None,
        }
    }
}

impl Verifier for ScriptedVerifier {
    type Error = VerifierError;

    fn start(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<(), VerifierError> {
        self.starts += 1;
        if self.fail_after.is_some_and(|n| self.starts > n) {
            return Err(VerifierError);
        }
        self.running = true;
        Ok(())
    }

    fn poll(&mut self, now_millis: u64) -> Result<VerifyStatus, VerifierError> {
        let pending = VerifyStatus::Pending {
            deadline_millis: self.deadline,
        };
        if !self.running {
            return Ok(pending);
        }
        if self.deadline.is_some_and(|at| now_millis >= at) {
            self.running = false;
            return Ok(VerifyStatus::TimedOut);
        }
        if self.pending > 0 {
            self.pending -= 1;
            return Ok(pending);
        }
        self.running = false;
        match self.outcome {
            Scripted::Done(verdict) => Ok(VerifyStatus::Done(verdict)),
            Scripted::TimedOut => Ok(VerifyStatus::TimedOut),
            Scripted::Fault => Err(VerifierError),
        }
    }

    fn cancel(&mut self) -> Result<(), VerifierError> {
        self.cancels += 1;
        self.running = false;
        Ok(())
    }
}

struct ScriptedBoard;

impl BoardCapabilities for ScriptedBoard {
    type Image = MemImage;
    type Verifier = ScriptedVerifier;
    type BootControl = MockReset;
    type BootWatch = MockWalk;
    type SvnFloor = MockFloor;
    type ReportSink = RecordingSink;
}

fn scripted_driver(pending: u32, outcome: Scripted) -> PlatformDriver<ScriptedBoard, 1> {
    scripted_driver_with_deadline(pending, outcome, None)
}

fn scripted_driver_with_deadline(
    pending: u32,
    outcome: Scripted,
    deadline: Option<u64>,
) -> PlatformDriver<ScriptedBoard, 1> {
    scripted_driver_with_walk(pending, outcome, deadline, MockWalk::idle())
}

fn scripted_driver_with_walk(
    pending: u32,
    outcome: Scripted,
    deadline: Option<u64>,
    walk: MockWalk,
) -> PlatformDriver<ScriptedBoard, 1> {
    let mut verifier = ScriptedVerifier::new(pending, outcome);
    verifier.deadline = deadline;
    PlatformDriver::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier,
        boot_controls: [MockReset::new()],
        boot_watches: [walk],
        component_kinds: [ComponentKind::Passive],
        svn_floors: [SvnFloorBinding::Erot(MockFloor::new())],
        report_sink: RecordingSink::new(),
    })
}

const AUTHENTICATED: Verdict = Verdict::Authenticated { svn: Svn(MOCK_SVN) };

// A slow verifier holds the verdict back without holding the driver: starting
// returns at once, polling says nothing until the check answers, and the
// verdict is delivered once.
#[test]
fn a_pending_check_holds_back_the_verdict() {
    let mut driver = scripted_driver(2, Scripted::Done(AUTHENTICATED));
    driver.stage_firmware(C0).unwrap();

    assert_eq!(driver.start_verify(C0), Ok(()));
    assert_eq!(driver.poll_verify(0), Ok(None));
    assert_eq!(driver.poll_verify(0), Ok(None));
    assert_eq!(
        driver.poll_verify(0),
        Ok(Some(Event::VerificationPassed(C0)))
    );
    assert_eq!(driver.poll_verify(0), Ok(None));
    assert_eq!(driver.board().verifier.starts, 1);
}

// A check that ran past its deadline is a failed verification, not a verdict
// and not a lock: the machine recovers the component. No verification completed, so
// no SVN is recorded for the commit.
#[test]
fn a_timed_out_check_reports_failure_and_records_no_svn() {
    let mut driver = scripted_driver(0, Scripted::TimedOut);
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    assert_eq!(
        driver.poll_verify(0),
        Ok(Some(Event::VerificationFailed(C0)))
    );
    assert_eq!(
        driver.commit_svn_floor(C0),
        Err(DriverError::NoVerifiedImage)
    );
}

// A verifier that cannot carry the check through is a fault, which ends the
// check; the run loop answers it with EffectFailed.
#[test]
fn a_failed_poll_ends_the_check_with_a_fault() {
    let mut driver = scripted_driver(0, Scripted::Fault);
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    assert_eq!(driver.poll_verify(0), Err(DriverError::VerifierFault));
    assert_eq!(driver.poll_verify(0), Ok(None), "nothing left outstanding");
}

// Re-staging replaces the image the outstanding check is verifying, so the check
// is cancelled and its answer never reaches the machine.
#[test]
fn restaging_cancels_the_outstanding_check() {
    let mut driver = scripted_driver(5, Scripted::Done(AUTHENTICATED));
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    driver.stage_firmware(C0).unwrap();

    assert_eq!(driver.board().verifier.cancels, 1);
    assert_eq!(driver.poll_verify(0), Ok(None), "the old check is gone");
}

// Starting a second check cancels the first, so only one is ever outstanding.
#[test]
fn starting_again_cancels_the_first_check() {
    let mut driver = scripted_driver(1, Scripted::Done(AUTHENTICATED));
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    driver.start_verify(C0).unwrap();

    assert_eq!(driver.board().verifier.cancels, 1);
    assert_eq!(driver.board().verifier.starts, 2);
    assert_eq!(driver.poll_verify(0), Ok(None));
    assert_eq!(
        driver.poll_verify(0),
        Ok(Some(Event::VerificationPassed(C0)))
    );
}

// Until a new check answers, the earlier authenticated reading must not reach
// the irreversible floor commit.
#[test]
fn starting_a_check_voids_the_earlier_svn() {
    let mut driver = scripted_driver(0, Scripted::Done(AUTHENTICATED));
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();
    assert_eq!(
        driver.poll_verify(0),
        Ok(Some(Event::VerificationPassed(C0)))
    );
    assert_eq!(driver.commit_svn_floor(C0), Ok(()));

    driver.start_verify(C0).unwrap();

    assert_eq!(
        driver.commit_svn_floor(C0),
        Err(DriverError::NoVerifiedImage)
    );
}

// The full path through the machine with a slow verifier: PowerGood starts
// the check and the machine waits in PreSupervision; only the polled verdict
// carries it to Ready.
#[test]
fn the_machine_waits_for_a_slow_verdict() {
    let mut orch = orchestrator();
    let mut driver = scripted_driver(2, Scripted::Done(AUTHENTICATED));

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::PreSupervision, "verdict not in yet");

    assert_eq!(driver.poll_verify(0), Ok(None));
    assert_eq!(driver.poll_verify(0), Ok(None));
    let verdict = driver.poll_verify(0).unwrap().expect("verdict");
    orch.dispatch(&mut driver, verdict);

    assert_eq!(orch.state(), State::Ready);
}

/// Delegates to the driver and records the effects the machine emitted.
struct Recording<'a, P: Platform> {
    inner: &'a mut P,
    effects: std::vec::Vec<Effect>,
}

impl<P: Platform> Platform for Recording<'_, P> {
    fn execute(
        &mut self,
        effect: Effect,
    ) -> Result<Option<Event>, openprot_orchestrator_sm::EffectError> {
        self.effects.push(effect);
        self.inner.execute(effect)
    }
}

// A timeout reaches the machine as a failed verification, so it asks for the
// component to be recovered. It does not report a fault, which would lock the
// platform. (The driver has no recovery executor yet, so the request itself
// still fails closed; that gap is separate from this one.)
#[test]
fn a_timeout_asks_the_machine_to_recover_the_component() {
    let mut orch = orchestrator();
    let mut driver = scripted_driver(0, Scripted::TimedOut);

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    let verdict = driver.poll_verify(0).unwrap().expect("a timeout answers");
    assert_eq!(verdict, Event::VerificationFailed(C0));

    let mut recording = Recording {
        inner: &mut driver,
        effects: std::vec::Vec::new(),
    };
    orch.dispatch(&mut recording, verdict);

    assert!(
        recording
            .effects
            .contains(&Effect::RecoverComponent { id: C0, attempt: 0 }),
        "the machine asked for recovery: {:?}",
        recording.effects
    );
}

// The run loop sleeps until the verifier's deadline, because a verifier that
// has stopped answering sends no signal. The deadline arrives with the first
// poll after start, and is gone once the check ends.
#[test]
fn the_driver_reports_the_verifiers_deadline_only_while_a_check_is_outstanding() {
    let mut driver = scripted_driver_with_deadline(1, Scripted::Done(AUTHENTICATED), Some(100));
    driver.stage_firmware(C0).unwrap();
    assert_eq!(driver.next_verify_deadline(), None, "nothing started yet");

    driver.start_verify(C0).unwrap();
    assert_eq!(
        driver.next_verify_deadline(),
        None,
        "not learnt until a poll"
    );

    assert_eq!(driver.poll_verify(10), Ok(None));
    assert_eq!(driver.next_verify_deadline(), Some(100));
    assert_eq!(
        driver.poll_verify(20),
        Ok(Some(Event::VerificationPassed(C0)))
    );
    assert_eq!(driver.next_verify_deadline(), None, "the check ended");
}

// A verifier that never answers: still pending before its deadline, a failed
// verification at it, so the machine recovers the component.
#[test]
fn a_silent_verifier_times_out_when_polled_at_its_deadline() {
    let mut driver =
        scripted_driver_with_deadline(u32::MAX, Scripted::Done(AUTHENTICATED), Some(100));
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    assert_eq!(driver.poll_verify(99), Ok(None));
    assert_eq!(driver.next_verify_deadline(), Some(100));
    assert_eq!(
        driver.poll_verify(100),
        Ok(Some(Event::VerificationFailed(C0)))
    );
    assert_eq!(driver.next_verify_deadline(), None);
    assert_eq!(
        driver.commit_svn_floor(C0),
        Err(DriverError::NoVerifiedImage)
    );
}

// A check that finishes inside start has no deadline for the loop to wake for.
#[test]
fn a_verifier_that_finishes_inside_start_has_no_deadline() {
    let mut driver = driver([MemImage::holding(valid_image())]);
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();
    driver.poll_verify(0).unwrap();

    assert_eq!(driver.next_verify_deadline(), None);
}

// An earlier authenticated reading must not survive a new check, even one the
// verifier cannot start: the SVN is voided before the verifier is asked.
#[test]
fn a_check_that_cannot_start_still_voids_the_recorded_svn() {
    let mut verifier = ScriptedVerifier::new(0, Scripted::Done(AUTHENTICATED));
    verifier.fail_after = Some(1);
    let mut driver = PlatformDriver::<ScriptedBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier,
        boot_controls: [MockReset::new()],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        svn_floors: [SvnFloorBinding::Erot(MockFloor::new())],
        report_sink: RecordingSink::new(),
    });
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();
    assert_eq!(
        driver.poll_verify(0),
        Ok(Some(Event::VerificationPassed(C0)))
    );

    assert_eq!(driver.start_verify(C0), Err(DriverError::VerifierFault));

    assert_eq!(
        driver.commit_svn_floor(C0),
        Err(DriverError::NoVerifiedImage),
        "the earlier reading must be gone"
    );
}

// poll: the one call a run loop makes. It polls the verification and the boot
// walks and reports the earliest wake-up, so the loop cannot sleep without
// the deadline of a check it has just started.
#[test]
fn poll_learns_a_new_checks_deadline_in_one_call() {
    let mut driver =
        scripted_driver_with_deadline(u32::MAX, Scripted::Done(AUTHENTICATED), Some(100));
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    assert_eq!(
        driver.poll(0),
        Ok(DriverPoll::Idle {
            next_wake_millis: Some(100)
        })
    );
}

#[test]
fn poll_reports_the_earliest_of_the_verifier_and_the_walks() {
    for (walk_at, verify_at, expected) in [(50, 100, 50), (500, 100, 100)] {
        let walk = MockWalk::scripted(std::vec![WalkVerdict::Waiting {
            deadline_millis: walk_at,
        }]);
        let mut driver = scripted_driver_with_walk(
            u32::MAX,
            Scripted::Done(AUTHENTICATED),
            Some(verify_at),
            walk,
        );
        driver.stage_firmware(C0).unwrap();
        driver.start_verify(C0).unwrap();
        driver.release_reset(C0).unwrap();

        assert_eq!(
            driver.poll(0),
            Ok(DriverPoll::Idle {
                next_wake_millis: Some(expected)
            }),
            "walk at {walk_at}, verifier at {verify_at}"
        );
    }
}

#[test]
fn poll_returns_the_verification_before_the_walks() {
    let walk = MockWalk::scripted(std::vec![WalkVerdict::Complete]);
    let mut driver = scripted_driver_with_walk(0, Scripted::Done(AUTHENTICATED), None, walk);
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();
    driver.release_reset(C0).unwrap();

    assert_eq!(
        driver.poll(0),
        Ok(DriverPoll::Event(Event::VerificationPassed(C0)))
    );
    assert_eq!(driver.poll(0), Ok(DriverPoll::Event(Event::Booted(C0))));
    assert_eq!(
        driver.poll(0),
        Ok(DriverPoll::Idle {
            next_wake_millis: None
        }),
        "each event is delivered once, then the loop is quiet"
    );
}

#[test]
fn poll_with_nothing_outstanding_is_quiet() {
    let mut driver = scripted_driver(0, Scripted::Done(AUTHENTICATED));

    assert_eq!(
        driver.poll(0),
        Ok(DriverPoll::Idle {
            next_wake_millis: None
        })
    );
}

#[test]
fn poll_passes_a_verifier_fault_through() {
    let mut driver = scripted_driver(0, Scripted::Fault);
    driver.stage_firmware(C0).unwrap();
    driver.start_verify(C0).unwrap();

    assert_eq!(driver.poll(0), Err(DriverError::VerifierFault));
}
