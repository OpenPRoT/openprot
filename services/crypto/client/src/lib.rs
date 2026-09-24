// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Crypto service client traits for OpenPRoT.
//!
//! Polled verification interface for firmware images. The FdOps
//! adapter calls `start` with a storage region, then polls until
//! the crypto service returns a verdict. Each `poll` drives one
//! chunk of hashing so the caller never blocks for the full image.

#![no_std]

/// Outcome of a completed verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Image is authentic.
    Pass,
    /// Image failed verification.
    Fail,
}

/// Progress of an in-flight verification.
///
/// After `Done`, further calls to `poll` return an error until the
/// next `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyPoll {
    /// Hashing in progress. `hashed` bytes of `total` processed so far.
    Progress { hashed: usize, total: usize },
    /// Verification finished.
    Done(Verdict),
}

/// Polled firmware image verification.
///
/// The caller provides a storage region (base address and size) at
/// start, then polls repeatedly. Each poll advances the hash by one
/// implementation-defined chunk so the caller can interleave other
/// work. The verifier reads the image itself; the caller never
/// feeds bytes.
pub trait VerifyClient {
    /// Error type for verification operations.
    type Error;

    /// Begin verifying `size` bytes starting at `addr`.
    ///
    /// `addr` and `size` name the image in the verifier's storage
    /// view; wiring configures both sides to agree on the address
    /// space. Resets any in-flight verification.
    fn start(&mut self, addr: usize, size: usize) -> Result<(), Self::Error>;

    /// Drive one chunk of hashing and report progress or the final
    /// verdict. Returns an error if called before `start`.
    fn poll(&mut self) -> Result<VerifyPoll, Self::Error>;
}
