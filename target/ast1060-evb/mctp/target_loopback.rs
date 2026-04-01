// Licensed under the Apache-2.0 license

//! AST1060-EVB MCTP Loopback Test Target
//!
//! This target runs a single test application that manages two MCTP
//! servers internally using loopback transport (no I2C required).

#![no_std]
#![no_main]

use target_common::{TargetInterface, declare_target};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST1060-EVB MCTP Loopback Test";

    fn main() -> ! {
        codegen_loopback::start();
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
