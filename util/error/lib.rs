// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use core::num::NonZero;
use ufmt::{uDebug, uDisplay, uwrite};

mod flash;
mod ipc;
mod kernel;

pub use flash::*;
pub use ipc::*;
pub use kernel::*;

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

    pub const fn from_pw(self, code: u16, err: pw_status::Error) -> ErrorCode {
        // pw_status::Error is 5 bits.
        self.error((code << 5) | (err as u16))
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

    pub fn kernel_error(e: pw_status::Error) -> Self {
        KERNEL_ERROR.error(e as u16)
    }
}

impl From<ErrorCode> for u32 {
    fn from(e: ErrorCode) -> u32 {
        e.0.get()
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
