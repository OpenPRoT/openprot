// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Client for the i3c target service.
//!
//! `I3cClient<T>` turns the byte-oriented `i3c_api` frames into typed calls —
//! [`send`](I3cClient::send), [`recv`](I3cClient::recv) and
//! [`dynamic_address`](I3cClient::dynamic_address). All wire marshalling lives
//! here and is generic over [`Transport`]: the *same* encode/decode code runs
//! in production (`IpcTransport`, cross-process) and in host tests
//! (`LoopbackTransport`, in-process against a mock target).
//!
//! This crate has **no kernel/IPC dependency** and builds on the host — that
//! is what makes the encoders/decoders testable without a kernel.

#![no_std]
#![deny(missing_docs)]

use i3c_api::{
    decode_response, encode_request, I3cOp, I3cStatus, Transport, TransportError, MAX_FRAME,
    MAX_PAYLOAD,
};

/// Why a client call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    /// The transport round-trip itself failed.
    Transport(TransportError),
    /// The server returned a status this call does not treat as success.
    ///
    /// `NoData` and `Unassigned` are *not* reported here — they are expected
    /// outcomes and surface as `Ok(None)` from the calls that can see them.
    Server(I3cStatus),
    /// The response was empty, truncated, or carried an unknown status byte.
    InvalidResponse,
    /// The payload exceeds [`MAX_PAYLOAD`], or the caller's buffer is too
    /// small for the response. One call is one frame; nothing is fragmented.
    BufferTooSmall,
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "i3c transport error: {e}"),
            Self::Server(s) => write!(f, "i3c server error: {s:?}"),
            Self::InvalidResponse => f.write_str("malformed i3c response"),
            Self::BufferTooSmall => f.write_str("i3c frame exceeds one round-trip buffer"),
        }
    }
}

impl core::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for ClientError {
    fn from(e: TransportError) -> Self {
        Self::Transport(e)
    }
}

/// Typed client for the i3c target service, generic over the transport.
pub struct I3cClient<T> {
    transport: T,
}

impl<T: Transport> I3cClient<T> {
    /// Bind the client to one transport (one i3c target).
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Borrow the underlying transport.
    pub fn transport(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Perform one request/response round-trip, returning the status and the
    /// decoded response body borrowed from `resp`.
    ///
    /// The transport's mutable borrow of `resp` ends when it returns, so the
    /// body can be handed back as an immutable slice with no copy.
    fn transact<'r>(
        &mut self,
        op: I3cOp,
        payload: &[u8],
        resp: &'r mut [u8; MAX_FRAME],
    ) -> Result<(I3cStatus, &'r [u8]), ClientError> {
        let mut req = [0u8; MAX_FRAME];
        // `encode_request` returns `Some` only when the frame fits `req`, so
        // `req_len <= MAX_FRAME` and the slice below is always in bounds.
        let req_len = encode_request(op, payload, &mut req).ok_or(ClientError::BufferTooSmall)?;
        let resp_len = self.transport.transact(&req[..req_len], resp)?;
        decode_response(&resp[..resp_len]).ok_or(ClientError::InvalidResponse)
    }

    /// Stage `payload` for transmission to the controller.
    ///
    /// The target cannot initiate a transfer, so this stages a TX and raises an
    /// IBI; the controller collects it with a private read. A payload over
    /// [`MAX_PAYLOAD`] cannot be framed and returns [`ClientError::BufferTooSmall`].
    pub fn send(&mut self, payload: &[u8]) -> Result<(), ClientError> {
        let mut resp = [0u8; MAX_FRAME];
        match self.transact(I3cOp::Send, payload, &mut resp)? {
            (I3cStatus::Ok, _) => Ok(()),
            (status, _) => Err(ClientError::Server(status)),
        }
    }

    /// Take the latched inbound frame, copying it into `buf`.
    ///
    /// Returns `Ok(None)` when no frame is latched — the ordinary outcome of
    /// polling an idle bus, not an error. A successful read consumes the latch.
    ///
    /// `buf` must be at least [`MAX_PAYLOAD`] bytes. The server clears the
    /// single-frame latch the instant it answers `Recv`, so a buffer too small
    /// to hold the frame would lose it with no way to re-read; this is rejected
    /// *before* the round-trip rather than after, leaving the latch intact.
    pub fn recv(&mut self, buf: &mut [u8]) -> Result<Option<usize>, ClientError> {
        if buf.len() < MAX_PAYLOAD {
            return Err(ClientError::BufferTooSmall);
        }
        let mut resp = [0u8; MAX_FRAME];
        match self.transact(I3cOp::Recv, &[], &mut resp)? {
            (I3cStatus::NoData, _) => Ok(None),
            (I3cStatus::Ok, body) => {
                let n = body.len();
                buf.get_mut(..n)
                    .ok_or(ClientError::BufferTooSmall)?
                    .copy_from_slice(body);
                Ok(Some(n))
            }
            (status, _) => Err(ClientError::Server(status)),
        }
    }

    /// Read back the 7-bit dynamic address the controller assigned.
    ///
    /// Returns `Ok(None)` before ENTDAA has run. The address is assigned at
    /// runtime and changes across a bus reset, so callers that embed it in a
    /// wire header must re-read it rather than cache it indefinitely.
    pub fn dynamic_address(&mut self) -> Result<Option<u8>, ClientError> {
        let mut resp = [0u8; MAX_FRAME];
        match self.transact(I3cOp::DynamicAddress, &[], &mut resp)? {
            (I3cStatus::Unassigned, _) => Ok(None),
            (I3cStatus::Ok, [addr]) => Ok(Some(*addr)),
            (I3cStatus::Ok, _) => Err(ClientError::InvalidResponse),
            (status, _) => Err(ClientError::Server(status)),
        }
    }
}
