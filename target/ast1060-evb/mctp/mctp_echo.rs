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

use openprot_mctp_api::stack::Stack;
use openprot_mctp_api::{MctpListener, MctpRespChannel};
use openprot_mctp_client::IpcMctpClient;

use pw_status::Result;
use userspace::entry;
use userspace::syscall;

use app_mctp_echo::handle;

/// MCTP message type for echo (vendor-defined type 1).
const ECHO_MSG_TYPE: u8 = 1;

fn mctp_echo_loop() -> Result<()> {
    pw_log::info!("MCTP echo starting");

    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));

    let mut listener = stack
        .listener(ECHO_MSG_TYPE, 0)
        .map_err(|_| pw_status::Error::Internal)?;

    let mut buf = [0u8; 1024];

    loop {
        // Block until a message arrives; recv returns the payload slice and
        // a response channel already bound to the sender's EID and tag.
        let (meta, msg, mut resp) = listener
            .recv(&mut buf)
            .map_err(|_| pw_status::Error::Internal)?;

        pw_log::info!(
            "Echo: received {} bytes from EID {}",
            meta.payload_size as u32,
            meta.remote_eid as u32,
        );

        // Echo the payload back through the response channel.
        if let Err(_) = resp.send(msg) {
            pw_log::error!("Echo: failed to send response to EID {}", meta.remote_eid as u32);
        }
    }
}

#[entry]
fn entry() -> ! {
    if let Err(e) = mctp_echo_loop() {
        pw_log::error!("MCTP echo error: {}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
