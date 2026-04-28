// Licensed under the Apache-2.0 license

#![no_main]
#![no_std]

use app_usart_client::handle;
use usart_client::UsartClient;
use userspace::entry;
use userspace::syscall;

#[entry]
fn entry() -> ! {
    let client = UsartClient::new(handle::USART);

    match client.configure(1_500_000) {
        Ok(()) => pw_log::info!("USART client configured"),
        Err(_) => pw_log::error!("USART client configure failed"),
    }

    match client.write(b"usart client online\r\n") {
        Ok(count) => pw_log::info!("USART client wrote {} bytes", count as u32),
        Err(_) => pw_log::error!("USART client write failed"),
    }

    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
