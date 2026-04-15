// Licensed under the Apache-2.0 license

//! MCTP-based SPDM Transport
//!
//! This module provides an implementation of the `SpdmTransport` trait from
//! spdm-lib that uses MCTP as the underlying transport layer.
//!
//! ## MCTP Binding
//!
//! SPDM messages are carried over MCTP using message type 0x05 (SPDM).
//! The transport handles:
//! - MCTP session management (handles for req/listener)
//! - Message fragmentation (via MCTP layer)
//! - Request/response correlation (via MCTP tags)

#![no_std]
#![warn(missing_docs)]

use openprot_mctp_api::{Handle, MctpClient, RecvMetadata};
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
pub struct MctpSpdmTransport<C: MctpClient> {
    /// MCTP client for transport operations
    client: C,

    /// MCTP handle for requester mode (outbound requests)
    req_handle: Option<Handle>,

    /// MCTP handle for responder mode (incoming requests)
    listener_handle: Option<Handle>,

    /// Remote endpoint ID (for requester mode)
    remote_eid: Option<u8>,

    /// Last received message metadata (for response correlation)
    last_request_meta: Option<RecvMetadata>,
}

impl<C: MctpClient> MctpSpdmTransport<C> {
    /// Create a new MCTP SPDM transport in requester mode.
    ///
    /// This will establish an MCTP request handle to the given remote EID.
    pub fn new_requester(client: C, remote_eid: u8) -> Self {
        Self {
            client,
            req_handle: None,
            listener_handle: None,
            remote_eid: Some(remote_eid),
            last_request_meta: None,
        }
    }

    /// Create a new MCTP SPDM transport in responder mode.
    ///
    /// This will register an MCTP listener for SPDM message type.
    pub fn new_responder(client: C) -> Self {
        Self {
            client,
            req_handle: None,
            listener_handle: None,
            remote_eid: None,
            last_request_meta: None,
        }
    }
}

impl<C: MctpClient> SpdmTransport for MctpSpdmTransport<C> {
    /// Initialize the MCTP transport session.
    ///
    /// For **requester mode**:
    /// - Establishes an MCTP request handle targeting the remote EID
    ///
    /// For **responder mode**:
    /// - Registers an MCTP listener for SPDM message type (0x05)
    ///
    /// # Errors
    ///
    /// Returns `TransportError::DriverError` if MCTP handle allocation fails.
    fn init_sequence(&mut self) -> TransportResult<()> {
        if let Some(remote_eid) = self.remote_eid {
            // Requester mode: get request handle for remote EID
            pw_log::debug!("MctpSpdmTransport: req(eid={})", remote_eid as u32);
            self.req_handle = Some(
                self.client
                    .req(remote_eid)
                    .map_err(|e| {
                        pw_log::error!(
                            "MctpSpdmTransport: req(eid={}) failed: ResponseCode={}",
                            remote_eid as u32,
                            e.code as u8,
                        );
                        TransportError::DriverError
                    })?,
            );
            pw_log::debug!("MctpSpdmTransport: req handle allocated");
        } else {
            // Responder mode: register listener for SPDM messages
            pw_log::debug!("MctpSpdmTransport: listener(msg_type=0x{:02x})", MCTP_MSG_TYPE_SPDM as u32);
            self.listener_handle = Some(
                self.client
                    .listener(MCTP_MSG_TYPE_SPDM)
                    .map_err(|e| {
                        pw_log::error!(
                            "MctpSpdmTransport: listener(msg_type=0x05) failed: ResponseCode={}",
                            e.code as u8,
                        );
                        TransportError::DriverError
                    })?,
            );
            pw_log::debug!("MctpSpdmTransport: listener handle allocated");
        }

        Ok(())
    }

    fn send_request<'a>(&mut self, dest_eid: u8, req: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the request handle (should be set by init_sequence)
        let handle = self.req_handle.ok_or(TransportError::NoRequestInFlight)?;

        // Get the message data from MessageBuf
        let msg_len = req.data_len();
        let msg_data = req.data(msg_len).map_err(|_| TransportError::SendError)?;

        // Send via MCTP
        self.client
            .send(
                Some(handle),
                MCTP_MSG_TYPE_SPDM,
                Some(dest_eid),
                None, // Let MCTP allocate tag
                false, // No integrity check for SPDM
                msg_data,
            )
            .map_err(|_| TransportError::SendError)?;

        Ok(())
    }

    fn receive_response<'a>(&mut self, rsp: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the request handle
        let handle = self.req_handle.ok_or(TransportError::ResponseNotExpected)?;

        // Allocate receive buffer
        let mut recv_buf = [0u8; MAX_SPDM_MESSAGE_SIZE];

        // Receive via MCTP (blocking with no timeout)
        let meta = self.client
            .recv(handle, 0, &mut recv_buf)
            .map_err(|_| TransportError::ReceiveError)?;

        // Verify message type
        if meta.msg_type != MCTP_MSG_TYPE_SPDM {
            return Err(TransportError::UnexpectedMessageType);
        }

        // Copy payload into MessageBuf
        let payload = &recv_buf[..meta.payload_size];
        rsp.reserve(MCTP_HEADER_SIZE).map_err(|_| TransportError::BufferTooSmall)?;
        rsp.put_data(meta.payload_size).map_err(|_| TransportError::BufferTooSmall)?;

        let rsp_buf = rsp.data_mut(meta.payload_size).map_err(|_| TransportError::BufferTooSmall)?;
        rsp_buf.copy_from_slice(payload);

        Ok(())
    }

    fn receive_request<'a>(&mut self, req: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get the listener handle
        let handle = self.listener_handle.ok_or(TransportError::DriverError)?;

        // Allocate receive buffer
        let mut recv_buf = [0u8; MAX_SPDM_MESSAGE_SIZE];

        // Receive via MCTP (blocking with no timeout)
        let meta = self.client
            .recv(handle, 0, &mut recv_buf)
            .map_err(|_| TransportError::ReceiveError)?;

        // Verify message type
        if meta.msg_type != MCTP_MSG_TYPE_SPDM {
            return Err(TransportError::UnexpectedMessageType);
        }

        // Store metadata for response correlation
        self.last_request_meta = Some(meta);

        // Copy payload into MessageBuf
        let payload = &recv_buf[..meta.payload_size];
        req.reserve(MCTP_HEADER_SIZE).map_err(|_| TransportError::BufferTooSmall)?;
        req.put_data(meta.payload_size).map_err(|_| TransportError::BufferTooSmall)?;

        let req_buf = req.data_mut(meta.payload_size).map_err(|_| TransportError::BufferTooSmall)?;
        req_buf.copy_from_slice(payload);

        Ok(())
    }

    fn send_response<'a>(&mut self, resp: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get metadata from last received request
        let meta = self.last_request_meta.ok_or(TransportError::NoRequestInFlight)?;

        // Get the response data from MessageBuf
        let msg_len = resp.data_len();
        let msg_data = resp.data(msg_len).map_err(|_| TransportError::SendError)?;

        // Send response back to requester
        self.client
            .send(
                None, // No handle for responses
                MCTP_MSG_TYPE_SPDM,
                Some(meta.remote_eid), // Back to requester
                Some(meta.msg_tag),    // Use same tag for correlation
                meta.msg_ic,           // Match integrity check
                msg_data,
            )
            .map_err(|_| TransportError::SendError)?;

        // Clear request metadata
        self.last_request_meta = None;

        Ok(())
    }

    fn max_message_size(&self) -> TransportResult<usize> {
        Ok(MAX_SPDM_MESSAGE_SIZE)
    }

    fn header_size(&self) -> usize {
        MCTP_HEADER_SIZE
    }
}

impl<C: MctpClient> Drop for MctpSpdmTransport<C> {
    fn drop(&mut self) {
        // Clean up MCTP handles
        if let Some(handle) = self.req_handle {
            self.client.drop_handle(handle);
        }
        if let Some(handle) = self.listener_handle {
            self.client.drop_handle(handle);
        }
    }
}
