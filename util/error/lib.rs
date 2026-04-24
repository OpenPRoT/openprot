// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error code handling.

#![no_std]

use core::num::NonZero;

mod flash;
mod ipc;
mod kernel;

pub use flash::*;
pub use ipc::*;
pub use kernel::*;

/// Represents an error module.
///
/// An error module is a non-zero 16-bit identifier that categorizes a set of
/// error codes.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorModule(pub NonZero<u16>);

impl ErrorModule {
    /// Creates a new `ErrorModule`.
    ///
    /// # Panics
    /// Panics if `val` is zero.
    pub const fn new(val: u16) -> Self {
        match NonZero::new(val) {
            Some(val) => Self(val),
            None => panic!("ErrorModule must be non-zero"),
        }
    }

    /// Creates an `ErrorCode` within this module.
    ///
    /// The resulting `ErrorCode` will have the module ID in the upper 16 bits
    /// and the provided `code` in the lower 16 bits.
    pub const fn error(self, code: u16) -> ErrorCode {
        ErrorCode::new(((self.0.get() as u32) << 16) | (code as u32))
    }

    /// Creates an `ErrorCode` from a Pigweed status.
    ///
    /// This is a convenience method for creating error codes that incorporate
    /// a Pigweed status.
    pub const fn from_pw(self, code: u16, err: pw_status::Error) -> ErrorCode {
        // pw_status::Error is 5 bits.
        self.error((code << 5) | (err as u16))
    }
}

/// A 32-bit error code.
///
/// An error code consists of a 16-bit module ID and a 16-bit module-specific
/// error value.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorCode(pub NonZero<u32>);

impl ErrorCode {
    /// Creates a new `ErrorCode`.
    ///
    /// # Panics
    /// Panics if `val` is zero.
    pub const fn new(val: u32) -> Self {
        match NonZero::new(val) {
            Some(val) => Self(val),
            None => panic!("ErrorCode must be non-zero"),
        }
    }

    /// Creates a kernel error code from a Pigweed status.
    pub fn kernel_error(e: pw_status::Error) -> Self {
        KERNEL_ERROR.error(e as u16)
    }
}

impl From<ErrorCode> for u32 {
    fn from(e: ErrorCode) -> u32 {
        e.0.get()
    }
}

/*
 * TODO: decide if we want ufmt or not.
use ufmt::{uDebug, uDisplay, uwrite};
impl uDisplay for ErrorCode {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        uwrite!(f, "0x{:x}", self.0.get())
    }
}

impl uDebug for ErrorCode {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        uDisplay::fmt(self, f)
    }
}
*/

impl core::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:x}", self.0.get())
    }
}

impl core::fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl core::error::Error for ErrorCode {}
