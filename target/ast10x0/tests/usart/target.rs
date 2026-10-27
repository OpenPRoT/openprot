// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 USART Service Target
//!
//! This target runs the USART server as a userspace process.
//! Clients can communicate with it over an IPC channel.

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use cortex_m_semihosting::debug::{exit, EXIT_FAILURE, EXIT_SUCCESS};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 USART Service";

    fn main() -> ! {
        codegen::start();
        // The bootstrap thread has nothing left to do. Sleep rather than spin: a
        // spinning thread at the same priority as the app threads takes a time
        // slice at every scheduler tick and delays every wake-up behind it.
        loop {
            let _ = kernel::sleep_until(Arch, kernel::Instant::MAX);
        }
    }

    fn shutdown(code: u32) -> ! {
        let status = if code == 0 {
            EXIT_SUCCESS
        } else {
            EXIT_FAILURE
        };
        exit(status);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
