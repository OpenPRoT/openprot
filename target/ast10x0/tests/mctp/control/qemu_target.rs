// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Kernel target for the MCTP control responder QEMU test.
//!
//! No I2C initialization: the fake server stands in for the bus, so this
//! kernel only starts the two processes.
//!
//! `fake_server_main.rs` ends the test with `syscall::debug_shutdown`, which
//! reaches [`Target::shutdown`]; that writes the `TEST_RESULT:` sentinel
//! `qemu_runner.py` reads from the UART.

#![no_std]
#![no_main]

use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 MCTP control responder test";

    fn main() -> ! {
        codegen::start();
        #[expect(clippy::empty_loop)]
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        let sentinel: &[u8] = if code == 0 {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
