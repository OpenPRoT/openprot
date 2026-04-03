// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

use console_backend as _;
use entry::exit;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra MCU Userspace IPC";

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
        exit(code);
    }
}

declare_target!(Target);
