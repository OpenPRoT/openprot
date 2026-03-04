// Licensed under the Apache-2.0 license

//! AST1060-EVB I2C Service Target
//!
//! This target runs the I2C server and client as separate userspace processes
//! communicating over IPC channels.  The client exercises the server via
//! the I2C wire protocol, then calls debug_shutdown to report results.

#![no_std]
#![no_main]

use target_common::{TargetInterface, declare_target};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST1060-EVB I2C Service";

    fn main() -> ! {
        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        pw_log::info!("Shutting down with code {}", code as u32);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
