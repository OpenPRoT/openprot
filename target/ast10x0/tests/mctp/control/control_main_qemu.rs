// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP control responder app for the QEMU image.
//!
//! Registers a listener for message type `0x00` and answers each control
//! request from `fake_server_main.rs` that expects a reply, by passing the
//! payload to [`ControlResponder::handle`] and sending back the bytes it
//! writes.
//!
//! The loop and its error handling match `control_main.rs`. This app builds a
//! dynamic responder, and `fake_server_main.rs` stands in for the MCTP server
//! app and its bus.

#![no_main]
#![no_std]

use app_mctp_control_qemu::handle;
use mctp_lib::config;
use openprot_mctp_api::{MctpListener, MctpRespChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_control::{
    ControlConfig, ControlError, ControlResponder, MAX_RESPONSE_LEN, MCTP_MSG_TYPE_CONTROL,
};
use userspace::entry;
use userspace::time::{sleep_until, Clock, Duration, Instant, SystemClock};

/// Message types this image reports, control itself excluded.
///
/// This image carries neither PLDM nor SPDM, but names both so that case 5 in
/// `fake_server_main.rs` can tell a reply carrying the configured list from
/// one dropping it.
static SUPPORTED_MSG_TYPES: [u8; 2] = [0x01, 0x05];

/// A listener timeout of 0 blocks until a message arrives.
const NO_TIMEOUT: u32 = 0;

#[entry]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let config = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);
    let responder = match ControlResponder::try_new(config) {
        Ok(responder) => responder,
        Err(e) => {
            pw_log::error!("control config refused: code={}", e.code as u32);
            loop {}
        }
    };

    let mut listener = match stack.listener(MCTP_MSG_TYPE_CONTROL, NO_TIMEOUT) {
        Ok(listener) => listener,
        Err(e) => {
            pw_log::error!("control listener failed: code={}", e.code as u32);
            loop {}
        }
    };

    pw_log::info!("MCTP control responder ready, dynamic EID");

    // The payload comes back as a reborrow of the receive buffer, so the reply
    // needs a buffer of its own.
    let mut request_buf = [0u8; config::MAX_PAYLOAD];
    let mut response_buf = [0u8; MAX_RESPONSE_LEN];

    loop {
        let (_meta, request, mut reply) = match listener.recv(&mut request_buf) {
            Ok(received) => received,
            Err(e) => {
                pw_log::error!("control recv failed: code={}", e.code as u32);
                // A persistent IPC failure would spin here and flood the UART.
                let deadline = SystemClock::now()
                    .checked_add_duration(Duration::from_millis(10))
                    .unwrap_or(Instant::MAX);
                let _ = sleep_until(deadline);
                continue;
            }
        };

        let len = match responder.handle(&stack, request, &mut response_buf) {
            Ok(len) => len,
            // DSP0236 clause 11.5: a response and a datagram each draw no
            // reply.
            Err(ControlError::NoReply) => continue,
            // Too short to carry a control header, so the reply would have no
            // command code or instance ID to echo.
            Err(ControlError::Malformed) => {
                pw_log::error!("control request too short for a header");
                continue;
            }
            // response_buf holds MAX_RESPONSE_LEN bytes, so this error is a
            // defect in this image, never bad input from the requester.
            Err(ControlError::ResponseTooShort) => {
                pw_log::error!("control response buffer too short for the reply");
                continue;
            }
            Err(_) => {
                pw_log::error!("control request failed with an unknown error");
                continue;
            }
        };

        // The channel fills in the message type, the destination EID, and the
        // request's own tag, so this call passes the reply bytes alone.
        if let Err(e) = reply.send(&response_buf[..len]) {
            pw_log::error!("control reply failed: code={}", e.code as u32);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
