// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The OpenPRoT runtime image for the Caliptra MCU.
//!
//! The caliptra-mcu-sw ROM loads this image into MCU SRAM and jumps to it.
//! When the kernel has started, `init_hardware` sets up the hardware that the
//! image uses. `system.json5.tmpl` lists the services of the image.

#![no_std]
#![no_main]

use arch_riscv::Arch;
use target_common::{declare_target, TargetInterface};
use veer_wdt::Wdt;
use {codegen as _, console_backend as _, entry as _};

/// Sets up the hardware. It runs in the kernel's bootstrap thread, before any
/// service starts.
fn init_hardware() {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers, and no other code in this image uses the watchdog.
    let mut wdt = unsafe { Wdt::new(kernel_config::MCI_BASE) };
    // The ROM leaves the watchdog running, and this image has no code that
    // restarts it.
    wdt.stop();
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "OpenPRoT Caliptra MCU Runtime";

    fn main() -> ! {
        init_hardware();
        pw_log::info!("OpenPRoT Caliptra MCU runtime: kernel started");
        // The bootstrap thread has nothing left to do.
        loop {
            let _ = kernel::sleep_until(Arch, kernel::Instant::MAX);
        }
    }
}

declare_target!(Target);
