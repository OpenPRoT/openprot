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
    decode_response, encode_request, DeviceInfo, I3cOp, I3cStatus, Transport, TransportError,
    MAX_FRAME, MAX_PAYLOAD, MAX_SEND,
};

/// Why a client call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    /// The transport round-trip itself failed.
    Transport(TransportError),
    /// The previous response is still staged: the controller has not read it, so
    /// nothing was staged for this `Send`. Try again later. Only `send` returns
    /// this.
    Busy,
    /// An earlier staged response was discarded unread, so a message in
    /// progress has lost a fragment. Nothing was staged for this `Send`; it is
    /// reported once, and the next `send` is accepted. Only `send` returns this.
    Discarded,
    /// The target no longer has the address the frame was built for, so nothing
    /// was staged: read the address again, rebuild the frame, send again. Only
    /// `send_for` returns this.
    AddressChanged,
    /// The server returned a status this call does not treat as success.
    ///
    /// `NoData`, `Unassigned` and `Busy` are *not* reported here — they are expected
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
            Self::Busy => f.write_str("i3c server busy: the previous response is unread"),
            Self::Discarded => f.write_str("i3c server discarded an unread response"),
            Self::AddressChanged => f.write_str("i3c address changed since the frame was built"),
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
        let req_len = encode_request(op, payload, &mut req).ok_or(ClientError::BufferTooSmall)?;
        let req = req.get(..req_len).ok_or(ClientError::BufferTooSmall)?;
        let resp_len = self.transport.transact(req, resp)?;
        match resp
            .get(..resp_len)
            .and_then(decode_response)
            .ok_or(ClientError::InvalidResponse)?
        {
            (I3cStatus::Busy, _) => Err(ClientError::Busy),
            decoded => Ok(decoded),
        }
    }

    /// Stage a frame that does not depend on the target's address.
    ///
    /// See [`send_for`](Self::send_for), which this is with no address.
    pub fn send(&mut self, frame: &[u8]) -> Result<(), ClientError> {
        self.send_with(0, frame)
    }

    /// Stage a frame built for the dynamic address `addr`.
    ///
    /// For a frame whose check covers the address byte, as an MCTP-over-I3C PEC
    /// does. The server stages it only if the target still has `addr`, and
    /// discards it if the address moves before the controller reads it. If the
    /// address has already moved this returns `Err(ClientError::AddressChanged)`
    /// with nothing staged: read the address again and rebuild the frame.
    ///
    /// The target cannot initiate a transfer, so this stages a TX and raises an
    /// IBI; the controller collects it with a private read. A frame over
    /// [`MAX_SEND`] cannot be sent and returns [`ClientError::BufferTooSmall`].
    ///
    /// Only one response is staged at a time. While the previous one has not
    /// been read by the controller, nothing is staged and this returns
    /// `Err(ClientError::Busy)`: retry later. If the server gave up on an unread
    /// response in the meantime, the next call returns
    /// `Err(ClientError::Discarded)` once, with nothing staged.
    pub fn send_for(&mut self, addr: u8, frame: &[u8]) -> Result<(), ClientError> {
        if addr == 0 {
            // 0 means "no address" on the wire, so it cannot name one.
            return Err(ClientError::Server(I3cStatus::InvalidOp));
        }
        self.send_with(addr, frame)
    }

    fn send_with(&mut self, addr: u8, frame: &[u8]) -> Result<(), ClientError> {
        if frame.len() > MAX_SEND {
            return Err(ClientError::BufferTooSmall);
        }
        let mut payload = [0u8; MAX_PAYLOAD];
        let (head, body) = payload
            .split_first_mut()
            .ok_or(ClientError::BufferTooSmall)?;
        *head = addr;
        body.get_mut(..frame.len())
            .ok_or(ClientError::BufferTooSmall)?
            .copy_from_slice(frame);
        let len = 1 + frame.len();

        let mut resp = [0u8; MAX_FRAME];
        let sent = payload.get(..len).ok_or(ClientError::BufferTooSmall)?;
        match self.transact(I3cOp::Send, sent, &mut resp)? {
            (I3cStatus::Ok, _) => Ok(()),
            (I3cStatus::Discarded, _) => Err(ClientError::Discarded),
            (I3cStatus::AddressChanged, _) => Err(ClientError::AddressChanged),
            (status, _) => Err(ClientError::Server(status)),
        }
    }

    /// Take the latched inbound frame, copying it into `buf`.
    ///
    /// Returns `Ok(None)` when no frame is latched — the ordinary outcome of
    /// polling an idle bus, not an error. A successful read consumes the frame.
    ///
    /// `buf` must be at least [`MAX_PAYLOAD`] bytes. The server pops the frame
    /// from its queue the instant it answers `Recv`, so a buffer too small to
    /// hold it would lose it with no way to re-read; this is rejected *before*
    /// the round-trip rather than after, leaving the frame queued.
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

    /// Read the target's addresses and transfer limits.
    ///
    /// The limits are what one `send` and one received frame can carry; size
    /// messages from them rather than assuming a number. They can change while
    /// the target runs (a controller can set them), so ask again when they matter.
    pub fn device_info(&mut self) -> Result<DeviceInfo, ClientError> {
        let mut resp = [0u8; MAX_FRAME];
        match self.transact(I3cOp::DeviceInfo, &[], &mut resp)? {
            (I3cStatus::Ok, body) => DeviceInfo::decode(body).ok_or(ClientError::InvalidResponse),
            (status, _) => Err(ClientError::Server(status)),
        }
    }
}
