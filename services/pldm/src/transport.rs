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

use openprot_mctp_api::{MctpClient, MctpListener, MctpRespChannel, MctpReqChannel, Stack};
use pldm_common::util::mctp_transport::MCTP_PLDM_MSG_TYPE;

use crate::error::PldmServiceError;

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
///     .recv_and_respond(&mut buf, 5_000, |framed_buf| {
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
    /// Useful when the raw stack is needed (e.g. to set the local EID or to
    /// hand to [`PldmResponder`] / [`PldmRequester`]).
    ///
    /// [`PldmResponder`]: crate::responder::PldmResponder
    /// [`PldmRequester`]: crate::requester::PldmRequester
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
    /// Returns [`PldmServiceError::Overflow`] if `buf` is too small to hold
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
            None => return Err(PldmServiceError::Overflow),
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
            .ok_or(PldmServiceError::Overflow)?;
        let req_payload = buf.get(1..req_end).ok_or(PldmServiceError::Overflow)?;
        req_channel
            .send(MCTP_PLDM_MSG_TYPE, req_payload)
            .map_err(PldmServiceError::Mctp)?;

        // Receive the PLDM response into buf[1..].
        let recv_buf = buf.get_mut(1..).ok_or(PldmServiceError::Overflow)?;
        let (meta, _) = req_channel
            .recv(recv_buf)
            .map_err(PldmServiceError::Mctp)?;

        Ok(meta.payload_size)
    }

    /// Receive one incoming PLDM request, frame it in `buf`, and invoke
    /// `handler` to produce a response.
    ///
    /// The PLDM payload is received into `buf[1..]`; `buf[0]` is set to the
    /// MCTP PLDM type byte (`0x01`).  `handler` receives the entire `buf`
    /// (with the request framed at `buf[..1+payload_size]`) so it has room to
    /// write a response that is larger than the request, processing it in
    /// place. `handler` is called with the full framed buffer and the total
    /// request length (including `buf[0]`). It must return the total number of
    /// bytes written for the response (including the type byte at `buf[0]`).
    /// Bytes
    /// `buf[1..resp_total_len]` are sent back to the requester.
    ///
    /// A `timeout_millis` of `0` blocks indefinitely.
    ///
    /// # Errors
    ///
    /// Returns [`PldmServiceError::Overflow`] if `buf` is too small.
    /// Returns [`PldmServiceError::Mctp`] on any MCTP transport error.
    /// Propagates any error returned by `handler`.
    pub fn recv_and_respond<F>(
        &self,
        buf: &mut [u8],
        timeout_millis: u32,
        handler: F,
    ) -> Result<(), PldmServiceError>
    where
        F: FnOnce(&mut [u8], usize) -> Result<usize, PldmServiceError>,
    {
        let mut listener = self
            .stack
            .listener(MCTP_PLDM_MSG_TYPE, timeout_millis)
            .map_err(PldmServiceError::Mctp)?;

        // Receive into buf[1..]; discard the payload sub-slice to end the
        // mutable borrow before we touch buf[0].
        let recv_buf = buf.get_mut(1..).ok_or(PldmServiceError::Overflow)?;
        let (meta, mut resp_channel) = listener
            .recv(recv_buf)
            .map(|(m, _, r)| (m, r))
            .map_err(PldmServiceError::Mctp)?;

        let payload_size = meta.payload_size;

        // Stamp the framing byte.
        match buf.first_mut() {
            Some(b) => *b = MCTP_PLDM_MSG_TYPE,
            None => return Err(PldmServiceError::Overflow),
        }

        // Ensure the buffer is at least large enough to hold the framed
        // request before handing it off.
        let req_total_len = payload_size
            .checked_add(1)
            .filter(|&total_len| total_len <= buf.len())
            .ok_or(PldmServiceError::Overflow)?;

        // Invoke the handler to process the request in-place.  The handler is
        // given the whole buffer so the response may exceed the request size.
        let resp_total_len = handler(buf, req_total_len)?;

        // Send the response, excluding the MCTP type byte that the transport
        // layer manages separately.
        let resp_payload = buf.get(1..resp_total_len).ok_or(PldmServiceError::Overflow)?;
        resp_channel
            .send(resp_payload)
            .map_err(PldmServiceError::Mctp)
    }
}
