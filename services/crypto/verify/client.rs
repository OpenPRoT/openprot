// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Client seam for firmware verification by the crypto service.
//!
//! The caller names a staged image by flash address and length. The
//! crypto service reads those bytes from the flash server itself, so
//! firmware never crosses this interface and the caller never feeds
//! chunks. `poll` returns at once with whatever the service has
//! reported so far, which is what keeps PLDM-FD and the orchestrator
//! running while a multi-megabyte image is hashed.

#![no_std]

use hal_flash_driver::FlashAddress;

/// A staged image in the flash server's address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRegion {
    /// First byte of the image.
    pub address: FlashAddress,
    /// Image length in bytes. Reads stop here, so a staging region
    /// larger than the candidate does not pull in the bytes behind it.
    pub length: u32,
}

/// Outcome of a completed verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Signature and policy checks passed over the whole image.
    Authenticated,
    /// The image was checked and found invalid.
    Rejected,
}

/// What the crypto service has got through so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyPoll {
    /// Hashing is still running: `hashed` of `total` bytes are done.
    /// The numbers move only when the service has read and hashed
    /// more, so polls between two chunks repeat the same pair.
    Hashing { hashed: u32, total: u32 },
    /// Verification finished.
    Done(Verdict),
}

/// Firmware verification by the crypto service.
///
/// `start` hands over a region and returns once the request is sent,
/// not once it is answered. Each `poll` reports progress without
/// waiting on the service. After `Done`, `poll` errors until the next
/// `start`.
pub trait VerifyClient {
    /// Why verification could not run: unreadable flash, crypto fault,
    /// or a poll with nothing in flight. An image that fails its checks
    /// is [`Verdict::Rejected`], not an error.
    type Error;

    /// Asks the crypto service to verify `region`. Replaces any
    /// verification still in flight.
    fn start(&mut self, region: ImageRegion) -> Result<(), Self::Error>;

    /// Reports progress or the verdict, without waiting on the crypto
    /// service.
    fn poll(&mut self) -> Result<VerifyPoll, Self::Error>;
}
