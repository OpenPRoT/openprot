// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use crate::*;
use openprot_orchestrator_sm::{
    ComponentAttrs, ComponentId, ComponentKind, Event, Orchestrator, PowerOnResult, State,
};
use orchestrator_capabilities::{BootWatch, FailureCause, WalkVerdict};

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

/// RAM-backed image source — the seam satisfied without a HAL.
struct MemImage {
    data: std::vec::Vec<u8>,
    fail_open: bool,
    fail_read: bool,
}

impl MemImage {
    fn holding(data: std::vec::Vec<u8>) -> Self {
        Self {
            data,
            fail_open: false,
            fail_read: false,
        }
    }
}

impl ImageSource for MemImage {
    type Error = MemFault;

    fn open(&mut self) -> Result<(), MemFault> {
        if self.fail_open {
            return Err(MemFault);
        }
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
            Verdict::Authenticated
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

/// Update adapter without a HAL. Wiring-only for now: it stages the whole
/// payload in one step. The update pump replaces it with a stepping mock
/// when the executors land.
struct MockUpdatable {
    ready: bool,
    active: bool,
}

impl MockUpdatable {
    fn new() -> Self {
        Self {
            ready: false,
            active: false,
        }
    }
}

impl orchestrator_capabilities::Updatable for MockUpdatable {
    fn poll_stage(
        &mut self,
        _payload: &dyn orchestrator_capabilities::PayloadSource,
    ) -> Result<orchestrator_capabilities::StageProgress, orchestrator_capabilities::UpdateError>
    {
        self.ready = true;
        Ok(orchestrator_capabilities::StageProgress::Ready)
    }

    fn abandon(&mut self) {
        self.ready = false;
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
    type Updatable = MockUpdatable;
}

fn driver(images: [MemImage; 1]) -> PlatformDriver<MockBoard, 1> {
    PlatformDriver::new(Board {
        images,
        verifier: XorVerifier { fault: false },
        boot_controls: [MockReset::new()],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    })
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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: true },
        boot_controls: [MockReset::new()],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    });

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
}

const C1: ComponentId = ComponentId::new(1);

#[test]
fn verify_for_a_different_component_is_refused() {
    let mut driver = PlatformDriver::<MockBoard, 2>::new(Board {
        images: [
            MemImage::holding(valid_image()),
            MemImage::holding(valid_image()),
        ],
        verifier: XorVerifier { fault: false },
        boot_controls: [MockReset::new(), MockReset::new()],
        boot_watches: [MockWalk::idle(), MockWalk::idle()],
        component_kinds: [ComponentKind::Passive, ComponentKind::Passive],
        updatables: [MockUpdatable::new(), MockUpdatable::new()],
    });

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
    let control = MockReset::new();
    let held = control.held.clone();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: false },
        boot_controls: [control],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    });

    driver.release_reset(C0).unwrap();
    assert!(!held.get());

    driver.assert_reset(C0).unwrap();
    assert!(held.get());
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
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: false },
        boot_controls: [control],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
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
    type Updatable = MockUpdatable;
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
        verifier: LineWatchingVerifier {
            inner: XorVerifier { fault: false },
            line: held.clone(),
            held_during_verify: held_during_verify.clone(),
        },
        boot_controls: [control],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    });
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
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: false },
        boot_controls: [control],
        boot_watches: [MockWalk::idle()],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    });
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
    PlatformDriver::new(Board {
        images: [
            MemImage::holding(valid_image()),
            MemImage::holding(valid_image()),
        ],
        verifier: XorVerifier { fault: false },
        boot_controls: [MockReset::new(), MockReset::new()],
        boot_watches: walks,
        component_kinds,
        updatables: [MockUpdatable::new(), MockUpdatable::new()],
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

// A failed walk becomes Timeout(id) regardless of cause — the retry
// decision is the SM's.
// TODO: the SM only knows Timeout, so DeviceFatal still spends retry
// budget. Add a fatal, unrecoverable-error event to the SM in a later PR.
#[test]
fn failed_walks_map_to_timeout() {
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

    assert_eq!(driver.poll_boot_walks(0).event, Some(Event::Timeout(C0)));
    assert_eq!(driver.poll_boot_walks(0).event, Some(Event::Timeout(C1)));
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
    assert_eq!(driver.poll_boot_walks(0).event, Some(Event::Timeout(C0)));

    driver.release_reset(C0).unwrap();
    assert_eq!(
        driver.poll_boot_walks(0).event,
        Some(Event::Timeout(C0)),
        "fresh attempt from the first checkpoint, not the old walk resumed"
    );
}

// End to end: a passive component is released speculatively (Ready), its
// walk completes, and the Booted event settles cleanly.
#[test]
fn booted_walk_settles_in_ready() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: false },
        boot_controls: [MockReset::new()],
        boot_watches: [MockWalk::scripted(std::vec![WalkVerdict::Complete])],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    });

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    let event = driver.poll_boot_walks(0).event.expect("walk completed");
    assert_eq!(event, Event::Booted(C0));
    orch.dispatch(&mut driver, event);

    assert_eq!(orch.state(), State::Ready);
}

// End to end, failure path: the released component never reports in, its
// Timeout enters recovery, and with no recovery capability composed yet
// the machine fails closed. The Recovery PR replaces this test with the
// recovery-path one — its failure there is the reminder.
#[test]
fn boot_timeout_fails_closed_without_recovery() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: false },
        boot_controls: [MockReset::new()],
        boot_watches: [MockWalk::scripted(std::vec![WalkVerdict::Failed {
            checkpoint: "heartbeat",
            cause: FailureCause::TimedOut,
        }])],
        component_kinds: [ComponentKind::Passive],
        updatables: [MockUpdatable::new()],
    });

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    let event = driver.poll_boot_walks(0).event.expect("walk failed");
    assert_eq!(event, Event::Timeout(C0));
    orch.dispatch(&mut driver, event);

    assert_eq!(orch.state(), State::Locked);
}

#[test]
fn submit_update_refuses_an_unknown_component() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(
        driver.submit_update(ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );
    assert_eq!(driver.pending_update(), None);
}

// Single update in flight by construction: a second submit is refused and
// the first job's target survives untouched.
#[test]
fn submit_update_refuses_a_second_in_flight() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    driver.submit_update(C0).unwrap();
    assert_eq!(driver.submit_update(C0), Err(DriverError::UpdateBusy));
    assert_eq!(driver.pending_update(), Some(C0));
}

// The frontend connection end to end: request_update records the job and
// the SM receives UpdateRequest. Ready accepts it and enters Updating,
// whose entry effects (AuthenticateUpdate, StageUpdate) have no executors
// yet, so the machine latches Locked — that latch is the proof the event
// arrived. Flips to an Updating/Ready assertion when the pump lands.
#[test]
fn request_update_reaches_the_sm() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));
    assert_eq!(orch.state(), State::Ready);

    request_update(&mut orch, &mut driver, C0).unwrap();

    assert_eq!(driver.pending_update(), Some(C0));
    assert_eq!(orch.state(), State::Locked);
}

// A refused submit injects nothing: no job, no event, the SM stays Ready.
#[test]
fn refused_request_update_injects_no_event() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);
    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(
        request_update(&mut orch, &mut driver, ComponentId::new(9)),
        Err(DriverError::UnknownComponent)
    );

    assert_eq!(driver.pending_update(), None);
    assert_eq!(orch.state(), State::Ready);
}

// The Updatable seam is wired but not yet driven: no executor exists until
// the update pump lands. This pins the mock against the trait's ordering
// rule so the wiring cannot rot in the meantime.
#[test]
fn updatable_seam_is_satisfiable_by_the_mock() {
    use orchestrator_capabilities::{PayloadReadError, PayloadSource, StageProgress, Updatable};

    struct SlicePayload(&'static [u8]);

    impl PayloadSource for SlicePayload {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadReadError> {
            let start = usize::try_from(offset).map_err(|_| PayloadReadError::OutOfRange)?;
            let end = start
                .checked_add(buf.len())
                .ok_or(PayloadReadError::OutOfRange)?;
            buf.copy_from_slice(self.0.get(start..end).ok_or(PayloadReadError::OutOfRange)?);
            Ok(())
        }
    }

    let mut dev = MockUpdatable::new();

    assert_eq!(
        dev.activate(),
        Err(orchestrator_capabilities::UpdateError::NothingStaged)
    );
    assert_eq!(
        dev.poll_stage(&SlicePayload(b"image")),
        Ok(StageProgress::Ready)
    );
    dev.activate().expect("activate failed");
    assert!(dev.active);
}
