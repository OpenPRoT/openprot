// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Console
//!
//! This crate provides basic console functionality.

#![no_std]

use core::convert::Infallible;

pub use ufmt;
use ufmt::uWrite;
pub use ufmt::{uwrite, uwriteln};

pub mod hexdump;

pub struct Console;

unsafe extern "C" {
    fn system_lowlevel_console_write(ptr: *const u8, length: usize);
}

impl uWrite for Console {
    type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Infallible> {
        // ok: unsafe-usage
        unsafe {
            // SAFETY: The pointer can never be null.
            system_lowlevel_console_write(s.as_ptr(), s.len())
        };
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use $crate::ufmt;
        $crate::uwrite!(&mut $crate::Console, $($arg)*).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use $crate::ufmt;
        $crate::ufmt::uwriteln!(&mut $crate::Console, $($arg)*).unwrap();
    }};
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if cfg!(feature = "trace") {
            use $crate::ufmt;
            $crate::uwrite!(&mut $crate::Console, $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! traceln {
    ($($arg:tt)*) => {
        if cfg!(feature = "trace") {
            use $crate::ufmt;
            $crate::uwriteln!(&mut $crate::Console, $($arg)*).unwrap();
        }
    };
}
