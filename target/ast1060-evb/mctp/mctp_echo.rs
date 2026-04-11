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

use openprot_mctp_api::MctpClient;
use openprot_mctp_client::IpcMctpClient;

use pw_status::Result;
use userspace::entry;
use userspace::syscall;

use app_mctp_echo::handle;

/// MCTP message type for echo (vendor-defined PCIe type).
/// 0x7e = MCTP_TYPE_VENDOR_PCIE
const ECHO_MSG_TYPE: u8 = 0x7e;

/// AMD PCIe Vendor ID
const AMD_PCIE_VENDOR_ID: u16 = 0x1022;

fn mctp_echo_loop() -> Result<()> {
    pw_log::info!("MCTP echo starting");

    let client = IpcMctpClient::new(handle::MCTP);

    // Register a listener for vendor-defined PCIe messages (0x7e)
    let listener = client
        .listener(ECHO_MSG_TYPE)
        .map_err(|e| {
            pw_log::error!(
                "Failed to register listener for msg_type 0x{:02x}: error code {}",
                ECHO_MSG_TYPE as u32,
                e.code as u32,
            );
            pw_status::Error::Internal
        })?;

    let mut buf = [0u8; 1024];

    loop {
        // Block until a message arrives
        let meta = client
            .recv(listener, 0, &mut buf)
            .map_err(|_| pw_status::Error::Internal)?;

        // Verify vendor ID (first 2 bytes should be AMD PCIe vendor ID in little-endian)
        if meta.payload_size >= 2 {
            let vendor_id = u16::from_le_bytes([buf[0], buf[1]]);
            pw_log::info!(
                "Echo: received {} bytes from EID {}, vendor ID: 0x{:04x}",
                meta.payload_size as u32,
                meta.remote_eid as u32,
                vendor_id as u32,
            );

            if vendor_id != AMD_PCIE_VENDOR_ID {
                pw_log::warn!(
                    "Unexpected vendor ID 0x{:04x}, expected AMD 0x{:04x}",
                    vendor_id as u32,
                    AMD_PCIE_VENDOR_ID as u32,
                );
            }
        } else {
            pw_log::warn!("Payload too short: {} bytes", meta.payload_size as u32);
        }

        // Echo the payload back (including vendor ID)
        let payload = &buf[..meta.payload_size];
        client
            .send(
                None,                    // no request handle (this is a response)
                meta.msg_type,           // same message type (0x7e)
                Some(meta.remote_eid),   // back to sender
                Some(meta.msg_tag),      // same tag
                meta.msg_ic,             // preserve integrity check
                payload,
            )
            .map_err(|_| pw_status::Error::Internal)?;
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
