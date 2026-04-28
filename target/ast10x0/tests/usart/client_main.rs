// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use app_usart_client::handle;
use usart_client::UsartClient;
use userspace::entry;
use userspace::time::Instant;

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

    let mut read_buf = [0u8; 16];
    // Use a bounded deadline so the test exercises timeout-capable try_read
    // semantics without risking an unbounded wait.
    match client.try_read_with_timeout(&mut read_buf, Instant::from_ticks(5_000_000)) {
        Ok(count) => pw_log::info!(
            "USART try_read returned immediately with {} bytes",
            count as u32
        ),
        Err(usart_client::ClientError::IpcError(e)) if (e as u32) == 4 => {
            pw_log::info!("USART try_read timeout observed as expected")
        }
        Err(usart_client::ClientError::IpcError(e)) => {
            pw_log::info!("USART try_read returned IPC status {}", e as u32)
        }
        Err(usart_client::ClientError::ServerError(e)) => {
            pw_log::info!("USART try_read returned server status {}", e as u8)
        }
        Err(usart_client::ClientError::InvalidResponse) => {
            pw_log::info!("USART try_read returned invalid response")
        }
        Err(usart_client::ClientError::BufferTooSmall) => {
            pw_log::info!("USART try_read returned buffer too small")
        }
    }

    // Keep client 1 alive; client 2 performs test shutdown once it runs.
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
