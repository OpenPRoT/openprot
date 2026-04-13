// Licensed under the Apache-2.0 license

//! MCTP Echo Application
//!
//! Listens for MCTP type-1 (vendor-defined) messages and echoes the
//! payload back to the sender. This is a direct port of the Hubris
//! `task/mctp-echo/` task.
//!
//! # Architecture
//!
//! The echo app is a simple loop:
//! 1. Register a listener for MCTP message type 1
//! 2. Receive a message (blocking)
//! 3. Send the payload back to the sender
//! 4. Repeat

#![no_main]
#![no_std]

use openprot_mctp_api::{MctpClient, MctpError};
use openprot_mctp_client::IpcMctpClient;

use userspace::entry;
use userspace::syscall;

use app_mctp_echo::handle;

/// MCTP message type for echo (vendor-defined type 1).
const ECHO_MSG_TYPE: u8 = 1;

fn die(e: MctpError) -> ! {
    let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
    let _ = e;
    loop {}
}

fn mctp_echo_loop() -> ! {
    pw_log::info!("MCTP echo starting");

    let client = IpcMctpClient::new(handle::MCTP);

    // Register a listener for type-1 messages
    let listener = client
        .listener(ECHO_MSG_TYPE)
        .unwrap_or_else(|e| {
            pw_log::error!("Failed to register listener: error code {}", e.code as u8);
            die(e)
        });

    let mut buf = [0u8; 1024];

    loop {
        // Block until a message arrives
        let meta = client
            .recv(listener, 0, &mut buf)
            .unwrap_or_else(|e| {
                pw_log::error!("recv failed: error code {}", e.code as u8);
                die(e)
            });

        pw_log::info!(
            "Echo: received {} bytes from EID {}",
            meta.payload_size as u32,
            meta.remote_eid as u32,
        );

        // Echo the payload back
        let payload = &buf[..meta.payload_size];
        client
            .send(
                None,                    // no request handle (this is a response)
                meta.msg_type,           // same message type
                Some(meta.remote_eid),   // back to sender
                Some(meta.msg_tag),      // same tag
                meta.msg_ic,             // preserve integrity check
                payload,
            )
            .unwrap_or_else(|e| {
                pw_log::error!("send failed: error code {}", e.code as u8);
                die(e)
            });
    }
}

#[entry]
fn entry() -> ! {
    mctp_echo_loop()
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
