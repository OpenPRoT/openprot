// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP control responder app for the hardware image.
//!
//! Registers a listener for message type `0x00` and answers each control
//! request a bus owner sends that expects a reply, by handing the payload to
//! [`ControlResponder::handle`] and sending back the bytes it writes.
//!
//! The endpoint's current EID lives in the MCTP stack, which `handle` reads
//! and writes.

#![no_main]
#![no_std]

use app_mctp_control::handle;
use mctp_lib::config;
use openprot_mctp_api::{MctpListener, MctpRespChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_control::{
    ControlConfig, ControlError, ControlResponder, MAX_RESPONSE_LEN, MCTP_MSG_TYPE_CONTROL,
};
use userspace::entry;
use userspace::time::{sleep_until, Clock, Duration, Instant, SystemClock};

/// Message types this image carries, control itself excluded.
///
/// This image carries no upper-layer service, so Get Message Type Support
/// reports an empty list. An image assembling PLDM or SPDM beside the
/// responder names those types here.
static SUPPORTED_MSG_TYPES: [u8; 0] = [];

/// A listener timeout of 0 blocks until a message arrives.
const NO_TIMEOUT: u32 = 0;

/// Static EID this endpoint reports, the `OWN_EID` of `mctp_server_main.rs`.
///
/// The MCTP server already runs its stack at this EID, so a dynamic endpoint
/// would report an EID that no bus owner assigned.
const CONFIGURED_EID: u8 = 8;

#[entry]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let config = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);
    let responder = match ControlResponder::try_new_with_static_eid(&stack, config, CONFIGURED_EID)
    {
        Ok(responder) => responder,
        Err(e) => {
            pw_log::error!("control static EID refused: code={}", e.code as u32);
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

    pw_log::info!(
        "MCTP control responder ready at EID {}",
        CONFIGURED_EID as u32
    );

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
