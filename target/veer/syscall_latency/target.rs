// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

use arch_riscv::Arch;
use entry::exit;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, kernel as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Earlgrey Syscall Latency Test";

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
        match code {
            0 => {
                pw_log::info!("PASS");
                exit(0);
            }
            _ => {
                pw_log::info!("FAIL: {}", code as u32);
                exit(code);
            }
        };
    }
}

declare_target!(Target);
