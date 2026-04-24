// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error code handling.

#![cfg_attr(not(test), no_std)]

use core::num::NonZero;
use zerocopy::{Immutable, IntoBytes};

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
#[derive(Clone, Copy, PartialEq, Eq, IntoBytes, Immutable)]
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

    /// Converts an integer status code into a Result<(), ErrorCode>.
    /// The status code 0 represents Ok.
    /// All other values represent errors.
    pub fn check_status(status: u32) -> Result<(), Self> {
        match status {
            0 => Ok(()),
            _ => Err(Self::new(status)),
        }
    }
}

impl From<ErrorCode> for u32 {
    fn from(e: ErrorCode) -> u32 {
        e.0.get()
    }
}

pub trait AsStatus {
    fn as_status(&self) -> u32;
}

impl<T> AsStatus for Result<T, ErrorCode> {
    fn as_status(&self) -> u32 {
        match self {
            Ok(_) => 0,
            Err(e) => e.0.get(),
        }
    }
}

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

#[cfg(feature = "ufmt")]
const _: () = {
    // TODO: decice if we care about `ufmt` support.
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
};

#[cfg(test)]
mod tests {
    use super::*;
    use pw_status::Error;

    #[test]
    fn test_error_module_new() {
        let module = ErrorModule::new(0x1234);
        assert_eq!(module.0.get(), 0x1234);
    }

    #[test]
    #[should_panic(expected = "ErrorModule must be non-zero")]
    fn test_error_module_new_panic() {
        let _ = ErrorModule::new(0);
    }

    #[test]
    fn test_error_module_error() {
        let module = ErrorModule::new(0x1234);
        let err = module.error(0x5678);
        assert_eq!(err.0.get(), 0x12345678);
    }

    #[test]
    fn test_error_module_from_pw() {
        let module = ErrorModule::new(0x1234);
        let err = module.from_pw(1, Error::InvalidArgument);
        assert_eq!(
            err.0.get(),
            (0x1234 << 16) | (1 << 5) | (Error::InvalidArgument as u32)
        );
    }

    #[test]
    fn test_error_code_new() {
        let err = ErrorCode::new(0x12345678);
        assert_eq!(err.0.get(), 0x12345678);
    }

    #[test]
    #[should_panic(expected = "ErrorCode must be non-zero")]
    fn test_error_code_new_panic() {
        let _ = ErrorCode::new(0);
    }

    #[test]
    fn test_error_code_kernel_error() {
        let err = ErrorCode::kernel_error(Error::NotFound);
        assert_eq!(err.0.get(), (0x4b45 << 16) | (Error::NotFound as u32));
    }

    #[test]
    fn test_error_code_check_status() {
        assert!(ErrorCode::check_status(0).is_ok());

        let err = ErrorCode::check_status(0x12345678);
        assert!(err.is_err());
        assert_eq!(err.unwrap_err().0.get(), 0x12345678);
    }

    #[test]
    fn test_u32_from_error_code() {
        let err = ErrorCode::new(0x12345678);
        let val: u32 = err.into();
        assert_eq!(val, 0x12345678);
    }

    #[test]
    fn test_display_and_debug() {
        let err = ErrorCode::new(0x12345678);
        assert_eq!(format!("{err}"), "0x12345678");
        assert_eq!(format!("{err:?}"), "0x12345678");
    }
}
