// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 User IPC";

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
        pw_log::info!("Shutting down with code {}", code as u32);
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
