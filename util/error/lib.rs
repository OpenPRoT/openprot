// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Error code handling.

#![cfg_attr(not(test), no_std)]

use core::num::NonZero;
use zerocopy::{Immutable, IntoBytes};

mod flash;
mod ipc;
mod kernel;
mod spi;

pub use flash::*;
pub use ipc::*;
pub use kernel::*;
pub use spi::*;

/// Represents an error module.
///
/// An error module is a non-zero 16-bit identifier that categorizes a set of
/// error codes.
#[derive(Clone, Copy, Debug)]
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
    ///
    /// While not strictly illegal, you are discouraged from creating an error
    /// with `code` zero.  Zero should be reserved for a truly unknown condition.
    /// Zero is permitted so that we can read a hardware error status register
    /// that might report zero.
    pub const fn error(self, code: u16) -> ErrorCode {
        ErrorCode::new(((self.0.get() as u32) << 16) | (code as u32))
    }

    /// Creates an `ErrorCode` from a Pigweed status.
    ///
    /// This is a convenience method for creating error codes that incorporate
    /// a Pigweed status.
    ///
    /// The resulting `ErrorCode` layout:
    /// - Bits 16..31: Module ID
    /// - Bits 8..15: The provided `code` (shifted up by 8 bits for readability in hex)
    /// - Bits 0..7: The Pigweed `err` status (normally fits in 5 bits)
    ///
    /// # Panics
    /// Panics if `code` is zero.
    pub const fn from_pw(self, code: u8, err: pw_status::Error) -> ErrorCode {
        // pw_status::Error is 5 bits, but we shift the error code up 8 bits
        // so the hex representation is easy to read..
        match NonZero::new(code as u16) {
            Some(val) => self.error((val.get() << 8) | (err as u16)),
            None => panic!("Error `code` must be non-zero"),
        }
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

    /// Returns the 16-bit module ID of this error code.
    pub fn module(self) -> u16 {
        (self.0.get() >> 16) as u16
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

    /// Extracts the Pigweed status from this error code.
    ///
    /// This assumes the error code was created in a way that embeds a Pigweed status
    /// in the lower bits (e.g., via [`ErrorModule::from_pw`] or [`ErrorCode::kernel_error`]).
    ///
    /// Invalid Pigweed status values are converted to [`pw_status::Error::Unknown`].
    pub fn as_pwerr(self) -> pw_status::Error {
        pw_status::Error::try_from(self.0.get() & 0x1f).unwrap_or(pw_status::Error::Unknown)
    }

    /// Extracts the module-specific error kind from this error code.
    ///
    /// This assumes the error code was created via [`ErrorModule::from_pw`], which
    /// embeds a `code` in bits 8..15.
    ///
    /// The extracted `code` is converted to `KIND` via `From<u32>`.
    /// Handling of unknown or invalid values is specific to the `KIND` implementation.
    ///
    /// TODO: Create a trait for all other `ErrorKind`s and constrain on that trait here.
    pub fn as_kind<KIND: From<u32>>(self) -> KIND {
        KIND::from((self.0.get() >> 8) & 0xFF)
    }
}

impl core::cmp::PartialEq<ErrorModule> for ErrorModule {
    fn eq(&self, other: &ErrorModule) -> bool {
        self.0.get() == other.0.get()
    }
}
impl core::cmp::PartialEq<ErrorModule> for u16 {
    fn eq(&self, other: &ErrorModule) -> bool {
        *self == other.0.get()
    }
}
impl core::cmp::PartialEq<u16> for ErrorModule {
    fn eq(&self, other: &u16) -> bool {
        self.0.get() == *other
    }
}
impl core::cmp::Eq for ErrorModule {}

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

/// Macro to define an error wrapper around `ErrorCode`.
#[macro_export]
macro_rules! error_wrapper {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct $name(pub $crate::ErrorCode);

        impl ::core::ops::Deref for $name {
            type Target = $crate::ErrorCode;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::core::convert::From<$crate::ErrorCode> for $name {
            #[inline(always)]
            fn from(err: $crate::ErrorCode) -> Self {
                Self(err)
            }
        }

        impl ::core::convert::From<$name> for $crate::ErrorCode {
            #[inline(always)]
            fn from(wrapper: $name) -> Self {
                wrapper.0
            }
        }

        impl $crate::AsStatus for $name {
            #[inline(always)]
            fn as_status(&self) -> u32 {
                self.0 .0.get()
            }
        }
    };
}

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
            (0x1234 << 16) | (1 << 8) | (Error::InvalidArgument as u32)
        );
    }

    #[test]
    #[should_panic(expected = "Error `code` must be non-zero")]
    fn test_error_module_from_pw_panic() {
        let module = ErrorModule::new(0x1234);
        let _ = module.from_pw(0, Error::InvalidArgument);
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
    fn test_error_code_module() {
        let err = ErrorCode::new(0x12345678);
        assert_eq!(err.module(), 0x1234);
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
    fn test_error_code_as_pwerr() {
        let module = ErrorModule::new(0x1234);
        let err = module.from_pw(1, Error::InvalidArgument);
        assert_eq!(err.as_pwerr(), Error::InvalidArgument);

        let err_kernel = ErrorCode::kernel_error(Error::NotFound);
        assert_eq!(err_kernel.as_pwerr(), Error::NotFound);

        let err_invalid = KERNEL_ERROR.error(31);
        assert_eq!(err_invalid.as_pwerr(), Error::Unknown);
    }

    #[derive(Debug, PartialEq, Eq)]
    enum MyKind {
        Zero,
        One,
        Unknown(u32),
    }
    impl From<u32> for MyKind {
        fn from(val: u32) -> Self {
            match val {
                0 => MyKind::Zero,
                1 => MyKind::One,
                _ => MyKind::Unknown(val),
            }
        }
    }

    #[test]
    fn test_error_code_as_kind() {
        let module = ErrorModule::new(0x1234);
        let err = module.from_pw(1, Error::InvalidArgument);
        assert_eq!(err.as_kind::<MyKind>(), MyKind::One);

        let err2 = module.from_pw(2, Error::InvalidArgument);
        assert_eq!(err2.as_kind::<MyKind>(), MyKind::Unknown(2));
    }

    #[test]
    fn test_error_module_eq() {
        let m1 = ErrorModule::new(0x1234);
        let m2 = ErrorModule::new(0x1234);
        let m3 = ErrorModule::new(0x5678);

        assert_eq!(m1, m2);
        assert_ne!(m1, m3);

        assert_eq!(m1, 0x1234);
        assert_eq!(0x1234, m1);
        assert_ne!(m1, 0x5678);
        assert_ne!(0x5678, m1);
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
    }

    error_wrapper!(TestWrapper);

    #[test]
    fn test_error_wrapper() {
        let err = ErrorCode::new(0x12345678);
        let wrapper = TestWrapper(err);

        assert_eq!(wrapper.0 .0.get(), 0x12345678);
        assert_eq!(wrapper.module(), 0x1234);

        let err2: ErrorCode = wrapper.into();
        assert_eq!(err2.0.get(), 0x12345678);

        let wrapper2 = TestWrapper::from(err);
        assert_eq!(wrapper2.0 .0.get(), 0x12345678);

        assert_eq!(wrapper.as_status(), 0x12345678);
    }
}
