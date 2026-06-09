// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP-based SPDM Transport
//!
//! This module provides an implementation of the `SpdmTransport` trait from
//! spdm-lib that uses MCTP as the underlying transport layer.
//!
//! ## MCTP Binding
//!
//! SPDM messages are carried over MCTP using message type 0x05 (SPDM).
//! The transport handles:
//! - MCTP session management (via Stack channels)
//! - Message fragmentation (via MCTP layer)
//! - Request/response correlation (via MCTP tags)

#![no_std]
#![warn(missing_docs)]

use openprot_mctp_api::stack::{Stack, StackListener, StackReqChannel, StackRespChannel};
use openprot_mctp_api::{MctpClient, MctpListener, MctpReqChannel, MctpRespChannel};
use spdm_lib::codec::MessageBuf;
use spdm_lib::platform::transport::{SpdmTransport, TransportError, TransportResult};

/// MCTP message type for SPDM (DMTF DSP0236 §4.2.1)
const MCTP_MSG_TYPE_SPDM: u8 = 0x05;

/// Maximum SPDM message size over MCTP
const MAX_SPDM_MESSAGE_SIZE: usize = 2048;

/// MCTP transport layer header size (none for SPDM over MCTP)
const MCTP_HEADER_SIZE: usize = 0;

/// SPDM transport implementation using MCTP as the underlying transport.
///
/// This transport can operate in two modes:
/// - **Requester mode**: Sends requests to a remote EID and receives responses
/// - **Responder mode**: Listens for incoming requests and sends responses
pub struct MctpSpdmTransport<'a, C: MctpClient> {
    /// MCTP stack for transport operations
    stack: &'a Stack<C>,

    /// Request channel for requester mode (outbound requests)
    req_channel: Option<StackReqChannel<'a, C>>,

    /// Listener for responder mode (incoming requests)
    listener: Option<StackListener<'a, C>>,

    /// Pending response channel (from last received request)
    pending_resp: Option<StackRespChannel<'a, C>>,

    /// Remote endpoint ID (for requester mode)
    remote_eid: Option<u8>,
}

impl<'a, C: MctpClient> MctpSpdmTransport<'a, C> {
    /// Create a new MCTP SPDM transport in requester mode.
    ///
    /// This will establish an MCTP request channel to the given remote EID.
    pub fn new_requester(stack: &'a Stack<C>, remote_eid: u8) -> Self {
        Self {
            stack,
            req_channel: None,
            listener: None,
            pending_resp: None,
            remote_eid: Some(remote_eid),
        }
    }

    /// Create a new MCTP SPDM transport in responder mode.
    ///
    /// This will register an MCTP listener for SPDM message type.
    pub fn new_responder(stack: &'a Stack<C>) -> Self {
        Self {
            stack,
            req_channel: None,
            listener: None,
            pending_resp: None,
            remote_eid: None,
        }
    }
}

impl<C: MctpClient> SpdmTransport for MctpSpdmTransport<'_, C> {
    /// Initialize the MCTP transport session.
    ///
    /// For **requester mode**:
    /// - Establishes an MCTP request channel targeting the remote EID
    ///
    /// For **responder mode**:
    /// - Registers an MCTP listener for SPDM message type (0x05)
    ///
    /// # Errors
    ///
    /// Returns `TransportError::DriverError` if channel allocation fails.
    fn init_sequence(&mut self) -> TransportResult<()> {
        if let Some(remote_eid) = self.remote_eid {
            // Requester mode: get request channel for remote EID
            pw_log::debug!("MctpSpdmTransport: req(eid={})", remote_eid as u32);
            self.req_channel = Some(self.stack.req(remote_eid, 0).map_err(|e| {
                pw_log::error!(
                    "MctpSpdmTransport: req(eid={}) failed: ResponseCode={}",
                    remote_eid as u32,
                    e.code as u32,
                );
                TransportError::DriverError
            })?);
            pw_log::debug!("MctpSpdmTransport: req channel allocated");
        } else {
            // Responder mode: register listener for SPDM messages
            pw_log::debug!(
                "MctpSpdmTransport: listener(msg_type=0x{:02x})",
                MCTP_MSG_TYPE_SPDM as u32
            );
            self.listener = Some(self.stack.listener(MCTP_MSG_TYPE_SPDM, 0).map_err(|e| {
                pw_log::error!(
                    "MctpSpdmTransport: listener(msg_type=0x05) failed: ResponseCode={}",
                    e.code as u32,
                );
                TransportError::DriverError
            })?);
            pw_log::debug!("MctpSpdmTransport: listener allocated");
        }

        Ok(())
    }

    fn send_request<'a>(&mut self, dest_eid: u8, req: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the request channel (should be set by init_sequence)
        let channel = self
            .req_channel
            .as_mut()
            .ok_or(TransportError::NoRequestInFlight)?;

        // message_data() returns buffer[head..tail] — the full serialized SPDM message
        // including header bytes that have been consumed by pull_data during encoding.
        // data_len() only returns the uncommitted tail bytes and must NOT be used here.
        let msg_data = req.message_data().map_err(|_| TransportError::SendError)?;
        pw_log::debug!(
            "send_request: eid={} len={} [0]={:#04x} [1]={:#04x}",
            dest_eid as u32,
            msg_data.len() as u32,
            msg_data.first().copied().unwrap_or(0) as u32,
            msg_data.get(1).copied().unwrap_or(0) as u32,
        );

        // Send via MCTP request channel
        channel
            .send(MCTP_MSG_TYPE_SPDM, msg_data)
            .map_err(|_| TransportError::SendError)?;

        Ok(())
    }

    fn receive_response<'a>(&mut self, rsp: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the request channel
        let channel = self
            .req_channel
            .as_mut()
            .ok_or(TransportError::ResponseNotExpected)?;

        // Allocate receive buffer
        let mut recv_buf = [0u8; MAX_SPDM_MESSAGE_SIZE];

        // Receive via MCTP (blocking with no timeout)
        let (meta, payload) = channel
            .recv(&mut recv_buf)
            .map_err(|_| TransportError::ReceiveError)?;

        // Verify message type
        if meta.msg_type != MCTP_MSG_TYPE_SPDM {
            return Err(TransportError::UnexpectedMessageType);
        }

        pw_log::debug!(
            "receive_response: len={} [0]={:#04x} [1]={:#04x}",
            payload.len() as u32,
            payload.first().copied().unwrap_or(0) as u32,
            payload.get(1).copied().unwrap_or(0) as u32,
        );

        // Copy payload into MessageBuf
        rsp.reserve(MCTP_HEADER_SIZE)
            .map_err(|_| TransportError::BufferTooSmall)?;
        rsp.put_data(payload.len())
            .map_err(|_| TransportError::BufferTooSmall)?;

        let rsp_buf = rsp
            .data_mut(payload.len())
            .map_err(|_| TransportError::BufferTooSmall)?;
        rsp_buf.copy_from_slice(payload);

        Ok(())
    }

    fn receive_request<'a>(&mut self, req: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the listener
        let listener = self.listener.as_mut().ok_or(TransportError::DriverError)?;

        // Allocate receive buffer
        let mut recv_buf = [0u8; MAX_SPDM_MESSAGE_SIZE];

        // Receive via MCTP (blocking with no timeout)
        let (meta, payload, resp_channel) = listener
            .recv(&mut recv_buf)
            .map_err(|_| TransportError::ReceiveError)?;

        // Verify message type
        if meta.msg_type != MCTP_MSG_TYPE_SPDM {
            return Err(TransportError::UnexpectedMessageType);
        }

        // Store response channel for send_response
        self.pending_resp = Some(resp_channel);

        pw_log::debug!(
            "receive_request: len={} [0]={:#04x} [1]={:#04x}",
            payload.len() as u32,
            payload.first().copied().unwrap_or(0) as u32,
            payload.get(1).copied().unwrap_or(0) as u32,
        );

        // Copy payload into MessageBuf
        req.reserve(MCTP_HEADER_SIZE)
            .map_err(|_| TransportError::BufferTooSmall)?;
        req.put_data(payload.len())
            .map_err(|_| TransportError::BufferTooSmall)?;

        let req_buf = req
            .data_mut(payload.len())
            .map_err(|_| TransportError::BufferTooSmall)?;
        req_buf.copy_from_slice(payload);

        Ok(())
    }

    fn send_response<'a>(&mut self, resp: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the pending response channel from last received request
        let mut resp_channel = self
            .pending_resp
            .take()
            .ok_or(TransportError::NoRequestInFlight)?;

        // message_data() returns buffer[head..tail] — the full serialized SPDM message.
        let msg_data = resp.message_data().map_err(|_| TransportError::SendError)?;
        pw_log::debug!(
            "send_response: eid={} len={} [0]={:#04x} [1]={:#04x}",
            resp_channel.remote_eid() as u32,
            msg_data.len() as u32,
            msg_data.first().copied().unwrap_or(0) as u32,
            msg_data.get(1).copied().unwrap_or(0) as u32,
        );

        // Send response back to requester
        resp_channel
            .send(msg_data)
            .map_err(|_| TransportError::SendError)?;

        Ok(())
    }

    fn max_message_size(&self) -> TransportResult<usize> {
        Ok(MAX_SPDM_MESSAGE_SIZE)
    }

    fn header_size(&self) -> usize {
        MCTP_HEADER_SIZE
    }
}
