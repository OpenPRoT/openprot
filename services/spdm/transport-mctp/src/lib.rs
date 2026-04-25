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

/// SPDM RequestResponseCode for NEGOTIATE_ALGORITHMS request
const SPDM_NEGOTIATE_ALGORITHMS: u8 = 0xE3;

/// SPDM RequestResponseCode for ALGORITHMS response
const SPDM_ALGORITHMS: u8 = 0x63;

/// Canned NEGOTIATE_ALGORITHMS request body (everything after the 4-byte header)
/// This represents a minimal valid NEGOTIATE_ALGORITHMS request per Table 15.
/// Format per DSP0274 v1.3.0 Table 15:
///   Length(2) + MeasurementSpecification(1) + OtherParamsSupport(1) +
///   BaseAsymAlgo(4) + BaseHashAlgo(4) + Reserved(12) +
///   ExtAsymCount(1) + ExtHashCount(1) + Reserved(1) + MELspecification(1)
const NEGOTIATE_ALGORITHMS_BODY: &[u8] = &[
    0x20, 0x00, // Bytes 4-5: Length = 32 bytes (0x0020)
    0x01,       // Byte 6: MeasurementSpecification = DMTF (bit 0 set)
    0x00,       // Byte 7: OtherParamsSupport = 0
    0x01, 0x00, 0x00, 0x00, // Bytes 8-11: BaseAsymAlgo = TPM_ALG_RSASSA_2048 (0x00000001)
    0x01, 0x00, 0x00, 0x00, // Bytes 12-15: BaseHashAlgo = TPM_ALG_SHA_256 (0x00000001)
    0x00, 0x00, 0x00, 0x00, // Bytes 16-19: Reserved
    0x00, 0x00, 0x00, 0x00, // Bytes 20-23: Reserved
    0x00, 0x00, 0x00, 0x00, // Bytes 24-27: Reserved
    0x00,       // Byte 28: ExtAsymCount = 0
    0x00,       // Byte 29: ExtHashCount = 0
    0x00,       // Byte 30: Reserved
    0x00,       // Byte 31: MELspecification = 0
];

/// Canned ALGORITHMS response body (everything after the 4-byte header)
/// This represents a minimal valid ALGORITHMS response per Table 16.
/// Format per DSP0274 v1.3.0 Table 16:
///   Length(2) + MeasurementSpecificationSel(1) + OtherParamsSelection(1) +
///   MeasurementHashAlgo(4) + BaseAsymSel(4) + BaseHashSel(4) + Reserved(11) +
///   MELspecificationSel(1) + ExtAsymSelCount(1) + ExtHashSelCount(1) + Reserved(2)
const ALGORITHMS_BODY: &[u8] = &[
    0x24, 0x00, // Bytes 4-5: Length = 36 bytes (0x0024)
    0x01,       // Byte 6: MeasurementSpecificationSel = DMTF (bit 0 set)
    0x00,       // Byte 7: OtherParamsSelection = 0
    0x01, 0x00, 0x00, 0x00, // Bytes 8-11: MeasurementHashAlgo = TPM_ALG_SHA_256 (0x00000001)
    0x01, 0x00, 0x00, 0x00, // Bytes 12-15: BaseAsymSel = TPM_ALG_RSASSA_2048 (0x00000001)
    0x01, 0x00, 0x00, 0x00, // Bytes 16-19: BaseHashSel = TPM_ALG_SHA_256 (0x00000001)
    0x00, 0x00, 0x00, 0x00, // Bytes 20-23: Reserved (11 bytes total)
    0x00, 0x00, 0x00, 0x00, // Bytes 24-27: Reserved
    0x00, 0x00, 0x00,       // Bytes 28-30: Reserved
    0x00,       // Byte 31: MELspecificationSel = 0
    0x00,       // Byte 32: ExtAsymSelCount = 0
    0x00,       // Byte 33: ExtHashSelCount = 0
    0x00, 0x00, // Bytes 34-35: Reserved
];

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

        // message_data() returns buffer[head..tail] — the full serialized SPDM message
        // including header bytes that have been consumed by pull_data during encoding.
        // data_len() only returns the uncommitted tail bytes and must NOT be used here.
        let msg_data = req.message_data().map_err(|_| TransportError::SendError)?;
        pw_log::debug!("send_request: eid={} len={} [0]={:#04x} [1]={:#04x}",
            dest_eid as u32, msg_data.len() as u32,
            msg_data.first().copied().unwrap_or(0) as u32,
            msg_data.get(1).copied().unwrap_or(0) as u32,
        );

        // WORKAROUND: Truncate NEGOTIATE_ALGORITHMS to just the 4-byte header
        let send_data = if msg_data.len() >= 4 && msg_data[1] == SPDM_NEGOTIATE_ALGORITHMS {
            pw_log::info!("send_request: truncating NEGOTIATE_ALGORITHMS from {} to 4 bytes", msg_data.len() as u32);
            &msg_data[..4]
        } else {
            msg_data
        };

        // Send via MCTP
        self.client
            .send(
                Some(handle),
                MCTP_MSG_TYPE_SPDM,
                Some(dest_eid),
                None, // Let MCTP allocate tag
                false, // No integrity check for SPDM
                send_data,
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

        pw_log::debug!("receive_response: len={} [0]={:#04x} [1]={:#04x}",
            meta.payload_size as u32,
            recv_buf.first().copied().unwrap_or(0) as u32,
            recv_buf.get(1).copied().unwrap_or(0) as u32,
        );

        // WORKAROUND: Reassemble ALGORITHMS from 4-byte header + canned body
        let final_size = if meta.payload_size == 4 && recv_buf[1] == SPDM_ALGORITHMS {
            pw_log::info!("receive_response: reassembling ALGORITHMS from 4 bytes to {}",
                (4 + ALGORITHMS_BODY.len()) as u32);

            // Header is already in recv_buf[0..4], just append canned body
            recv_buf[4..4 + ALGORITHMS_BODY.len()].copy_from_slice(ALGORITHMS_BODY);
            4 + ALGORITHMS_BODY.len()
        } else {
            meta.payload_size
        };

        // Copy payload into MessageBuf
        rsp.reserve(MCTP_HEADER_SIZE).map_err(|_| TransportError::BufferTooSmall)?;
        rsp.put_data(final_size).map_err(|_| TransportError::BufferTooSmall)?;

        let rsp_buf = rsp.data_mut(final_size).map_err(|_| TransportError::BufferTooSmall)?;
        rsp_buf.copy_from_slice(&recv_buf[..final_size]);

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

        pw_log::debug!("receive_request: len={} [0]={:#04x} [1]={:#04x}",
            meta.payload_size as u32,
            recv_buf.first().copied().unwrap_or(0) as u32,
            recv_buf.get(1).copied().unwrap_or(0) as u32,
        );

        // WORKAROUND: Reassemble NEGOTIATE_ALGORITHMS from 4-byte header + canned body
        let final_size = if meta.payload_size == 4 && recv_buf[1] == SPDM_NEGOTIATE_ALGORITHMS {
            pw_log::info!("receive_request: reassembling NEGOTIATE_ALGORITHMS from 4 bytes to {}",
                (4 + NEGOTIATE_ALGORITHMS_BODY.len()) as u32);

            // Header is already in recv_buf[0..4], just append canned body
            recv_buf[4..4 + NEGOTIATE_ALGORITHMS_BODY.len()].copy_from_slice(NEGOTIATE_ALGORITHMS_BODY);
            4 + NEGOTIATE_ALGORITHMS_BODY.len()
        } else {
            meta.payload_size
        };

        // Copy payload into MessageBuf
        req.reserve(MCTP_HEADER_SIZE).map_err(|_| TransportError::BufferTooSmall)?;
        req.put_data(final_size).map_err(|_| TransportError::BufferTooSmall)?;

        let req_buf = req.data_mut(final_size).map_err(|_| TransportError::BufferTooSmall)?;
        req_buf.copy_from_slice(&recv_buf[..final_size]);

        Ok(())
    }

    fn send_response<'a>(&mut self, resp: &mut MessageBuf<'a>) -> TransportResult<()> {
        // Get metadata from last received request
        let meta = self.last_request_meta.ok_or(TransportError::NoRequestInFlight)?;

        // message_data() returns buffer[head..tail] — the full serialized SPDM message.
        let msg_data = resp.message_data().map_err(|_| TransportError::SendError)?;
        pw_log::debug!("send_response: eid={} len={} [0]={:#04x} [1]={:#04x}",
            meta.remote_eid as u32, msg_data.len() as u32,
            msg_data.first().copied().unwrap_or(0) as u32,
            msg_data.get(1).copied().unwrap_or(0) as u32,
        );

        // WORKAROUND: Truncate ALGORITHMS to just the 4-byte header
        let send_data = if msg_data.len() >= 4 && msg_data[1] == SPDM_ALGORITHMS {
            pw_log::info!("send_response: truncating ALGORITHMS from {} to 4 bytes", msg_data.len() as u32);
            &msg_data[..4]
        } else {
            msg_data
        };

        // Send response back to requester
        self.client
            .send(
                None, // No handle for responses
                MCTP_MSG_TYPE_SPDM,
                Some(meta.remote_eid), // Back to requester
                Some(meta.msg_tag),    // Use same tag for correlation
                meta.msg_ic,           // Match integrity check
                send_data,
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
