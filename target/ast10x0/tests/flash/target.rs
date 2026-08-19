// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 Flash Service Target
//!
//! This target runs the flash server as a userspace process.
//! Clients can communicate with it over an IPC channel.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use cortex_m_semihosting::debug::{exit, EXIT_FAILURE, EXIT_SUCCESS};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Flash Service";

    fn main() -> ! {
        // Static pinmux configuration, applied before any process starts so
        // no task ever needs SCU access (avoids cross-task RMW races on the
        // shared pinctrl registers).
        // SAFETY: kernel main() runs once, single-threaded, with exclusive
        // hardware ownership.
        let scu = unsafe { ScuRegisters::new_global_unlocked() };
        scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
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
