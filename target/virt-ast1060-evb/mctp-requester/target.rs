// Licensed under the Apache-2.0 license

//! Virtual AST1060-EVB MCTP Requester Target (QEMU)
//!
//! Kernel entry shim for the virt-ast1060-evb mctp-requester system image.
//! Uses cortex-m-semihosting to signal test pass/fail back to QEMU.

#![no_std]
#![no_main]

use cortex_m_semihosting::debug::{EXIT_FAILURE, EXIT_SUCCESS, exit};
use target_common::{TargetInterface, declare_target};
use {console_backend as _, entry as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "virt-AST1060-EVB MCTP Requester";

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
