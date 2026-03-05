// Licensed under the Apache-2.0 license

//! AST1060-EVB I2C Slave Echo Target

#![no_std]
#![no_main]

use target_common::{TargetInterface, declare_target};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST1060-EVB I2C Slave Echo";

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
