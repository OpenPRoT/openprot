// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use crate::*;
use openprot_orchestrator_sm::{
    ComponentAttrs, ComponentId, Event, Orchestrator, PowerOnResult, State,
};

const C0: ComponentId = ComponentId::new(0);

// Test image convention, shared with the fwmanager itests: 4 magic bytes,
// payload, and a final byte making the XOR over the whole image zero. A
// board-side stand-in for signature + SVN verification — deliberately
// defined here, not in the shell.
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

/// RAM-backed image — the source seam must be satisfiable without a HAL,
/// exactly as a PLDM-stream-backed source would satisfy it without flash.
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

/// The magic + XOR-zero check as a board-supplied verifier, streaming the
/// image from its source in chunks.
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
            Verdict::Authentic
        } else {
            Verdict::Rejected
        })
    }
}

/// The test board, naming its type choices once.
struct MockBoard;

impl BoardTypes for MockBoard {
    type Image = MemImage;
    type Verifier = XorVerifier;
}

fn shell(images: [MemImage; 1]) -> Shell<MockBoard, 1> {
    Shell::new(Board {
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

// Power-on drives the SM's ReadFirmware + VerifyFirmware into the shell;
// the shell owes the verdict back as an event.
#[test]
fn boot_verifies_the_first_component() {
    let mut orch = orchestrator();
    let mut shell = shell([MemImage::holding(valid_image())]);

    orch.dispatch(&mut shell, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(shell.take_event(), Some(Event::VerificationPassed(C0)));
    assert_eq!(shell.take_event(), None);
    assert_eq!(orch.state(), State::PreSupervision);
}

#[test]
fn corrupt_image_fails_verification() {
    let mut corrupt = valid_image();
    corrupt[7] ^= 0x01;
    let mut shell = shell([MemImage::holding(corrupt)]);

    shell.read_firmware(C0).unwrap();
    shell.verify_firmware(C0).unwrap();

    assert_eq!(shell.take_event(), Some(Event::VerificationFailed(C0)));
}

#[test]
fn verify_without_read_is_refused() {
    let mut shell = shell([MemImage::holding(valid_image())]);

    assert_eq!(shell.verify_firmware(C0), Err(ShellError::NoImage));
    assert_eq!(shell.take_event(), None);
}

// A source that cannot be opened is a failed actuation, not a verdict: the
// SM latches Locked instead of receiving a forged VerificationFailed.
#[test]
fn unopenable_source_fails_closed() {
    let mut orch = orchestrator();
    let mut image = MemImage::holding(valid_image());
    image.fail_open = true;
    let mut shell = shell([image]);

    orch.dispatch(&mut shell, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
    assert_eq!(shell.take_event(), None);
}

// Same for a source that opens but cannot be streamed: the verifier reports
// it could not perform the check.
#[test]
fn unreadable_source_fails_closed() {
    let mut orch = orchestrator();
    let mut image = MemImage::holding(valid_image());
    image.fail_read = true;
    let mut shell = shell([image]);

    orch.dispatch(&mut shell, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
    assert_eq!(shell.take_event(), None);
}

// And for a verifier that cannot perform its check at all.
#[test]
fn verifier_fault_fails_closed() {
    let mut orch = orchestrator();
    let mut shell = Shell::<MockBoard, 1>::new(Board {
        images: [MemImage::holding(valid_image())],
        verifier: XorVerifier { fault: true },
    });

    orch.dispatch(&mut shell, Event::PowerGood(PowerOnResult::Provisioned));

    assert_eq!(orch.state(), State::Locked);
    assert_eq!(shell.take_event(), None);
}
