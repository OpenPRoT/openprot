// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The wire the PLDM service uses to tell the orchestrator that something has
//! happened.
//!
//! PLDM initiates and the orchestrator handles, because PLDM is where these
//! things are known and the orchestrator's loop already waits on several
//! objects. Contract only, no transport and no state, so both processes depend
//! on it and it builds and tests on the host.
//!
//! ```text
//! Request (4 bytes + payload):     Response (4 bytes + payload):
//! ┌─────┬─────┬──────────┐         ┌──────┬─────┬──────────┐
//! │ op  │ len │ reserved │         │ code │ len │ reserved │
//! │ 1B  │ 1B  │    2B    │         │  1B  │ 1B  │    2B    │
//! └─────┴─────┴──────────┘         └──────┴─────┴──────────┘
//! ```
//!
//! One op today, [`NotifyOp::UpdateRequested`], and two answers,
//! [`Response::Accepted`] and [`Response::Rejected`]. All carry no payload,
//! so a frame is 4 bytes each way.
//!
//! Adding a notification: give it the next [`NotifyOp`] discriminant, and if it
//! carries data, put the length in `len` and raise [`MAX_REQUEST_SIZE`]. `len`
//! exists now so that does not change the frame format. Nothing here is
//! specific to firmware update; the opcode space is flat and any PLDM type can
//! take a range of it. Both enums are `#[non_exhaustive]` because the two sides
//! are separate processes that may be built from different revisions.
//!
//! Coalescing and latching live on the sender side, not here. The firmware
//! device fires `UpdateRequested` only on the `!was_update_mode &&
//! is_update_mode()` edge, so a second `RequestUpdate` while already in update
//! mode never reaches the channel.
//!
//! The request side is another process and is treated as untrusted: decoding
//! validates length, opcode, reserved fields, and that `len` is zero on an op
//! that carries nothing. Nothing here panics.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Largest payload any op carries. Zero while every op is a bare
/// notification; raise it with the first op that carries data.
pub const MAX_PAYLOAD: usize = 0;

/// Request buffer size a handler must provide.
pub const MAX_REQUEST_SIZE: usize = RequestHeader::SIZE + MAX_PAYLOAD;

/// Response buffer size an initiator must provide.
pub const MAX_RESPONSE_SIZE: usize = ResponseHeader::SIZE;

/// Why a buffer did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Shorter than the message the header declares.
    Truncated,
    /// The output buffer cannot hold the encoding.
    BufferTooSmall,
    /// The op byte names no operation this build knows.
    InvalidOpcode(u8),
    /// A field carries a value this op does not define: a reserved field set,
    /// `len` non-zero on an op that carries nothing, or a response code that
    /// names no answer.
    InvalidField,
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            WireError::Truncated => "buffer shorter than the message",
            WireError::BufferTooSmall => "buffer too small for the message",
            WireError::InvalidOpcode(_) => "unknown operation code",
            WireError::InvalidField => "field value not defined for this operation",
        })
    }
}

impl core::error::Error for WireError {}

/// What the PLDM service can report. One per thing that has happened, whatever
/// part of PLDM it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[repr(u8)]
pub enum NotifyOp {
    /// A UA sent a `RequestUpdate`. The orchestrator's answer decides whether
    /// the FD accepts.
    UpdateRequested = 0,
}

impl TryFrom<u8> for NotifyOp {
    type Error = WireError;

    fn try_from(value: u8) -> Result<Self, WireError> {
        match value {
            0 => Ok(NotifyOp::UpdateRequested),
            other => Err(WireError::InvalidOpcode(other)),
        }
    }
}

/// One notification from the PLDM service to the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    /// [`NotifyOp::UpdateRequested`]. Carries nothing: the request itself is
    /// the whole message.
    UpdateRequested,
}

impl Request {
    /// The op this request encodes as.
    pub fn op(&self) -> NotifyOp {
        match self {
            Request::UpdateRequested => NotifyOp::UpdateRequested,
        }
    }

    /// Encodes into `buf`, returning the encoded length.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        let header = RequestHeader {
            op_code: self.op() as u8,
            len: 0,
            reserved: 0,
        };
        let out = buf
            .get_mut(..RequestHeader::SIZE)
            .ok_or(WireError::BufferTooSmall)?;
        out.copy_from_slice(header.as_bytes());
        Ok(RequestHeader::SIZE)
    }

    /// Decodes one notification out of a handler's read buffer.
    pub fn decode(buf: &[u8]) -> Result<Request, WireError> {
        let head = buf.get(..RequestHeader::SIZE).ok_or(WireError::Truncated)?;
        let header = RequestHeader::read_from_bytes(head).map_err(|_| WireError::Truncated)?;
        if header.reserved != 0 {
            return Err(WireError::InvalidField);
        }
        match NotifyOp::try_from(header.op_code)? {
            NotifyOp::UpdateRequested => {
                if header.len != 0 {
                    return Err(WireError::InvalidField);
                }
                Ok(Request::UpdateRequested)
            }
        }
    }
}

/// What the orchestrator answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[repr(u8)]
pub enum Response {
    /// The orchestrator accepts the notification and will act on it.
    Accepted = 0,
    /// The orchestrator rejects the request (e.g., already updating,
    /// recovering, locked, or policy).
    Rejected = 1,
}

impl Response {
    /// Encodes into `buf`, returning the encoded length.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        let header = ResponseHeader {
            code: *self as u8,
            len: 0,
            reserved: 0,
        };
        let out = buf
            .get_mut(..ResponseHeader::SIZE)
            .ok_or(WireError::BufferTooSmall)?;
        out.copy_from_slice(header.as_bytes());
        Ok(ResponseHeader::SIZE)
    }

    /// Decodes the orchestrator's answer.
    pub fn decode(buf: &[u8]) -> Result<Response, WireError> {
        let head = buf
            .get(..ResponseHeader::SIZE)
            .ok_or(WireError::Truncated)?;
        let header = ResponseHeader::read_from_bytes(head).map_err(|_| WireError::Truncated)?;
        if header.reserved != 0 || header.len != 0 {
            return Err(WireError::InvalidField);
        }
        match header.code {
            0 => Ok(Response::Accepted),
            1 => Ok(Response::Rejected),
            _ => Err(WireError::InvalidField),
        }
    }
}

/// The 4-byte request header. Fields are private because [`Request::decode`]
/// is what validates them.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct RequestHeader {
    op_code: u8,
    len: u8,
    reserved: u16,
}

impl RequestHeader {
    const SIZE: usize = core::mem::size_of::<Self>();
}

/// The 4-byte response header.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct ResponseHeader {
    code: u8,
    len: u8,
    reserved: u16,
}

impl ResponseHeader {
    const SIZE: usize = core::mem::size_of::<Self>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_notification_round_trips() {
        let request = Request::UpdateRequested;
        let mut buf = [0u8; MAX_REQUEST_SIZE];
        let len = request.encode(&mut buf).unwrap();
        assert_eq!(Request::decode(&buf[..len]), Ok(request));
    }

    #[test]
    fn every_answer_round_trips() {
        for answer in [Response::Accepted, Response::Rejected] {
            let mut buf = [0u8; MAX_RESPONSE_SIZE];
            let len = answer.encode(&mut buf).unwrap();
            assert_eq!(Response::decode(&buf[..len]), Ok(answer));
        }
    }

    #[test]
    fn a_short_buffer_is_truncated() {
        assert_eq!(Request::decode(&[0, 0, 0]), Err(WireError::Truncated));
        assert_eq!(Response::decode(&[0, 0, 0]), Err(WireError::Truncated));
    }

    #[test]
    fn an_unknown_opcode_is_refused() {
        assert_eq!(
            Request::decode(&[0xFF, 0, 0, 0]),
            Err(WireError::InvalidOpcode(0xFF))
        );
    }

    #[test]
    fn a_set_reserved_field_is_refused() {
        assert_eq!(
            Request::decode(&[NotifyOp::UpdateRequested as u8, 0, 1, 0]),
            Err(WireError::InvalidField)
        );
        assert_eq!(
            Response::decode(&[Response::Accepted as u8, 0, 0, 1]),
            Err(WireError::InvalidField)
        );
    }

    /// Every op carries nothing today, so a length is a malformed frame rather
    /// than a payload this build should skip.
    #[test]
    fn a_length_on_an_op_that_carries_nothing_is_refused() {
        assert_eq!(
            Request::decode(&[NotifyOp::UpdateRequested as u8, 1, 0, 0]),
            Err(WireError::InvalidField)
        );
        assert_eq!(
            Response::decode(&[Response::Accepted as u8, 1, 0, 0]),
            Err(WireError::InvalidField)
        );
    }

    #[test]
    fn an_unknown_answer_is_refused() {
        for code in [2u8, 0xFF] {
            assert_eq!(
                Response::decode(&[code, 0, 0, 0]),
                Err(WireError::InvalidField)
            );
        }
    }

    #[test]
    fn an_output_buffer_too_small_is_refused() {
        let mut buf = [0u8; RequestHeader::SIZE - 1];
        assert_eq!(
            Request::UpdateRequested.encode(&mut buf),
            Err(WireError::BufferTooSmall)
        );
        assert_eq!(
            Response::Accepted.encode(&mut buf),
            Err(WireError::BufferTooSmall)
        );
    }
}
