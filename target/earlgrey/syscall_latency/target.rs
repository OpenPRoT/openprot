// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _, kernel as _};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Earlgrey Syscall Latency Test";

    fn main() -> ! {
        codegen::start();
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        pw_log::info!("Shutting down with code {}", code as u32);
        match code {
            0 => pw_log::info!("PASS"),
            _ => pw_log::info!("FAIL: {}", code as u32),
        };
        loop {}
    }
}

declare_target!(Target);
