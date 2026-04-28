// Licensed under the Apache-2.0 license

#![no_main]
#![no_std]

use app_usart_server::{handle, signals};
use usart_backend::Backend;
use usart_server::runtime;
use userspace::entry;
use userspace::syscall::{self, Signals};

#[entry]
fn entry() -> ! {
    let mut backend = Backend::new();

    let _ = syscall::wait_group_add(
        handle::WG,
        handle::USART,
        Signals::READABLE,
        handle::USART as usize,
    );
    let _ = syscall::wait_group_add(
        handle::WG,
        handle::UART5_IRQ,
        signals::UART,
        handle::UART5_IRQ as usize,
    );

    runtime::run(&mut backend, handle::WG, handle::UART5_IRQ, signals::UART);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
