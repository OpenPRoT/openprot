// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The board stubs every integration scenario needs and none of them
//! vary: [`BoardFault`], [`StubVerifier`] and [`StubImage`].
//!
//! A scenario board has to name a type at every capability seam, even the
//! seams it does not exercise. Signature checking is one: the crypto
//! service does not exist yet, so each scenario stubbed it, and the three
//! copies were identical down to the comments. They live here so a change
//! to the `Verifier` or `ImageSource` contract is one edit.

#![no_std]

use core::sync::atomic::{AtomicU32, Ordering};

use openprot_orchestrator_driver::{ImageSource, Verdict, Verifier};
use openprot_orchestrator_sm::ComponentId;
use orchestrator_capabilities::{AuthenticatedImage, LockdownLatch};

/// One board fault, for every seam that has to name an error type. A
/// scenario cares whether an actuation failed, never which one.
#[derive(Debug)]
pub struct BoardFault;

impl core::fmt::Display for BoardFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("board fault")
    }
}

impl core::error::Error for BoardFault {}

/// Signature checking is stubbed until the crypto service exists.
pub struct StubVerifier;

impl Verifier for StubVerifier {
    type Error = BoardFault;

    fn verify(
        &mut self,
        _id: ComponentId,
        _image: &mut impl ImageSource,
    ) -> Result<Verdict, BoardFault> {
        Ok(Verdict::Authenticated(AuthenticatedImage {
            // The stub verifier reads nothing: no manifest to take an SVN
            // from, and no hash engine to measure with.
            svn: None,
            measurement: None,
        }))
    }
}

/// The image a board names for a component whose bytes nothing reads.
/// [`StubVerifier`] authenticates without opening it, so `read_at` is the
/// one call that faults rather than serving bytes no image holds.
pub struct StubImage;

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

/// How many times the platform latched into its safe state, which is what
/// a scenario asserts. A static because the latch moves into the board and
/// the board into the driver, while the run loop that judges it is outside
/// both.
static LATCHES: AtomicU32 = AtomicU32::new(0);

/// Times the platform was latched in this run.
pub fn latches() -> u32 {
    LATCHES.load(Ordering::Relaxed)
}

/// The terminal safe state, counted instead of actuated.
///
/// A real one holds every managed device in reset and denies writes in the
/// SPI monitor, and answers `Ok` only once both are in force. QEMU models
/// one chip and the scenarios have no second device to hold, so what they
/// claim is narrower: that the orchestrator engaged the safe state exactly
/// when it said it would, and never in a run that was supposed to come up.
pub struct CountingLatch;

impl LockdownLatch for CountingLatch {
    type Error = BoardFault;

    fn latch(&mut self) -> Result<(), BoardFault> {
        LATCHES.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
