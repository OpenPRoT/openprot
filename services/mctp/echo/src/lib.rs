// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Reusable MCTP echo loop.
//!
//! This crate keeps the echo application policy separate from target wiring:
//! callers create an `openprot_mctp_api::Stack`, then hand it to the helpers
//! here to configure the local EID, open a listener, and echo received payloads
//! back to the sender.
//!
//! The loop can optionally send periodic test messages to a peer endpoint
//! to bootstrap communication when both endpoints are passive listeners.

#![no_std]

use openprot_mctp_api::{
    MctpClient, MctpError, MctpReqChannel, MctpRespChannel, Stack, StackListener,
};

/// Default MCTP message type used by the echo app.
pub const ECHO_MSG_TYPE: u8 = 1;

/// Default local EID used by the echo app.
pub const ECHO_EID: u8 = 8;

/// Prepare a stack for echoing by setting the local EID and opening a listener.
pub fn prepare_listener<C: MctpClient>(
    stack: &Stack<C>,
) -> Result<StackListener<'_, C>, MctpError> {
    stack.set_eid(ECHO_EID)?;
    stack.listener(ECHO_MSG_TYPE, 0)
}

/// Prepare a stack with a custom EID and timeout for echoing.
pub fn prepare_listener_with_eid_and_timeout<C: MctpClient>(
    stack: &Stack<C>,
    eid: u8,
    timeout_millis: u32,
) -> Result<StackListener<'_, C>, MctpError> {
    stack.set_eid(eid)?;
    stack.listener(ECHO_MSG_TYPE, timeout_millis)
}

/// Process one echo receive/send cycle.
pub fn echo_once<L>(listener: &mut L, buf: &mut [u8]) -> Result<(), openprot_mctp_api::MctpError>
where
    L: openprot_mctp_api::MctpListener,
{
    let (_meta, msg, mut resp) = listener.recv(buf)?;
    resp.send(msg)
}

/// Run the echo loop forever, echoing received messages.
pub fn run<L>(listener: &mut L) -> !
where
    L: openprot_mctp_api::MctpListener,
{
    let mut buf = [0u8; 255];
    loop {
        match listener.recv(&mut buf) {
            Ok((_meta, msg, mut resp)) => {
                let _ = resp.send(msg);
            }
            Err(e) => {
                // Timeouts are expected noise from the short recv deadline; suppress them.
                if !e.is_timeout() {
                    pw_log::error!("echo recv failed: code={}", e.code as u32);
                }
            }
        }
    }
}

/// Run the echo loop with periodic sends to a peer endpoint, forever.
pub fn run_with_peer<C: MctpClient, L: openprot_mctp_api::MctpListener>(
    stack: &Stack<C>,
    peer_eid: u8,
    listener: &mut L,
) -> ! {
    loop {
        run_with_peer_round_trip_limit(stack, peer_eid, listener, u32::MAX);
    }
}

/// Run the echo loop with periodic sends to a peer endpoint, returning after
/// `max_round_trips` successful request/response exchanges.
pub fn run_with_peer_round_trip_limit<C: MctpClient, L: openprot_mctp_api::MctpListener>(
    stack: &Stack<C>,
    peer_eid: u8,
    listener: &mut L,
    max_round_trips: u32,
) {
    let mut buf = [0u8; 255];
    let mut iteration: u32 = 0;
    // 10 iterations between sends keeps the channel active without flooding it.
    let send_interval = 10;

    // Keep retrying until the first successful round-trip so a slow-booting
    // peer (>1 min) does not cause us to give up before it is ready.
    loop {
        match listener.recv(&mut buf) {
            Ok((_meta, msg, mut resp)) => {
                let _ = resp.send(msg);
            }
            Err(e) => {
                // Timeouts are expected noise from the short recv deadline; suppress them.
                if !e.is_timeout() {
                    pw_log::error!("echo recv failed: code={}", e.code as u32);
                }
            }
        }

        if let Ok(mut req) = stack.req(peer_eid, 10000) {
            let test_msg = b"echo_test";
            let _ = req.send(ECHO_MSG_TYPE, test_msg);
            if req.recv(&mut buf).is_ok() {
                break;
            }
        }
    }

    // First round-trip succeeded; count it and continue with periodic sends.
    let mut completed_round_trips: u32 = 1;

    loop {
        iteration = iteration.wrapping_add(1);

        // Periodically try to send a test message to the peer.
        if iteration % send_interval == 0
            && completed_round_trips < max_round_trips
            && let Ok(mut req) = stack.req(peer_eid, 100)
        {
            let test_msg = b"echo_test";
            let _ = req.send(ECHO_MSG_TYPE, test_msg);
            // Try to receive response with a short timeout.
            if req.recv(&mut buf).is_ok() {
                completed_round_trips = completed_round_trips.saturating_add(1);
            }
        }

        // Listen for incoming messages and echo them back.
        match listener.recv(&mut buf) {
            Ok((_meta, msg, mut resp)) => {
                let _ = resp.send(msg);
            }
            Err(e) => {
                // Timeouts are expected noise from the short recv deadline; suppress them.
                if !e.is_timeout() {
                    pw_log::error!("echo recv failed: code={}", e.code as u32);
                }
            }
        }
    }
}
