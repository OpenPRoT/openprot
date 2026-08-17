// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use crate::*;
use openprot_orchestrator_sm::{
    ComponentAttrs, ComponentId, Event, Orchestrator, PowerOnResult, State,
};

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
struct VerifierBroken;

impl core::fmt::Display for VerifierBroken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("verifier broken")
    }
}

impl core::error::Error for VerifierBroken {}

/// The magic + XOR-zero check as a board-supplied verifier, reading in
/// chunks.
struct XorVerifier {
    fault: bool,
}

impl Verifier for XorVerifier {
    type Error = VerifierBroken;

    fn verify(
        &mut self,
        _id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, VerifierBroken> {
        if self.fault {
            return Err(VerifierBroken);
        }
        let len = image.size().map_err(|_| VerifierBroken)?;
        let mut magic = [0u8; 4];
        let mut xor = 0u8;
        let mut offset = 0;
        let mut chunk = [0u8; 4];
        while offset < len {
            let take = chunk.len().min(len - offset);
            image
                .read_at(offset, &mut chunk[..take])
                .map_err(|_| VerifierBroken)?;
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

/// The test board's type choices.
struct MockBoard;

impl BoardCapabilities for MockBoard {
    type Image = MemImage;
    type Verifier = XorVerifier;
    type BootControl = MockReset;
}

fn driver(images: [MemImage; 1]) -> PlatformDriver<MockBoard, 1> {
    PlatformDriver::new(Board {
        images,
        verifier: XorVerifier { fault: false },
        boot_controls: [MockReset::new()],
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
    type Error = VerifierBroken;

    fn verify(
        &mut self,
        id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, VerifierBroken> {
        self.held_during_verify.set(self.line.get());
        self.inner.verify(id, image)
    }
}

struct WatchBoard;

impl BoardCapabilities for WatchBoard {
    type Image = MemImage;
    type Verifier = LineWatchingVerifier;
    type BootControl = MockReset;
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
    });
    let mut orch = orchestrator();

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
    assert!(held.get(), "never left reset");
}
