// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use arch_riscv::Arch;
use console_backend as _;
use entry::exit;
use target_common::{declare_target, TargetInterface};
use veer_wdt::Wdt;

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU MCU mailbox";

    fn main() -> ! {
        // The ROM leaves the watchdog running, and nothing in this image
        // restarts it. The host decides how long the test runs.
        //
        // SAFETY: this code runs in the kernel, which can access the MCI
        // registers, and no other code in this image uses the watchdog.
        unsafe { Wdt::new(kernel_config::MCI_BASE) }.stop();
        codegen::start();
        // The bootstrap thread has nothing left to do.
        loop {
            let _ = kernel::sleep_until(Arch, kernel::Instant::MAX);
        }
    }

    fn shutdown(code: u32) -> ! {
        match code {
            0 => pw_log::info!("PASS"),
            _ => pw_log::info!("FAIL: {}", code as u32),
        };
        exit(code);
    }
}

declare_target!(Target);
