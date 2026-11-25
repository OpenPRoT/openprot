// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOA smoke test (userspace).

#![no_main]
#![no_std]

use app_gpio_smoke_app_pin_claims::take_pin_claims;
use app_gpio_smoke_app_regions::take_mmaps;
use ast10x0_peripherals::gpio::{
    bind_gpio, ActiveLow, GpioBlock, InputPin, OutputPin, StatefulOutputPin,
};
use pw_status::Error;
use userspace::entry;
use userspace::syscall;

macro_rules! fail {
    ($($arg:tt)*) => {{
        pw_log::error!($($arg)*);
        let _ = syscall::debug_shutdown(Err(Error::Unknown));
        #[expect(clippy::empty_loop)]
        loop {}
    }};
}

#[entry]
fn entry() {
    // The kernel side routed these pins at the SCU before starting us; userspace has no SCU grant,
    // so we bind the already-routed pins rather than re-muxing them.
    // SAFETY: mints this process's claimed pins once, at its entry point.
    let pins = unsafe { take_pin_claims() };
    // SAFETY: mints this process's mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    let gpio = GpioBlock::new(mmaps.gpio_regs);
    let mut a0 = bind_gpio(pins.scu410_0, &gpio).into_input();
    let mut a1 = bind_gpio(pins.scu410_1, &gpio).into_input();
    let mut a3 = bind_gpio(pins.scu410_3, &gpio).into_open_drain_output::<ActiveLow>();
    let mut a4 = bind_gpio(pins.scu410_4, &gpio).into_output();
    pw_log::info!("=== AST10x0 GPIOA smoke test ===");

    // AST1060 has no internal pull control on these inputs, so their sampled level is set by external
    // wiring — either value is valid. Configure as input and log the live level; do not assert.
    pw_log::info!(
        "GPIOA0 input sampled level={}",
        a0.is_high().unwrap_or(false) as u32
    );
    pw_log::info!(
        "GPIOA1 input sampled level={}",
        a1.is_high().unwrap_or(false) as u32
    );

    // GPIOA3 is active-low open-drain: `set_low` actively drives it, `set_high` releases it
    // (direction = input) and relies on an external pull-up — verified via the direction
    // register (is_set_low/is_set_high), never the data latch, since open-drain never writes
    // the data register again once configured.
    let _ = a3.set_low();
    if !a3.is_set_low().unwrap_or(false) {
        fail!("GPIOA3 open-drain output did not latch low");
    }
    pw_log::info!("GPIOA3 open-drain output latched low");

    let _ = a3.set_high();
    if !a3.is_set_high().unwrap_or(false) {
        fail!("GPIOA3 open-drain output did not latch high");
    }
    pw_log::info!("GPIOA3 open-drain output latched high");

    // GPIOA4 is push-pull: unlike GPIOA3, it actively drives both levels, verified against the
    // output latch (`OUT_LEVEL`).
    let _ = a4.set_low();
    if a4.is_set_high().unwrap_or(true) {
        fail!("GPIOA4 push-pull output did not latch low");
    }
    pw_log::info!("GPIOA4 push-pull output latched low");

    let _ = a4.set_high();
    if !a4.is_set_high().unwrap_or(false) {
        fail!("GPIOA4 push-pull output did not latch high");
    }
    pw_log::info!("GPIOA4 push-pull output latched high");

    pw_log::info!("PASS: GPIOA smoke test verified");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
