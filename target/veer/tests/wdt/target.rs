// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Starts, restarts and stops the MCU watchdog of the emulator or the FPGA
//! with the watchdog driver.
//!
//! Timer 2 has a period that it does not reach during the test, so the
//! watchdog does not raise its NMI. The emulator needs the long period: when a
//! timer 1 timeout is cleared, the hardware restarts timer 2, but the emulator
//! lets it keep running.

#![no_std]
#![no_main]

use core::ptr::with_exposed_provenance;

use entry::exit;
use kernel_config::{MCI_BASE, MTIME_REGISTER};
use target_common::{declare_target, TargetInterface};
use veer_wdt::Wdt;
use {codegen as _, console_backend as _};

/// Timer 1 period in clock cycles: 10 ms on the FPGA.
const PERIOD: u32 = 200_000;

/// Timer 2 period in clock cycles: more than 3 minutes on the FPGA.
const TIMER2_PERIOD: u64 = u32::MAX as u64;

/// The low word of `mtime`, which counts the same clock cycles as the
/// watchdog.
fn mtime() -> u32 {
    // SAFETY: `MTIME_REGISTER` is a readable register of the MCI, which this
    // code can access because it runs in the kernel.
    unsafe { with_exposed_provenance::<u32>(MTIME_REGISTER).read_volatile() }
}

fn wait(cycles: u32) {
    let start = mtime();
    while mtime().wrapping_sub(start) < cycles {
        core::hint::spin_loop();
    }
}

fn run(wdt: &mut Wdt) -> Result<(), &'static str> {
    if !wdt.is_enabled() {
        return Err("the ROM did not leave the watchdog running");
    }
    wdt.stop();
    if wdt.is_enabled() {
        return Err("stop left the watchdog enabled");
    }

    wdt.start(u64::from(PERIOD), TIMER2_PERIOD);
    if !wdt.is_enabled() {
        return Err("start did not enable the watchdog");
    }
    // Restarts keep the watchdog from timing out for several periods.
    for _ in 0..8 {
        wait(PERIOD / 2);
        if wdt.timer1_expired() {
            return Err("timer 1 timed out although it was restarted");
        }
        wdt.restart();
    }

    wait(2 * PERIOD);
    if !wdt.timer1_expired() {
        return Err("timer 1 did not time out");
    }
    if wdt.timer2_expired() {
        return Err("timer 2 timed out");
    }
    wdt.restart();
    if wdt.timer1_expired() {
        return Err("restart did not clear the timeout");
    }
    wait(PERIOD / 2);
    if wdt.timer1_expired() {
        return Err("timer 1 timed out early after the timeout was cleared");
    }

    wdt.stop();
    wait(2 * PERIOD);
    if wdt.timer1_expired() {
        return Err("timer 1 timed out while stopped");
    }
    Ok(())
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU Watchdog";

    fn main() -> ! {
        // SAFETY: this code runs in the kernel, which can access the MCI
        // registers, and this image has no other user of the watchdog.
        let mut wdt = unsafe { Wdt::new(MCI_BASE) };
        match run(&mut wdt) {
            Ok(()) => {
                pw_log::info!("PASS");
                exit(0);
            }
            Err(reason) => {
                pw_log::info!("FAIL: {}", reason as &str);
                exit(1);
            }
        }
    }
}

declare_target!(Target);
