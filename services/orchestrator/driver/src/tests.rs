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

    fn size(&mut self) -> Result<usize, MemFault> {
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

/// The test board's type choices.
struct MockBoard;

impl BoardCapabilities for MockBoard {
    type Image = MemImage;
    type Verifier = XorVerifier;
}

fn driver(images: [MemImage; 1]) -> PlatformDriver<MockBoard, 1> {
    PlatformDriver::new(Board {
        images,
        verifier: XorVerifier { fault: false },
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
// verdict comes back as an event.
#[test]
fn boot_verifies_the_first_component() {
    let mut orch = orchestrator();
    let mut driver = driver([MemImage::holding(valid_image())]);

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(driver.take_event(), Some(Event::VerificationPassed(C0)));
    assert_eq!(driver.take_event(), None);
    assert_eq!(orch.state(), State::PreSupervision);
}

#[test]
fn corrupt_image_fails_verification() {
    let mut corrupt = valid_image();
    corrupt[7] ^= 0x01;
    let mut driver = driver([MemImage::holding(corrupt)]);

    driver.stage_firmware(C0).unwrap();
    driver.verify_firmware(C0).unwrap();

    assert_eq!(driver.take_event(), Some(Event::VerificationFailed(C0)));
}

#[test]
fn verify_without_read_is_refused() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    assert_eq!(driver.verify_firmware(C0), Err(DriverError::NotStaged));
    assert_eq!(driver.take_event(), None);
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
    assert_eq!(driver.take_event(), None);
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
    assert_eq!(driver.take_event(), None);
}

// So does a verifier that cannot run its check.
#[test]
fn verifier_fault_fails_closed() {
    let mut orch = orchestrator();
    let mut driver = PlatformDriver::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: true },
    });

    orch.dispatch(&mut driver, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
    assert_eq!(driver.take_event(), None);
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

// Undrained verdicts eventually fill the queue; the overflow is reported,
// not silently dropped.
#[test]
fn event_queue_overflow_is_reported() {
    let mut driver = driver([MemImage::holding(valid_image())]);

    let mut queued = 0;
    loop {
        driver.stage_firmware(C0).unwrap();
        match driver.verify_firmware(C0) {
            Ok(()) => queued += 1,
            Err(e) => {
                assert_eq!(e, DriverError::QueueFull);
                break;
            }
        }
        assert!(queued < 64, "queue never filled");
    }
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
    assert_eq!(driver.take_event(), None);
}
