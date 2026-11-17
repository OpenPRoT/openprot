// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIOA0 interrupt configuration test (userspace).

#![no_main]
#![no_std]

use app_gpio_irq_server::{handle, signals};
use app_gpio_irq_server_pin_claims::take_pin_claims;
use app_gpio_irq_server_regions::take_mmaps;
use ast10x0_peripherals::gpio::{bind_gpio, GpioBlock, IntTrigger};
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
    // The kernel side routed this pin at the SCU before starting us; userspace has no SCU grant, so
    // we bind the already-routed pin rather than re-muxing it.
    // SAFETY: mints this process's claimed pins once, at its entry point.
    let pins = unsafe { take_pin_claims() };
    // SAFETY: mints this process's mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    let gpio = GpioBlock::new(mmaps.gpio_regs);
    let a0 = bind_gpio(pins.scu410_0, &gpio).into_input();

    if syscall::wait_group_add(
        handle::WG,
        handle::GPIO_IRQ,
        signals::GPIO,
        handle::GPIO_IRQ as usize,
    )
    .is_err()
    {
        fail!("wait_group_add failed");
    }

    a0.clear_int();
    a0.enable_interrupt(IntTrigger::Both);

    if syscall::interrupt_ack(handle::GPIO_IRQ, signals::GPIO).is_err() {
        fail!("initial interrupt_ack failed");
    }

    let int_en = a0.int_enabled();
    let both_edge = a0.sense_both();
    let pending = a0.int_pending();

    pw_log::info!(
        "GPIO IRQ state: int_en={} sensitivity2={} status={}",
        int_en as u32,
        both_edge as u32,
        pending as u32,
    );

    if !int_en {
        fail!("GPIOA0 interrupt not enabled");
    }
    if !both_edge {
        fail!("GPIOA0 both-edge sensitivity not configured");
    }

    pw_log::info!("PASS: GPIO IRQ configuration verified");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
