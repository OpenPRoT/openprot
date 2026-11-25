// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIOH loopback test (userspace).

#![no_main]
#![no_std]

use app_gpioh_loopback_app_pin_claims::take_pin_claims;
use app_gpioh_loopback_app_regions::take_mmaps;
use ast10x0_peripherals::gpio::{
    bind_gpio, GpioBlock, GpioPin, Input, InputPin, IntTrigger, Output, OutputPin,
};
use ast10x0_peripherals::scu::pins::{scu414_28, scu414_29};
use pw_status::Error;
use userspace::entry;
use userspace::syscall;
use userspace::time::{sleep_until, Clock, Duration, SystemClock};
use util_region::Mmap;

// GPIO electrical settling across the loopback jumper is on the order of microseconds; this is a
// generous margin, not a tuned minimum — prefer a slow, reliable test over a fast, flaky one.
const GPIO_LEVEL_DELAY: Duration = Duration::from_millis(100);

macro_rules! fail {
    ($($arg:tt)*) => {{
        pw_log::error!($($arg)*);
        let _ = syscall::debug_shutdown(Err(Error::Unknown));
        #[expect(clippy::empty_loop)]
        loop {}
    }};
}

fn delay() {
    let _ = sleep_until(SystemClock::now() + GPIO_LEVEL_DELAY);
}

fn test_gpio_loopback<R: Mmap>(
    output: &mut GpioPin<scu414_28, R, Output>,
    input: &mut GpioPin<scu414_29, R, Input>,
) -> bool {
    let interrupt_cases = [
        ("level-high", IntTrigger::LevelHigh, false, true),
        ("level-low", IntTrigger::LevelLow, true, false),
        ("rising-edge", IntTrigger::Rising, false, true),
        ("falling-edge", IntTrigger::Falling, true, false),
        ("both-edge rising", IntTrigger::Both, false, true),
        ("both-edge falling", IntTrigger::Both, true, false),
    ];

    for (name, mode, initial_high, trigger_high) in interrupt_cases {
        pw_log::info!("=== Testing GPIOH5 {} interrupt ===", name as &str);

        input.disable_interrupt();
        input.clear_int();

        if initial_high {
            let _ = output.set_high();
        } else {
            let _ = output.set_low();
        }
        let initial_level = if initial_high { "high" } else { "low" };
        pw_log::info!(
            "{}: GPIOH4 as output drive initial level {}",
            name as &str,
            initial_level as &str
        );
        delay();

        let level = input.is_high().unwrap_or(false);
        let input_matches = level == initial_high;
        if !input_matches {
            pw_log::error!("{}: input and output GPIO level mismatch", name as &str);
            return false;
        }
        pw_log::info!(
            "{}: GPIOH5 as input read initial level {}",
            name as &str,
            initial_level as &str
        );

        input.clear_int();
        input.enable_interrupt(mode);
        if input.int_pending() {
            pw_log::error!("{}: interrupt pending before trigger", name as &str);
            input.disable_interrupt();
            return false;
        }

        if trigger_high {
            let _ = output.set_high();
        } else {
            let _ = output.set_low();
        }
        let trigger_level = if trigger_high { "high" } else { "low" };
        pw_log::info!(
            "{}: GPIOH4 as output drive trigger level {}",
            name as &str,
            trigger_level as &str
        );
        delay();

        let level = input.is_high().unwrap_or(false);
        let input_matches = level == trigger_high;
        if !input_matches {
            pw_log::error!("{}: input and output GPIO level mismatch", name as &str);
            input.disable_interrupt();
            return false;
        }
        pw_log::info!(
            "{}: GPIOH5 as input read trigger level {}",
            name as &str,
            trigger_level as &str
        );

        if !input.int_pending() {
            pw_log::error!("{}: interrupt status was not set", name as &str);
            input.disable_interrupt();
            return false;
        }
        pw_log::info!("GPIOH5 {} interrupt status set", name as &str);

        // Disable level-sensitive modes before clearing so an active level
        // cannot immediately reassert the status bit.
        input.disable_interrupt();
        input.clear_int();
        if input.int_pending() {
            pw_log::error!("{}: interrupt status did not clear", name as &str);
            return false;
        }
    }

    true
}

#[entry]
fn entry() {
    // The kernel side routed these pins at the SCU before starting us; userspace has no SCU
    // grant, so we bind the already-routed pins rather than re-muxing them.
    // SAFETY: mints this process's claimed pins once, at its entry point.
    let pins = unsafe { take_pin_claims() };
    // SAFETY: mints this process's mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    let gpio = GpioBlock::new(mmaps.gpio_regs);
    let mut output = bind_gpio(pins.scu414_28, &gpio).into_output();
    let mut input = bind_gpio(pins.scu414_29, &gpio).into_input();
    pw_log::info!("--- GPIOH loopback test ---");

    if !test_gpio_loopback(&mut output, &mut input) {
        fail!("GPIOH loopback test failed");
    }

    pw_log::info!("PASS: GPIOH loopback test verified");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
