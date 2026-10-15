// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 reset-passthrough test, RoT side.
//!
//! The RoT cannot reach the mock BMC's reset line directly, so it raises GPIOJ0 and the Pi harness
//! mirrors that level onto the mock BMC's SRST. GPIOH4 carries the mock BMC's heartbeat back: a line
//! that keeps toggling means it is running, and a line that has gone static is how this side observes
//! that the reset actually landed. A steady level would not do, because an undriven line reads low
//! and so a mock BMC that has not booted yet is indistinguishable from one that is alive.

#![no_std]
#![no_main]

use ast10x0_board::delay_us;
use ast10x0_peripherals::aperture::take_aperture;
use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::gpio::{GpioBlock, IntoGpio, OutputPin};
use ast10x0_peripherals::scu;
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

const SAMPLE_MICROS: u32 = 100;
const WINDOW_SAMPLES: u32 = 2_000;
/// `delay_us` is a calibration-free busy loop and runs about three times fast, so these two are
/// sized by what they measure on the bench rather than by their nominal microseconds.
const BOOT_MICROS: u32 = 20_000_000;
const RESET_MICROS: u32 = 3_000_000;

fn run_test() -> bool {
    // SAFETY: created once, exclusive SoC access; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    // SAFETY: kernel-only binary, minted once; no process holds a conflicting grant.
    let gpio = GpioBlock::new(unsafe { take_aperture() });
    let request = pins.scu418_8.into_gpio(&gpio);
    let heartbeat = pins.scu414_28.into_gpio(&gpio);
    scu::route(&(&request, &heartbeat));

    let mut request = request.into_output();
    let _ = request.set_low();
    let heartbeat = heartbeat.into_input();

    pw_log::info!("=== AST10x0 reset passthrough test (RoT) ===");

    let toggling = || {
        let first = heartbeat.read(heartbeat.map().in_level);
        let mut changed = false;
        for _ in 0..WINDOW_SAMPLES {
            changed |= heartbeat.read(heartbeat.map().in_level) != first;
            delay_us(SAMPLE_MICROS);
        }
        changed
    };

    // Held low throughout, which is also how the Pi arms its mirror.
    pw_log::info!("waiting for the harness to upload and boot the mock BMC");
    delay_us(BOOT_MICROS);

    if !toggling() {
        pw_log::error!("mock BMC never started heartbeating");
        return false;
    }
    pw_log::info!("mock BMC is alive; requesting a reset");

    let _ = request.set_high();
    delay_us(RESET_MICROS);
    let still_beating = toggling();
    let _ = request.set_low();

    if still_beating {
        pw_log::error!("mock BMC kept heartbeating; the reset request never reached it");
        return false;
    }
    pw_log::info!("mock BMC stopped heartbeating: the reset request reached it");
    true
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 reset passthrough test (RoT)";

    fn main() -> ! {
        let sentinel = if run_test() {
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
