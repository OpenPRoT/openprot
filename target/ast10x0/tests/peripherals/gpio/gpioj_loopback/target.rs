// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOI loopback test target.

#![no_std]
#![no_main]

use ast10x0_peripherals::gpio::{gpioj, GpioExt, InterruptMode};
use ast10x0_peripherals::scu::{pinctrl, ScuRegisters};
use console_backend::console_backend_write_all;
use embedded_hal::digital::{InputPin, OutputPin};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

const GPIO_LEVEL_DELAY_CYCLES: u32 = 200_000_000;

fn test_gpio_loopback() -> bool {
    let scu = unsafe { ScuRegisters::new_global() };
    scu.unlock_and_lock_after(|scu| {
        scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOJ4);
        scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOJ5);
    });

    // The loopback requires a jumper between the fixture board's exposed
    // GPIOJ4 pin and GPIOJ5 pin.
    let gpioj = unsafe { gpioj::GPIOJ::new_global().split() };
    pw_log::info!("--- GPIOJ loopback test ---");

    let mut output = gpioj.pj4.into_push_pull_output();
    let mut input = gpioj.pj5.into_pull_down_input();

    let interrupt_cases = [
        ("level-high", InterruptMode::LevelHigh, false, true),
        ("level-low", InterruptMode::LevelLow, true, false),
        ("rising-edge", InterruptMode::EdgeRising, false, true),
        ("falling-edge", InterruptMode::EdgeFalling, true, false),
        ("both-edge rising", InterruptMode::EdgeBoth, false, true),
        ("both-edge falling", InterruptMode::EdgeBoth, true, false),
    ];

    for (name, mode, initial_high, trigger_high) in interrupt_cases {
        pw_log::info!("=== Testing GPIOJ5 {} interrupt ===", name as &str);

        input.set_interrupt_mode(InterruptMode::Disabled);
        input.clear_interrupt();

        let initial_result = if initial_high {
            output.set_high()
        } else {
            output.set_low()
        };
        if initial_result.is_err() {
            pw_log::error!("{}: GPIOJ4 could not drive initial level", name as &str);
            return false;
        }
        let initial_level = if initial_high { "high" } else { "low" };
        pw_log::info!(
            "{}: GPIOJ4 as output drive initial level {}",
            name as &str,
            initial_level as &str
        );
        cortex_m::asm::delay(GPIO_LEVEL_DELAY_CYCLES);

        let input_matches = if initial_high {
            input.is_high().unwrap_or(false)
        } else {
            input.is_low().unwrap_or(false)
        };
        if !input_matches {
            pw_log::error!("{}: input and output GPIO level mismatch", name as &str);
            return false;
        }
        pw_log::info!(
            "{}: GPIOJ5 as input read initial level {}",
            name as &str,
            initial_level as &str
        );

        input.clear_interrupt();
        input.set_interrupt_mode(mode);
        if input.get_interrupt_status() {
            pw_log::error!("{}: interrupt pending before trigger", name as &str);
            input.set_interrupt_mode(InterruptMode::Disabled);
            return false;
        }

        let trigger_result = if trigger_high {
            output.set_high()
        } else {
            output.set_low()
        };
        if trigger_result.is_err() {
            pw_log::error!("{}: GPIOJ4 could not drive trigger level", name as &str);
            input.set_interrupt_mode(InterruptMode::Disabled);
            return false;
        }
        let trigger_level = if trigger_high { "high" } else { "low" };
        pw_log::info!(
            "{}: GPIOJ4 as output drive trigger level {}",
            name as &str,
            trigger_level as &str
        );
        cortex_m::asm::delay(GPIO_LEVEL_DELAY_CYCLES);

        let input_matches = if trigger_high {
            input.is_high().unwrap_or(false)
        } else {
            input.is_low().unwrap_or(false)
        };
        if !input_matches {
            pw_log::error!("{}: input and output GPIO level mismatch", name as &str);
            input.set_interrupt_mode(InterruptMode::Disabled);
            return false;
        }
        pw_log::info!(
            "{}: GPIOJ5 as input read trigger level {}",
            name as &str,
            trigger_level as &str
        );

        if !input.get_interrupt_status() {
            pw_log::error!("{}: interrupt status was not set", name as &str);
            input.set_interrupt_mode(InterruptMode::Disabled);
            return false;
        }
        pw_log::info!("GPIOJ5 {} interrupt status set", name as &str);

        // Disable level-sensitive modes before clearing so an active level
        // cannot immediately reassert the status bit.
        input.set_interrupt_mode(InterruptMode::Disabled);
        input.clear_interrupt();
        if input.get_interrupt_status() {
            pw_log::error!("{}: interrupt status did not clear", name as &str);
            return false;
        }
    }

    true
}

fn run_gpioj_loopback_test() -> bool {
    pw_log::info!("=== AST10x0 GPIOJ loopback test ===");
    // connect a jumper between the exposed GPIOJ4 and GPIOJ5 pins to verify the loopback.
    test_gpio_loopback()
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 GPIOJ loopback test";

    fn main() -> ! {
        let sentinel = if run_gpioj_loopback_test() {
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
