// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOA smoke test target.

#![no_std]
#![no_main]

use ast10x0_peripherals::gpio::{gpioa, ActiveLow, GpioExt};
use ast10x0_peripherals::scu::{pinctrl, ScuRegisters};
use console_backend::console_backend_write_all;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

fn run_gpioa_test() -> bool {
    // SAFETY: this kernel-only test is the sole owner of the SCU and GPIO peripherals.
    let gpioa = unsafe {
        let scu = ScuRegisters::new_global_unlocked();
        scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOA0);
        scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOA1);
        scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOA3);
        scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOA4);
        gpioa::GPIOA::new_global().split()
    };
    pw_log::info!("=== AST10x0 GPIOA smoke test ===");

    // AST1060 does not provide internal pull-up or pull-down control for these
    // GPIO inputs. Their sampled levels are determined by external connections,
    // so either 0 or 1 is valid in this configuration test.
    let mut pa0 = gpioa.pa0.into_pull_down_input();
    let pa0_level = pa0.is_high().unwrap_or(false);
    pw_log::info!("GPIOA0 input configured; sampled level={}", pa0_level as u32);

    let mut pa1 = gpioa.pa1.into_pull_up_input();
    let pa1_level = pa1.is_high().unwrap_or(false);
    pw_log::info!("GPIOA1 input configured; sampled level={}", pa1_level as u32);

    let mut pa3 = gpioa.pa3.into_open_drain_output::<ActiveLow>();
    if pa3.set_low().is_err() || !pa3.is_set_low().unwrap_or(false) {
        pw_log::error!("GPIOA3 open-drain output did not latch low");
        return false;
    }
    pw_log::info!("GPIOA3 open-drain output latched low");

    if pa3.set_high().is_err() || !pa3.is_set_high().unwrap_or(false) {
        pw_log::error!("GPIOA3 open-drain output did not latch high");
        return false;
    }
    pw_log::info!("GPIOA3 open-drain output latched high");

    let mut pa4 = gpioa.pa4.into_push_pull_output();
    if pa4.set_low().is_err() || !pa4.is_set_low().unwrap_or(false) {
        pw_log::error!("GPIOA4 push-pull output did not latch low");
        return false;
    }
    pw_log::info!("GPIOA4 push-pull output latched low");

    if pa4.set_high().is_err() || !pa4.is_set_high().unwrap_or(false) {
        pw_log::error!("GPIOA4 push-pull output did not latch high");
        return false;
    }
    pw_log::info!("GPIOA4 push-pull output latched high");
    true
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 GPIOA smoke test";

    fn main() -> ! {
        let sentinel = if run_gpioa_test() {
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
