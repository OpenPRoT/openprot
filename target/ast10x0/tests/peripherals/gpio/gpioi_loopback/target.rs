// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOI loopback test target.

#![no_std]
#![no_main]

use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::gpio::{GpioRole, IntoGpio};
use ast10x0_peripherals::scu;
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

const GPIO_LEVEL_DELAY_CYCLES: u32 = 200_000_000;

fn test_gpio_loopback() -> bool {
    // Jumper: GPIOH4/SCL1 <-> GPIOH5/SDA1 (J16 pins 1-4, rot-ast-ctrl); other teams swap to GPIOI0/I1 (scu418_0/scu418_1).
    // SAFETY: created once at boot, exclusive SoC access; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let output = pins.scu414_28.into_gpio();
    let input = pins.scu414_29.into_gpio();
    scu::route(&(&output, &input));
    pw_log::info!("--- GPIOI loopback test ---");

    output.apply(GpioRole::Output);
    input.apply(GpioRole::Input);

    let interrupt_cases = [
        ("level-high", GpioRole::EnableLevelHigh, false, true),
        ("level-low", GpioRole::EnableLevelLow, true, false),
        ("rising-edge", GpioRole::EnableRising, false, true),
        ("falling-edge", GpioRole::EnableFalling, true, false),
        ("both-edge rising", GpioRole::EnableBoth, false, true),
        ("both-edge falling", GpioRole::EnableBoth, true, false),
    ];

    for (name, mode, initial_high, trigger_high) in interrupt_cases {
        pw_log::info!("=== Testing GPIOI1 {} interrupt ===", name as &str);

        input.apply(GpioRole::DisableInt);
        input.ack(input.map().int_status);

        if initial_high {
            output.apply(GpioRole::SetHigh);
        } else {
            output.apply(GpioRole::SetLow);
        }
        let initial_level = if initial_high { "high" } else { "low" };
        pw_log::info!(
            "{}: GPIOI0 as output drive initial level {}",
            name as &str,
            initial_level as &str
        );
        cortex_m::asm::delay(GPIO_LEVEL_DELAY_CYCLES);

        let input_matches = if initial_high {
            input.read(input.map().in_level)
        } else {
            !input.read(input.map().in_level)
        };
        if !input_matches {
            pw_log::error!("{}: input and output GPIO level mismatch", name as &str);
            return false;
        }
        pw_log::info!(
            "{}: GPIOI1 as input read initial level {}",
            name as &str,
            initial_level as &str
        );

        input.ack(input.map().int_status);
        input.apply(mode);
        if input.read(input.map().int_status) {
            pw_log::error!("{}: interrupt pending before trigger", name as &str);
            input.apply(GpioRole::DisableInt);
            return false;
        }

        if trigger_high {
            output.apply(GpioRole::SetHigh);
        } else {
            output.apply(GpioRole::SetLow);
        }
        let trigger_level = if trigger_high { "high" } else { "low" };
        pw_log::info!(
            "{}: GPIOI0 as output drive trigger level {}",
            name as &str,
            trigger_level as &str
        );
        cortex_m::asm::delay(GPIO_LEVEL_DELAY_CYCLES);

        let input_matches = if trigger_high {
            input.read(input.map().in_level)
        } else {
            !input.read(input.map().in_level)
        };
        if !input_matches {
            pw_log::error!("{}: input and output GPIO level mismatch", name as &str);
            input.apply(GpioRole::DisableInt);
            return false;
        }
        pw_log::info!(
            "{}: GPIOI1 as input read trigger level {}",
            name as &str,
            trigger_level as &str
        );

        if !input.read(input.map().int_status) {
            pw_log::error!("{}: interrupt status was not set", name as &str);
            input.apply(GpioRole::DisableInt);
            return false;
        }
        pw_log::info!("GPIOI1 {} interrupt status set", name as &str);

        // Disable level-sensitive modes before clearing so an active level
        // cannot immediately reassert the status bit.
        input.apply(GpioRole::DisableInt);
        input.ack(input.map().int_status);
        if input.read(input.map().int_status) {
            pw_log::error!("{}: interrupt status did not clear", name as &str);
            return false;
        }
    }

    true
}

fn run_gpioi_loopback_test() -> bool {
    pw_log::info!("=== AST10x0 GPIOI loopback test ===");
    // connect a jumper between GPIOH4/SCL1 and GPIOH5/SDA1 (or GPIOI0/I1 for other teams) to verify the loopback.
    test_gpio_loopback()
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 GPIOI loopback test";

    fn main() -> ! {
        let sentinel = if run_gpioi_loopback_test() {
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
