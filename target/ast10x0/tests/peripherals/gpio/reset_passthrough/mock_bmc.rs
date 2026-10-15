// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 reset-passthrough test, mock BMC side.
//!
//! Toggles GPIOH5 for the rest of the run to tell the RoT it is alive. Toggling rather than holding
//! a level is the whole job: when the RoT's reset request lands, the chip stops driving the line, and
//! a line that has gone static is what the RoT checks.

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

const HALF_PERIOD_MICROS: u32 = 1_000;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 reset passthrough test (mock BMC)";

    fn main() -> ! {
        // SAFETY: created once, exclusive SoC access; the pins! table is this chip's true pin map.
        let pins = unsafe { create_pins() };
        // SAFETY: kernel-only binary, minted once; no process holds a conflicting grant.
        let gpio = GpioBlock::new(unsafe { take_aperture() });
        let heartbeat = pins.scu414_29.into_gpio(&gpio);
        scu::route(&(&heartbeat,));

        pw_log::info!("=== AST10x0 reset passthrough test (mock BMC) ===");

        let mut heartbeat = heartbeat.into_output();
        let _ = heartbeat.set_low();
        if heartbeat.read(heartbeat.map().out_level) {
            pw_log::error!("heartbeat line did not latch low");
            let _ = console_backend_write_all(b"TEST_RESULT:FAIL\n");

            #[expect(clippy::empty_loop)]
            loop {}
        }

        // Reported before the heartbeat starts, because the heartbeat never ends.
        pw_log::info!("heartbeat starting; waiting to be reset");
        let _ = console_backend_write_all(b"TEST_RESULT:PASS\n");

        loop {
            let _ = heartbeat.set_high();
            delay_us(HALF_PERIOD_MICROS);
            let _ = heartbeat.set_low();
            delay_us(HALF_PERIOD_MICROS);
        }
    }
}

declare_target!(Target);
