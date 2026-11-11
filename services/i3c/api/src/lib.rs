// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C target IPC transport protocol.
//!
//! The wire contract between the i3c server-runtime (which owns the
//! [`I3cTarget`](openprot_hal_blocking::i3c_hardware::I3cTarget) facade) and
//! any client process that speaks to it over a Pigweed channel — the i3c IPC
//! transport client that the `services/mctp` binding drives.
//!
//! Both directions are a single opcode/status byte followed by an optional
//! payload; there is one operation per whole request. This crate is
//! host-buildable and kernel-free: the marshalling is verified in host tests,
//! and the server-runtime and client only supply the syscalls.
//!
//! ```text
//! request : [op: u8][payload …]
//! response: [status: u8][payload …]
//! ```

#![no_std]

mod transport;

pub use transport::{Transport, TransportError};

/// Largest message payload carried in one request or response, matching the
/// caliptra i3c-core private read/write limit.
pub const MAX_PAYLOAD: usize = 250;

/// Size of the one-byte opcode/status header on each frame.
pub const HEADER: usize = 1;

/// Largest whole frame (header + payload) in either direction.
pub const MAX_FRAME: usize = HEADER + MAX_PAYLOAD;

/// Operation a client asks the i3c server to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum I3cOp {
    /// Transmit `payload` to the controller (staged TX + IBI).
    Send = 0,
    /// Return the latched inbound frame, or `NoData` if none is pending.
    Recv = 1,
    /// Return the assigned 7-bit dynamic address, or `Unassigned`.
    DynamicAddress = 2,
}

impl I3cOp {
    /// Decode an opcode byte.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Send),
            1 => Some(Self::Recv),
            2 => Some(Self::DynamicAddress),
            _ => None,
        }
    }
}

/// Result byte the server returns for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum I3cStatus {
    /// Success; any payload follows.
    Ok = 0,
    /// `Recv` with no inbound frame latched.
    NoData = 1,
    /// `DynamicAddress` before the controller has assigned one.
    Unassigned = 2,
    /// `Send` payload exceeded [`MAX_PAYLOAD`].
    TooLong = 3,
    /// The facade reported a hardware error.
    Internal = 4,
    /// Opcode byte was unknown or the frame was empty.
    InvalidOp = 5,
}

impl I3cStatus {
    /// Decode a status byte.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Ok),
            1 => Some(Self::NoData),
            2 => Some(Self::Unassigned),
            3 => Some(Self::TooLong),
            4 => Some(Self::Internal),
            5 => Some(Self::InvalidOp),
            _ => None,
        }
    }
}

/// Encode a request into `buf`, returning the frame length.
///
/// Fails if `payload` exceeds [`MAX_PAYLOAD`] or `buf` is too small.
pub fn encode_request(op: I3cOp, payload: &[u8], buf: &mut [u8]) -> Option<usize> {
    if payload.len() > MAX_PAYLOAD || buf.len() < HEADER + payload.len() {
        return None;
    }
    let (head, body) = buf.split_first_mut()?;
    *head = op as u8;
    body.get_mut(..payload.len())?.copy_from_slice(payload);
    Some(HEADER + payload.len())
}

/// Decode a request frame into its opcode and payload slice.
pub fn decode_request(frame: &[u8]) -> Option<(I3cOp, &[u8])> {
    let (&head, body) = frame.split_first()?;
    Some((I3cOp::from_u8(head)?, body))
}

/// Encode a response into `buf`, returning the frame length.
///
/// Fails if `payload` exceeds [`MAX_PAYLOAD`] or `buf` is too small.
pub fn encode_response(status: I3cStatus, payload: &[u8], buf: &mut [u8]) -> Option<usize> {
    if payload.len() > MAX_PAYLOAD || buf.len() < HEADER + payload.len() {
        return None;
    }
    let (head, body) = buf.split_first_mut()?;
    *head = status as u8;
    body.get_mut(..payload.len())?.copy_from_slice(payload);
    Some(HEADER + payload.len())
}

/// Decode a response frame into its status and payload slice.
pub fn decode_response(frame: &[u8]) -> Option<(I3cStatus, &[u8])> {
    let (&head, body) = frame.split_first()?;
    Some((I3cStatus::from_u8(head)?, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_round_trip() {
        for op in [I3cOp::Send, I3cOp::Recv, I3cOp::DynamicAddress] {
            assert_eq!(I3cOp::from_u8(op as u8), Some(op));
        }
        assert_eq!(I3cOp::from_u8(3), None);
    }

    #[test]
    fn status_round_trip() {
        for st in [
            I3cStatus::Ok,
            I3cStatus::NoData,
            I3cStatus::Unassigned,
            I3cStatus::TooLong,
            I3cStatus::Internal,
            I3cStatus::InvalidOp,
        ] {
            assert_eq!(I3cStatus::from_u8(st as u8), Some(st));
        }
        assert_eq!(I3cStatus::from_u8(6), None);
    }

    #[test]
    fn request_encode_decode() {
        let mut buf = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::Send, b"ping", &mut buf).unwrap();
        assert_eq!(n, HEADER + 4);
        assert_eq!(decode_request(&buf[..n]), Some((I3cOp::Send, &b"ping"[..])));

        let n = encode_request(I3cOp::Recv, &[], &mut buf).unwrap();
        assert_eq!(decode_request(&buf[..n]), Some((I3cOp::Recv, &[][..])));
    }

    #[test]
    fn response_encode_decode() {
        let mut buf = [0u8; MAX_FRAME];
        let n = encode_response(I3cStatus::Ok, b"pong", &mut buf).unwrap();
        assert_eq!(
            decode_response(&buf[..n]),
            Some((I3cStatus::Ok, &b"pong"[..]))
        );

        let n = encode_response(I3cStatus::NoData, &[], &mut buf).unwrap();
        assert_eq!(
            decode_response(&buf[..n]),
            Some((I3cStatus::NoData, &[][..]))
        );
    }

    #[test]
    fn oversize_payload_rejected() {
        let big = [0u8; MAX_PAYLOAD + 1];
        let mut buf = [0u8; MAX_FRAME + 8];
        assert_eq!(encode_request(I3cOp::Send, &big, &mut buf), None);
        assert_eq!(encode_response(I3cStatus::Ok, &big, &mut buf), None);
    }

    #[test]
    fn empty_frame_is_none() {
        assert_eq!(decode_request(&[]), None);
        assert_eq!(decode_response(&[]), None);
    }
}
