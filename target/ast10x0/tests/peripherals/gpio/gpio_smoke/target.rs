// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOA smoke test target.

#![no_std]
#![no_main]

use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::gpio::{GpioRole, IntoGpio};
use ast10x0_peripherals::scu;
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

fn run_gpioa_test() -> bool {
    // SAFETY: created once, exclusive SoC access; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let a0 = pins.scu410_0.into_gpio();
    let a1 = pins.scu410_1.into_gpio();
    let a3 = pins.scu410_3.into_gpio();
    let a4 = pins.scu410_4.into_gpio();
    // Each handle carries its route at the type level; the tuple's `COALESCED` folds them to the
    // minimal set of RMWs at compile time — apply once.
    scu::route(&(&a0, &a1, &a3, &a4));
    pw_log::info!("=== AST10x0 GPIOA smoke test ===");

    // AST1060 has no internal pull control on these inputs, so their sampled level is set by external
    // wiring — either value is valid. Configure as input and log the live level; do not assert.
    a0.apply(GpioRole::Input);
    pw_log::info!(
        "GPIOA0 input sampled level={}",
        a0.read(a0.map().in_level) as u32
    );

    a1.apply(GpioRole::Input);
    pw_log::info!(
        "GPIOA1 input sampled level={}",
        a1.read(a1.map().in_level) as u32
    );

    // Outputs are verified against the output latch (`OUT_LEVEL`), which is wiring-independent — an
    // open-drain high floats the pin, so reading the live level would depend on an external pull-up.
    a3.apply(GpioRole::Output);
    a3.apply(GpioRole::SetLow);
    if a3.read(a3.map().out_level) {
        pw_log::error!("GPIOA3 open-drain output did not latch low");
        return false;
    }
    pw_log::info!("GPIOA3 open-drain output latched low");

    a3.apply(GpioRole::SetHigh);
    if !a3.read(a3.map().out_level) {
        pw_log::error!("GPIOA3 open-drain output did not latch high");
        return false;
    }
    pw_log::info!("GPIOA3 open-drain output latched high");

    a4.apply(GpioRole::Output);
    a4.apply(GpioRole::SetLow);
    if a4.read(a4.map().out_level) {
        pw_log::error!("GPIOA4 push-pull output did not latch low");
        return false;
    }
    pw_log::info!("GPIOA4 push-pull output latched low");

    a4.apply(GpioRole::SetHigh);
    if !a4.read(a4.map().out_level) {
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
