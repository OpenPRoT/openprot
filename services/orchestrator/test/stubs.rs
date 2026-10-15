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

use openprot_orchestrator_driver::{ImageSource, Verdict, Verifier};
use openprot_orchestrator_sm::ComponentId;
use orchestrator_capabilities::AuthenticatedImage;

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
