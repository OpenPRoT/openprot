// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use app_usart_client2::handle;
use usart_client::UsartClient;
use userspace::entry;
use userspace::syscall;

#[entry]
fn entry() -> ! {
    let client = UsartClient::new(handle::USART);

    match client.configure(1_500_000) {
        Ok(()) => pw_log::info!("USART client2 configured"),
        Err(_) => pw_log::error!("USART client2 configure failed"),
    }

    match client.write(b"usart client2 online\r\n") {
        Ok(count) => pw_log::info!("USART client2 wrote {} bytes", count as u32),
        Err(_) => pw_log::error!("USART client2 write failed"),
    }

    // End the test only after client2 has executed, proving multi-client app
    // startup and request handling against the same server endpoint.
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
