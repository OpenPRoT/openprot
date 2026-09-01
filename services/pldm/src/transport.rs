// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP transport adapter for PLDM messages.
//!
//! [`MctpPldmTransport`] wraps a [`Stack`] (backed by any [`MctpClient`]) and
//! provides PLDM-specific send/receive helpers that manage the MCTP framing
//! byte automatically.
//!
//! ## Buffer layout
//!
//! All methods in this module use the same flat-buffer convention as the rest
//! of this crate:
//!
//! ```text
//! buf[0]          : MCTP message-type (0x01) – written/verified by this layer
//! buf[1..]        : PLDM message (header + data)
//! ```
//!
//! Callers only deal with PLDM bytes; the framing byte is inserted or stripped
//! transparently.

use openprot_mctp_api::stack::StackListener;
use openprot_mctp_api::{MctpClient, MctpListener, MctpReqChannel, MctpRespChannel, Stack};
use pldm_common::util::mctp_transport::MCTP_PLDM_MSG_TYPE;

use crate::error::{PldmMemError, PldmServiceError};

/// MCTP transport adapter for PLDM messages.
///
/// Wraps a [`Stack`] backed by any [`MctpClient`] implementation and provides
/// PLDM-specific send/receive helpers that manage the MCTP framing byte
/// (`buf[0]` = `0x01`) automatically.
///
/// # Example
///
/// ```rust,ignore
/// use openprot_pldm_service::transport::MctpPldmTransport;
/// use openprot_mctp_client::IpcMctpClient;
///
/// let transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));
///
/// // Use the underlying stack directly if needed.
/// transport.stack().set_eid(8).unwrap();
///
/// // Send a PLDM request; the caller fills buf[1..1+pldm_len] first.
/// let pldm_resp_len = transport
///     .send_request(remote_eid, pldm_len, &mut buf, 5_000)
///     .unwrap();
///
/// // Receive and respond to one inbound PLDM request via a handler closure.
/// transport
///     .recv_and_respond(&mut buf, 5_000, |framed_buf, _req_total_len, _source_eid| {
///         // framed_buf[0] == 0x01, framed_buf[1..] is the PLDM payload.
///         // Process and write the response in-place; return total bytes.
///         my_cmd_interface.handle_responder_msg(framed_buf)
///             .map_err(PldmServiceError::MsgHandler)
///     })
///     .unwrap();
/// ```
pub struct MctpPldmTransport<C: MctpClient> {
    stack: Stack<C>,
}

impl<C: MctpClient> MctpPldmTransport<C> {
    /// Create a new PLDM transport wrapping the given [`MctpClient`].
    pub fn new(client: C) -> Self {
        MctpPldmTransport {
            stack: Stack::new(client),
        }
    }

    /// Get a reference to the underlying MCTP [`Stack`].
    ///
    /// Useful when the raw stack is needed (e.g. to set the local EID).
    pub fn stack(&self) -> &Stack<C> {
        &self.stack
    }

    /// Send a PLDM request to `remote_eid` and receive the response.
    ///
    /// The caller must place the PLDM request bytes in `buf[1..1+pldm_len]`
    /// before calling this method.  `buf[0]` is overwritten with the MCTP
    /// PLDM message-type byte (`0x01`).
    ///
    /// On success, the PLDM response bytes are written into `buf[1..]` and
    /// the number of response bytes is returned.
    ///
    /// A `timeout_millis` of `0` blocks indefinitely.
    ///
    /// # Errors
    ///
    /// Returns [`PldmServiceError::PldmMem`] if `buf` is too small to hold
    /// the request or if an arithmetic overflow would occur.
    /// Returns [`PldmServiceError::Mctp`] on any MCTP transport error.
    pub fn send_request(
        &self,
        remote_eid: u8,
        pldm_len: usize,
        buf: &mut [u8],
        timeout_millis: u32,
    ) -> Result<usize, PldmServiceError> {
        // Stamp the framing byte even though the MCTP layer manages it; this
        // keeps buf consistent for callers that inspect buf[0] afterward.
        match buf.first_mut() {
            Some(b) => *b = MCTP_PLDM_MSG_TYPE,
            None => return Err(PldmServiceError::PldmMem(PldmMemError::MalformedBuffer)),
        }

        // Open an outbound request channel.
        let mut req_channel = self
            .stack
            .req(remote_eid, timeout_millis)
            .map_err(PldmServiceError::Mctp)?;

        // Send the PLDM payload (buf[1..1+pldm_len]).  The MCTP layer adds
        // its own framing, so we exclude buf[0].
        let req_end = pldm_len
            .checked_add(1)
            .ok_or(PldmServiceError::PldmMem(PldmMemError::OverflowMaxSize))?;
        let req_payload = buf
            .get(1..req_end)
            .ok_or(PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
        req_channel
            .send(MCTP_PLDM_MSG_TYPE, req_payload)
            .map_err(PldmServiceError::Mctp)?;

        // Receive the PLDM response into buf[1..].
        let recv_buf = buf
            .get_mut(1..)
            .ok_or(PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
        let (meta, _) = req_channel.recv(recv_buf).map_err(PldmServiceError::Mctp)?;

        Ok(meta.payload_size)
    }

    /// Receive one incoming PLDM request, frame it in `buf`, and invoke
    /// `handler` to produce a response.
    ///
    /// The PLDM payload is received into `buf[1..]`; `buf[0]` is set to the
    /// MCTP PLDM type byte (`0x01`).  `handler` receives the entire `buf`
    /// (with the request framed at `buf[..1+payload_size]`) so it has room to
    /// write a response that is larger than the request, processing it in
    /// place. `handler` is called with the full framed buffer, the total
    /// request length (including `buf[0]`), and the source EID of the
    /// request (`RecvMetadata::remote_eid`), so callers can filter requests
    /// by sender. It must return the total number of bytes written for the
    /// response (including the type byte at `buf[0]`), or `0` to indicate no
    /// response should be sent (e.g. the request came from an unexpected
    /// EID). Bytes `buf[1..resp_total_len]` are sent back to the requester.
    ///
    /// A `timeout_millis` of `0` blocks indefinitely.
    ///
    /// This registers and drops a fresh listener on every call. A caller that
    /// invokes this repeatedly in a tight loop with no other MCTP traffic in
    /// between is unaffected, but a caller that interleaves other MCTP
    /// activity on the same endpoint between calls (e.g.
    /// [`FirmwareDevice::run_terminus`](crate::firmware_device::FirmwareDevice::run_terminus),
    /// which interleaves FD-initiated requests) should use
    /// [`responder_listener`](Self::responder_listener) and
    /// [`respond_once`](Self::respond_once) instead, to avoid a window with no
    /// listener registered in which an inbound request could be dropped by
    /// the underlying MCTP stack.
    ///
    /// # Errors
    ///
    /// Returns [`PldmServiceError::PldmMem(PldmMemError::BufferTooSmall)`] if `buf` is too small.
    /// Returns [`PldmServiceError::Mctp`] on any MCTP transport error.
    /// Propagates any error returned by `handler`.
    pub fn recv_and_respond<F>(
        &self,
        buf: &mut [u8],
        timeout_millis: u32,
        handler: F,
    ) -> Result<(), PldmServiceError>
    where
        F: FnOnce(&mut [u8], usize, u8) -> Result<usize, PldmServiceError>,
    {
        let mut listener = self.responder_listener(timeout_millis)?;
        self.respond_once(&mut listener, buf, handler)
    }

    /// Register a persistent listener for inbound PLDM requests.
    ///
    /// Unlike [`recv_and_respond`](Self::recv_and_respond), which registers
    /// and drops a listener handle on every call, the returned listener can
    /// be reused across many [`respond_once`](Self::respond_once) calls
    /// (adjusting its timeout with [`StackListener::set_timeout`] as needed).
    /// This matters because the underlying MCTP stack requires an active
    /// listener registration to accept an inbound request of a given
    /// message type; a caller that interleaves other MCTP traffic (e.g.
    /// FD-initiated requests) between polls would otherwise have a window
    /// with no listener registered in which an inbound UA command could be
    /// silently dropped.
    ///
    /// A `timeout_millis` of `0` blocks indefinitely.
    pub fn responder_listener(
        &self,
        timeout_millis: u32,
    ) -> Result<StackListener<'_, C>, PldmServiceError> {
        self.stack
            .listener(MCTP_PLDM_MSG_TYPE, timeout_millis)
            .map_err(PldmServiceError::Mctp)
    }

    /// Receive one incoming PLDM request on an already-registered `listener`
    /// (obtained via [`responder_listener`](Self::responder_listener)), frame
    /// it in `buf`, and invoke `handler` to produce a response.
    ///
    /// See [`recv_and_respond`](Self::recv_and_respond) for the buffer and
    /// handler contract; this method has the same semantics but reuses an
    /// existing listener instead of registering a new one.
    ///
    /// # Errors
    ///
    /// Returns [`PldmServiceError::PldmMem(PldmMemError::BufferTooSmall)`] if `buf` is too small.
    /// Returns [`PldmServiceError::Mctp`] on any MCTP transport error.
    /// Propagates any error returned by `handler`.
    pub fn respond_once<F>(
        &self,
        listener: &mut StackListener<'_, C>,
        buf: &mut [u8],
        handler: F,
    ) -> Result<(), PldmServiceError>
    where
        F: FnOnce(&mut [u8], usize, u8) -> Result<usize, PldmServiceError>,
    {
        // Receive into buf[1..]; discard the payload sub-slice to end the
        // mutable borrow before we touch buf[0].
        let recv_buf = buf
            .get_mut(1..)
            .ok_or(PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
        let (meta, mut resp_channel) = listener
            .recv(recv_buf)
            .map(|(m, _, r)| (m, r))
            .map_err(PldmServiceError::Mctp)?;

        let payload_size = meta.payload_size;

        // Stamp the framing byte.
        match buf.first_mut() {
            Some(b) => *b = MCTP_PLDM_MSG_TYPE,
            None => return Err(PldmServiceError::PldmMem(PldmMemError::MalformedBuffer)),
        }

        // Ensure the buffer is at least large enough to hold the framed
        // request before handing it off.
        let req_total_len = payload_size
            .checked_add(1)
            .filter(|&total_len| total_len <= buf.len())
            .ok_or(PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;

        // Invoke the handler to process the request in-place.  The handler is
        // given the whole buffer so the response may exceed the request size.
        let resp_total_len = handler(buf, req_total_len, meta.remote_eid)?;

        // A handler returning 0 means it deliberately chose not to respond
        // (e.g. the request came from an EID it does not serve).
        if resp_total_len == 0 {
            return Ok(());
        }

        // Send the response, excluding the MCTP type byte that the transport
        // layer manages separately.
        let resp_payload = buf
            .get(1..resp_total_len)
            .ok_or(PldmServiceError::PldmMem(PldmMemError::BufferTooSmall))?;
        resp_channel
            .send(resp_payload)
            .map_err(PldmServiceError::Mctp)
    }
}
