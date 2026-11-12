// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C target IPC transport protocol.
//!
//! The wire contract between the i3c server-runtime (which owns the
//! [`I3cTarget`](openprot_hal_blocking::i3c_hardware::I3cTarget) facade) and
//! any client process that speaks to it over a Pigweed channel. A consumer such
//! as an MCTP binding sits on top of the client.
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

// The transport seam is the shared one in //util/transport: one whole serialized
// request in, one whole response out. `I3cClient` (in `i3c-client`) is generic
// over it, so swapping `IpcTransport` (in `i3c-client-ipc`, the cross-process
// path) for `LoopbackTransport` (in `i3c-server`, an in-process call to
// `dispatch`) is a wiring choice, never a code fork.
pub use util_transport::{Transport, TransportError};

/// Largest message payload carried in one request or response: the service's
/// protocol ceiling.
///
/// A frame from the controller is at most 256 bytes on the hardware this was
/// written against (a 64-DWORD FIFO), so the ceiling is 256; it sizes the wire
/// frame, every inbound ring slot, and the client buffers. A target that can
/// stage less reports a smaller limit through `I3cTarget::max_send_len`, which
/// `dispatch` enforces, and an inbound frame must never exceed this ceiling.
pub const MAX_PAYLOAD: usize = 256;

/// Size of the one-byte opcode/status header on each frame.
pub const HEADER: usize = 1;

/// Largest whole frame (header + payload) in either direction.
pub const MAX_FRAME: usize = HEADER + MAX_PAYLOAD;

/// Operation a client asks the i3c server to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum I3cOp {
    /// Transmit `payload` to the controller (staged TX + IBI). At most one
    /// response is staged at a time: a `Send` while one is still unread by the
    /// controller is answered with [`I3cStatus::Busy`].
    Send = 0,
    /// Return the latched inbound frame, or `NoData` if none is pending.
    Recv = 1,
    /// Return the assigned 7-bit dynamic address, or `Unassigned`.
    DynamicAddress = 2,
    /// Return the target's addresses and transfer limits as a [`DeviceInfo`].
    DeviceInfo = 3,
}

impl I3cOp {
    /// Decode an opcode byte.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Send),
            1 => Some(Self::Recv),
            2 => Some(Self::DynamicAddress),
            3 => Some(Self::DeviceInfo),
            _ => None,
        }
    }
}

/// Size of the [`DeviceInfo`] payload.
pub const DEVICE_INFO_LEN: usize = 6;

/// A target's addresses and transfer limits, as `I3cOp::DeviceInfo` returns them.
///
/// The limits are what the target can carry in one transfer, already clamped to
/// the service's frame ceiling ([`MAX_PAYLOAD`]), so a client can size a message
/// from them directly: the most it can `Send`, and the most it will `Recv`. They
/// can change while the target runs (a controller can set them), so ask again
/// when they matter.
///
/// ```text
/// [dynamic: u8][static: u8][max_send: u16 LE][max_recv: u16 LE]
/// ```
///
/// Address 0 means "none": 0x00 is reserved on an I3C bus, so it cannot be a
/// real address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceInfo {
    /// The dynamic address currently assigned, if any.
    pub dynamic_address: Option<u8>,
    /// The static address the target answers on before assignment, if any.
    pub static_address: Option<u8>,
    /// The most bytes one `Send` can carry.
    pub max_send_len: u16,
    /// The most bytes one received frame can carry.
    pub max_recv_len: u16,
}

impl DeviceInfo {
    /// Encode into `buf`, returning the length ([`DEVICE_INFO_LEN`]), or `None`
    /// if `buf` is too small.
    pub fn encode(&self, buf: &mut [u8]) -> Option<usize> {
        let out = buf.get_mut(..DEVICE_INFO_LEN)?;
        out[0] = self.dynamic_address.unwrap_or(0);
        out[1] = self.static_address.unwrap_or(0);
        out[2..4].copy_from_slice(&self.max_send_len.to_le_bytes());
        out[4..6].copy_from_slice(&self.max_recv_len.to_le_bytes());
        Some(DEVICE_INFO_LEN)
    }

    /// Decode a payload of exactly [`DEVICE_INFO_LEN`] bytes.
    pub fn decode(payload: &[u8]) -> Option<Self> {
        let &[dynamic, stat, send_lo, send_hi, recv_lo, recv_hi] = payload else {
            return None;
        };
        let addr = |a: u8| if a == 0 { None } else { Some(a) };
        Some(Self {
            dynamic_address: addr(dynamic),
            static_address: addr(stat),
            max_send_len: u16::from_le_bytes([send_lo, send_hi]),
            max_recv_len: u16::from_le_bytes([recv_lo, recv_hi]),
        })
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
    /// `Send` while the previous staged response has not been read by the
    /// controller yet. Nothing was staged; retry once the controller has read it.
    Busy = 6,
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
            6 => Some(Self::Busy),
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
        for op in [
            I3cOp::Send,
            I3cOp::Recv,
            I3cOp::DynamicAddress,
            I3cOp::DeviceInfo,
        ] {
            assert_eq!(I3cOp::from_u8(op as u8), Some(op));
        }
        assert_eq!(I3cOp::from_u8(4), None);
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
            I3cStatus::Busy,
        ] {
            assert_eq!(I3cStatus::from_u8(st as u8), Some(st));
        }
        assert_eq!(I3cStatus::from_u8(7), None);
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
    fn device_info_round_trips() {
        let info = DeviceInfo {
            dynamic_address: Some(0x08),
            static_address: Some(0x30),
            max_send_len: 256,
            max_recv_len: 64,
        };
        let mut buf = [0u8; 8];
        assert_eq!(info.encode(&mut buf), Some(DEVICE_INFO_LEN));
        assert_eq!(
            &buf[..DEVICE_INFO_LEN],
            &[0x08, 0x30, 0x00, 0x01, 0x40, 0x00]
        );
        assert_eq!(DeviceInfo::decode(&buf[..DEVICE_INFO_LEN]), Some(info));
    }

    #[test]
    fn device_info_zero_address_means_none() {
        let mut buf = [0u8; DEVICE_INFO_LEN];
        let info = DeviceInfo {
            dynamic_address: None,
            static_address: None,
            max_send_len: 1,
            max_recv_len: 2,
        };
        info.encode(&mut buf).unwrap();
        assert_eq!(&buf[..2], &[0, 0]);
        assert_eq!(DeviceInfo::decode(&buf), Some(info));
    }

    #[test]
    fn device_info_rejects_the_wrong_length() {
        let mut small = [0u8; DEVICE_INFO_LEN - 1];
        assert_eq!(DeviceInfo::decode(&small), None);
        assert_eq!(DeviceInfo::decode(&[0u8; DEVICE_INFO_LEN + 1]), None);
        let info = DeviceInfo {
            dynamic_address: None,
            static_address: None,
            max_send_len: 0,
            max_recv_len: 0,
        };
        assert_eq!(info.encode(&mut small), None);
    }

    #[test]
    fn empty_frame_is_none() {
        assert_eq!(decode_request(&[]), None);
        assert_eq!(decode_response(&[]), None);
    }
}
