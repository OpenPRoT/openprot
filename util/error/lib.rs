#![no_std]

use core::num::NonZero;

use ufmt::{uDebug, uDisplay, uwrite};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorModule(pub NonZero<u16>);

impl ErrorModule {
    pub const fn new(val: u16) -> Self {
        match NonZero::new(val) {
            Some(val) => Self(val),
            None => panic!("ErrorModule must be non-zero"),
        }
    }

    pub const fn error(self, code: u16) -> ErrorCode {
        ErrorCode::new(((self.0.get() as u32) << 16) | (code as u32))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorCode(pub NonZero<u32>);
impl ErrorCode {
    pub const fn new(val: u32) -> Self {
        match NonZero::new(val) {
            Some(val) => Self(val),
            None => panic!("ErrorCode must be non-zero"),
        }
    }
}

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

pub const KERNEL_ERROR: ErrorModule = ErrorModule::new(0x4b45); // ascii `KE`
pub const KERNEL_ERROR_CANCELLED: ErrorCode = KERNEL_ERROR.error(1);
pub const KERNEL_ERROR_UNKNOWN: ErrorCode = KERNEL_ERROR.error(2);
pub const KERNEL_ERROR_INVALID_ARGUMENT: ErrorCode = KERNEL_ERROR.error(3);
pub const KERNEL_ERROR_DEADLINE_EXCEEDED: ErrorCode = KERNEL_ERROR.error(4);
pub const KERNEL_ERROR_NOT_FOUND: ErrorCode = KERNEL_ERROR.error(5);
pub const KERNEL_ERROR_ALREADY_EXISTS: ErrorCode = KERNEL_ERROR.error(6);
pub const KERNEL_ERROR_PERMISSION_DENIED: ErrorCode = KERNEL_ERROR.error(7);
pub const KERNEL_ERROR_RESOURCE_EXHAUSTED: ErrorCode = KERNEL_ERROR.error(8);
pub const KERNEL_ERROR_FAILED_PRECONDITION: ErrorCode = KERNEL_ERROR.error(9);
pub const KERNEL_ERROR_ABORTED: ErrorCode = KERNEL_ERROR.error(10);
pub const KERNEL_ERROR_OUT_OF_RANGE: ErrorCode = KERNEL_ERROR.error(11);
pub const KERNEL_ERROR_UNIMPLEMENTED: ErrorCode = KERNEL_ERROR.error(12);
pub const KERNEL_ERROR_INTERNAL: ErrorCode = KERNEL_ERROR.error(13);
pub const KERNEL_ERROR_UNAVAILABLE: ErrorCode = KERNEL_ERROR.error(14);
pub const KERNEL_ERROR_DATA_LOSS: ErrorCode = KERNEL_ERROR.error(15);
pub const KERNEL_ERROR_UNAUTHENTICATED: ErrorCode = KERNEL_ERROR.error(16);

impl From<pw_status::Error> for ErrorCode {
    fn from(err: pw_status::Error) -> Self {
        KERNEL_ERROR.error(err as u16)
    }
}
