// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Capability traits for word-addressed one-time-programmable (OTP) memory.
//!
//! One trait object covers one OTP region (config, data, …); a driver exposes
//! an accessor per region it serves. Reading and programming are separate
//! capabilities, so a consumer that only reads (an SVN floor check, say) can
//! never be handed programming rights. The traits know nothing about what the
//! words mean — key hashes, straps, monotonic counters are policy that lives
//! in the service on top.

#![cfg_attr(not(test), no_std)]

/// Generic OTP failure modes; implementations map their error types onto
/// these.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum OtpErrorKind {
    /// Word index is outside the region.
    OutOfBounds,
    /// The requested value would reverse already-programmed bits.
    AlreadyProgrammed,
    /// Post-program read-back did not match the requested value.
    VerifyFailed,
    /// The word or region is locked against programming.
    Locked,
    /// Hardware failure during the operation.
    HardwareFailure,
    /// The operation timed out.
    Timeout,
}

/// Trait for OTP errors: convert an implementation error to a generic kind.
pub trait OtpError: core::fmt::Debug {
    /// The generic failure mode this error maps to.
    fn kind(&self) -> OtpErrorKind;
}

impl OtpError for core::convert::Infallible {
    fn kind(&self) -> OtpErrorKind {
        match *self {}
    }
}

/// Shared error type for the OTP capability traits.
pub trait OtpErrorType {
    /// The error type returned by OTP operations.
    type Error: OtpError;
}

impl<T: OtpErrorType + ?Sized> OtpErrorType for &mut T {
    type Error = T::Error;
}

/// Read capability over one OTP region, addressed in 32-bit words.
pub trait OtpRead: OtpErrorType {
    /// Number of 32-bit words in this region.
    fn words(&self) -> usize;

    /// Read the word at `index`.
    fn read_word(&mut self, index: usize) -> Result<u32, Self::Error>;
}

impl<T: OtpRead + ?Sized> OtpRead for &mut T {
    #[inline(always)]
    fn words(&self) -> usize {
        (**self).words()
    }
    #[inline(always)]
    fn read_word(&mut self, index: usize) -> Result<u32, Self::Error> {
        (**self).read_word(index)
    }
}

/// Program capability over one OTP region. A consumer bound only on
/// [`OtpRead`] cannot program.
pub trait OtpProgram: OtpRead {
    /// Program `value` into the word at `index` and verify by read-back.
    ///
    /// Fails with [`OtpErrorKind::AlreadyProgrammed`] if `value` would
    /// reverse already-programmed bits, leaving the word unchanged.
    /// Re-programming the value a word already holds succeeds.
    fn program_word(&mut self, index: usize, value: u32) -> Result<(), Self::Error>;
}

impl<T: OtpProgram + ?Sized> OtpProgram for &mut T {
    #[inline(always)]
    fn program_word(&mut self, index: usize, value: u32) -> Result<(), Self::Error> {
        (**self).program_word(index, value)
    }
}
