// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use console_backend as _;
use cortex_m_semihosting::debug::{EXIT_FAILURE, EXIT_SUCCESS, exit};
use entry as _;
use target_common::{TargetInterface, declare_target};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 User Interrupts";

    fn main() -> ! {
        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        pw_log::info!("Shutting down with code {}", code as u32);
        let status = match code {
            0 => EXIT_SUCCESS,
            _ => EXIT_FAILURE,
        };
        exit(status);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
