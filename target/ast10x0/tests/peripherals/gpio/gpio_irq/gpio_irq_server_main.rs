// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIOA0 interrupt configuration test (userspace).

#![no_main]
#![no_std]

use app_gpio_irq_server::{handle, signals};
use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::gpio::{bind_gpio, GpioRole};
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
    // SAFETY: sole pin creation site in this binary, at boot; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let a0 = bind_gpio(pins.scu410_0);
    a0.apply(GpioRole::Input);
    a0.apply(GpioRole::SetLow);

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

    a0.ack(a0.map().int_status);
    a0.apply(GpioRole::EnableBoth);

    if syscall::interrupt_ack(handle::GPIO_IRQ, signals::GPIO).is_err() {
        fail!("initial interrupt_ack failed");
    }

    let int_en = a0.read(a0.map().int_enable);
    let both_edge = a0.read(a0.map().sense_both);
    let pending = a0.read(a0.map().int_status);

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
