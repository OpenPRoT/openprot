// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIOA0 interrupt configuration test (userspace).

#![no_main]
#![no_std]

use app_gpio_irq_server::{handle, signals};
use ast10x0_peripherals::gpio::{GpioExt, InterruptMode, gpioa};
use pw_status::Error;
use userspace::entry;
use userspace::syscall;

const GPIOA0_MASK: u32 = 1;

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
    // SAFETY: this process exclusively owns the GPIO device mapping declared
    // in system.json5.
    let gpioa = unsafe { gpioa::GPIOA::new_global().split() };
    let mut pa0 = gpioa.pa0.into_pull_down_input();

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

    pa0.clear_interrupt();
    pa0.set_interrupt_mode(InterruptMode::EdgeBoth);

    if syscall::interrupt_ack(handle::GPIO_IRQ, signals::GPIO).is_err() {
        fail!("initial interrupt_ack failed");
    }

    // SAFETY: the GPIO register block is mapped exclusively into this process.
    let registers = unsafe { &*ast1060_pac::Gpio::ptr() };
    let int_en = registers.gpio008().read().bits();
    let sensitivity2 = registers.gpio014().read().bits();
    let int_status = registers.gpio018().read().bits();

    pw_log::info!(
        "GPIO IRQ state: int_en=0x{:08x} sensitivity2=0x{:08x} status=0x{:08x}",
        int_en as u32,
        sensitivity2 as u32,
        int_status as u32,
    );

    if int_en & GPIOA0_MASK != GPIOA0_MASK {
        fail!(
            "GPIOA0 interrupt not enabled: int_en=0x{:08x}",
            int_en as u32
        );
    }
    if sensitivity2 & GPIOA0_MASK != GPIOA0_MASK {
        fail!(
            "GPIOA0 both-edge sensitivity not configured: sensitivity2=0x{:08x}",
            sensitivity2 as u32
        );
    }

    pw_log::info!("PASS: GPIO IRQ configuration verified");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
