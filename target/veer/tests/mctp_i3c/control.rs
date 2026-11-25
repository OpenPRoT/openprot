// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP control responder app: answers the control requests (message type 0) a bus
//! owner sends to this endpoint.
//!
//! It is the same app as `target/ast10x0/tests/mctp/control`, a client of the MCTP
//! server like the echo app, with its own channel. It registers a listener for
//! message type `0x00` and hands each request to [`ControlResponder::handle`],
//! which reads and writes the endpoint's EID in the MCTP stack.

#![no_main]
#![no_std]

use mctp_control_codegen::handle;
use mctp_i3c_ids::OWN_EID;
use mctp_lib::config;
use openprot_mctp_api::{MctpListener, MctpRespChannel, Stack};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_mctp_control::{
    ControlConfig, ControlResponder, MsgTypeVersions, MAX_RESPONSE_LEN, MCTP_CONTROL_MALFORMED,
    MCTP_CONTROL_RESPONSE_TOO_SHORT, MCTP_MSG_TYPE_CONTROL,
};
use userspace::process_entry;

/// Message types this image carries, control itself excluded.
///
/// None: the echo app uses message type 1, which is PLDM's number (DSP0239), but
/// it is a test service, not PLDM, so Get Message Type Support must not say so.
static SUPPORTED_MSG_TYPES: [MsgTypeVersions<'static>; 0] = [];

/// A listener timeout of 0 blocks until a message arrives.
const NO_TIMEOUT: u32 = 0;

/// The EID this endpoint reports: the one the MCTP server runs its stack at. A
/// dynamic endpoint would report one that no bus owner assigned.
const CONFIGURED_EID: u8 = OWN_EID;

#[process_entry("mctp_control")]
fn entry() {
    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let server_eid = stack.get_eid();
    if server_eid != CONFIGURED_EID {
        pw_log::warn!(
            "MCTP server runs at EID {}, not the configured {}",
            server_eid as u32,
            CONFIGURED_EID as u32
        );
    }
    let config = ControlConfig::simple_endpoint(&SUPPORTED_MSG_TYPES);
    let responder = match ControlResponder::try_new_with_static_eid(&stack, config, CONFIGURED_EID)
    {
        Ok(responder) => responder,
        Err(e) => {
            pw_log::error!(
                "control static EID refused: code=0x{:08x}",
                u32::from(e) as u32
            );
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
        "mctp control: responder ready at EID {}",
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
                // The listener blocks, so an error is a failure of the IPC call
                // itself and retrying at once would spin.
                pw_log::error!("control recv failed: code={}", e.code as u32);
                loop {}
            }
        };

        let len = match responder.handle(&stack, request, &mut response_buf) {
            Ok(Some(len)) => len,
            // DSP0236 clause 11.5: a response and a datagram draw no reply.
            Ok(None) => continue,
            // Too short to carry a control header, so there is no command code
            // or instance ID to echo.
            Err(MCTP_CONTROL_MALFORMED) => {
                pw_log::error!("control request too short for a header");
                continue;
            }
            // `response_buf` holds MAX_RESPONSE_LEN bytes, so this is a defect in
            // this image, never bad input from the requester.
            Err(MCTP_CONTROL_RESPONSE_TOO_SHORT) => {
                pw_log::error!("control response buffer too short for the reply");
                continue;
            }
            Err(e) => {
                pw_log::error!("control request failed: code=0x{:08x}", u32::from(e) as u32);
                continue;
            }
        };

        // The channel fills in the message type, the destination EID and the
        // request's tag, so this passes the reply bytes alone. `handle` wrote
        // `len` bytes into `response_buf`, so the slice exists; `get` keeps a
        // slice failure from being a panic path.
        let Some(response) = response_buf.get(..len) else {
            pw_log::error!("control response longer than its buffer");
            continue;
        };
        if let Err(e) = reply.send(response) {
            pw_log::error!("control reply failed: code={}", e.code as u32);
        }
    }
}
