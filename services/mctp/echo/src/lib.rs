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

use openprot_mctp_api::{MctpClient, MctpError, MctpRespChannel, Stack, StackListener};

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
///
/// Returns an error from either receive or response send.
pub fn echo_once<L>(listener: &mut L, buf: &mut [u8]) -> Result<(), MctpError>
where
    L: openprot_mctp_api::MctpListener,
{
    let (_meta, msg, mut resp) = listener.recv(buf)?;
    resp.send(msg)
}
