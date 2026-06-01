// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP IPC Wire Protocol
//!
//! Binary wire protocol for MCTP operations over IPC channels.
//! Uses manual byte encoding for `no_std` compatibility.
//!
//! ## Wire Format
//!
//! ```text
//! Request (12 bytes header + optional payload):
//! ┌────┬───────┬──────────┬─────┬────────┬─────┬──────────┐
//! │ op │ flags │ msg_type │ eid │ handle │ tag │ reserved │
//! │ 1B │  1B   │   1B     │ 1B  │  4B LE │ 1B  │   3B     │
//! └────┴───────┴──────────┴─────┴────────┴─────┴──────────┘
//!
//! Response (12 bytes header + optional payload):
//! ┌──────┬───────┬──────────┬─────┬────────┬────────────┬─────┐
//! │ code │ flags │ msg_type │ eid │ handle │ payload_len│ tag │  + [payload]
//! │ 1B   │  1B   │   1B     │ 1B  │  4B LE │   2B LE    │ 1B  │
//! └──────┴───────┴──────────┴─────┴────────┴────────────┴─────┘
//! ```
//!
//! For `Recv` requests, the first 4 bytes of payload contain `timeout_millis` (u32 LE).
//! For `Send` requests, the MCTP payload follows the header.

use crate::ResponseCode;

// ============================================================================
// Wire Error
// ============================================================================

/// Error type for wire protocol encoding/decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Output buffer too small.
    BufferTooSmall,
    /// Payload exceeds maximum size.
    PayloadTooLarge,
    /// Unrecognized operation code.
    InvalidOpcode(u8),
    /// Input buffer too short for a complete header.
    Truncated,
}

// ============================================================================
// Operation Codes
// ============================================================================

/// MCTP IPC operation codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MctpOp {
    /// Set the local endpoint ID.
    SetEid = 0,
    /// Get the local endpoint ID.
    GetEid = 1,
    /// Register a listener for a message type.
    Listener = 2,
    /// Allocate a request handle for a remote EID.
    Req = 3,
    /// Receive a message on a handle.
    Recv = 4,
    /// Send a message.
    Send = 5,
    /// Release a handle.
    Unbind = 6,
}

impl MctpOp {
    /// Convert from raw byte.
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::SetEid),
            1 => Some(Self::GetEid),
            2 => Some(Self::Listener),
            3 => Some(Self::Req),
            4 => Some(Self::Recv),
            5 => Some(Self::Send),
            6 => Some(Self::Unbind),
            _ => None,
        }
    }
}

// ============================================================================
// Request Flags
// ============================================================================

/// Request flag bits.
pub mod flags {
    /// Integrity check bit in flags byte.
    pub const IC: u8 = 1 << 0;
    /// Handle field is valid.
    pub const HAS_HANDLE: u8 = 1 << 1;
    /// EID field is valid.
    pub const HAS_EID: u8 = 1 << 2;
    /// Tag field is valid.
    pub const HAS_TAG: u8 = 1 << 3;
}

// ============================================================================
// Request Header
// ============================================================================

/// MCTP request header (12 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MctpRequestHeader {
    /// Operation code.
    pub op: u8,
    /// Flags (see [`flags`] module).
    pub flags: u8,
    /// MCTP message type.
    pub msg_type: u8,
    /// Endpoint ID.
    pub eid: u8,
    /// Handle value.
    pub handle: u32,
    /// Tag value.
    pub tag: u8,
}

impl MctpRequestHeader {
    /// Header size in bytes.
    pub const SIZE: usize = 12;

    /// Encode to bytes (little-endian).
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let h = self.handle.to_le_bytes();
        [
            self.op,
            self.flags,
            self.msg_type,
            self.eid,
            h[0], h[1], h[2], h[3],
            self.tag,
            0, 0, 0, // reserved
        ]
    }

    /// Decode from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            op: bytes[0],
            flags: bytes[1],
            msg_type: bytes[2],
            eid: bytes[3],
            handle: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            tag: bytes[8],
        })
    }

    /// Get the operation code.
    pub fn operation(&self) -> Option<MctpOp> {
        MctpOp::from_u8(self.op)
    }
}

// ============================================================================
// Response Header
// ============================================================================

/// MCTP response header (12 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MctpResponseHeader {
    /// Response code.
    pub code: u8,
    /// Flags (bit 0 = msg_ic).
    pub flags: u8,
    /// MCTP message type (for Recv responses).
    pub msg_type: u8,
    /// Remote endpoint ID (for Recv responses).
    pub eid: u8,
    /// Handle (for Listener/Req) or 0.
    pub handle: u32,
    /// Payload length (for Recv responses).
    pub payload_len: u16,
    /// Message tag (for Recv/Send responses).
    pub tag: u8,
}

impl MctpResponseHeader {
    /// Header size in bytes.
    pub const SIZE: usize = 12;

    /// Create a success response with no data.
    pub const fn success() -> Self {
        Self {
            code: ResponseCode::Success as u8,
            flags: 0,
            msg_type: 0,
            eid: 0,
            handle: 0,
            payload_len: 0,
            tag: 0,
        }
    }

    /// Create an error response.
    pub const fn error(code: ResponseCode) -> Self {
        Self {
            code: code as u8,
            flags: 0,
            msg_type: 0,
            eid: 0,
            handle: 0,
            payload_len: 0,
            tag: 0,
        }
    }

    /// Check if the response indicates success.
    pub fn is_success(&self) -> bool {
        self.code == ResponseCode::Success as u8
    }

    /// Get the response code.
    pub fn response_code(&self) -> ResponseCode {
        ResponseCode::from_u8(self.code).unwrap_or(ResponseCode::InternalError)
    }

    /// Encode to bytes (little-endian).
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let h = self.handle.to_le_bytes();
        let pl = self.payload_len.to_le_bytes();
        [
            self.code,
            self.flags,
            self.msg_type,
            self.eid,
            h[0], h[1], h[2], h[3],
            pl[0], pl[1],
            self.tag,
            0, // reserved
        ]
    }

    /// Decode from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            code: bytes[0],
            flags: bytes[1],
            msg_type: bytes[2],
            eid: bytes[3],
            handle: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            payload_len: u16::from_le_bytes([bytes[8], bytes[9]]),
            tag: bytes[10],
        })
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Maximum MCTP payload size.
pub const MAX_PAYLOAD_SIZE: usize = 1023;

/// Maximum total request size (header + payload).
pub const MAX_REQUEST_SIZE: usize = MctpRequestHeader::SIZE + MAX_PAYLOAD_SIZE;

/// Maximum total response size (header + payload).
pub const MAX_RESPONSE_SIZE: usize = MctpResponseHeader::SIZE + MAX_PAYLOAD_SIZE;

/// Sentinel value for "no handle".
pub const NO_HANDLE: u32 = 0xFFFF_FFFF;

// ============================================================================
// Request Encoding
// ============================================================================

/// Encode a `SetEid` request.
pub fn encode_set_eid(buf: &mut [u8], eid: u8) -> Result<usize, WireError> {
    if buf.len() < MctpRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let header = MctpRequestHeader {
        op: MctpOp::SetEid as u8,
        flags: 0,
        msg_type: 0,
        eid,
        handle: 0,
        tag: 0,
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    Ok(MctpRequestHeader::SIZE)
}

/// Encode a `GetEid` request.
pub fn encode_get_eid(buf: &mut [u8]) -> Result<usize, WireError> {
    if buf.len() < MctpRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let header = MctpRequestHeader {
        op: MctpOp::GetEid as u8,
        flags: 0,
        msg_type: 0,
        eid: 0,
        handle: 0,
        tag: 0,
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    Ok(MctpRequestHeader::SIZE)
}

/// Encode a `Listener` request.
pub fn encode_listener(buf: &mut [u8], msg_type: u8) -> Result<usize, WireError> {
    if buf.len() < MctpRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let header = MctpRequestHeader {
        op: MctpOp::Listener as u8,
        flags: 0,
        msg_type,
        eid: 0,
        handle: 0,
        tag: 0,
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    Ok(MctpRequestHeader::SIZE)
}

/// Encode a `Req` request.
pub fn encode_req(buf: &mut [u8], eid: u8) -> Result<usize, WireError> {
    if buf.len() < MctpRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let header = MctpRequestHeader {
        op: MctpOp::Req as u8,
        flags: 0,
        msg_type: 0,
        eid,
        handle: 0,
        tag: 0,
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    Ok(MctpRequestHeader::SIZE)
}

/// Encode a `Recv` request.
pub fn encode_recv(buf: &mut [u8], handle: u32, timeout_millis: u32) -> Result<usize, WireError> {
    let total = MctpRequestHeader::SIZE + 4;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let header = MctpRequestHeader {
        op: MctpOp::Recv as u8,
        flags: flags::HAS_HANDLE,
        msg_type: 0,
        eid: 0,
        handle,
        tag: 0,
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    buf[MctpRequestHeader::SIZE..total].copy_from_slice(&timeout_millis.to_le_bytes());
    Ok(total)
}

/// Encode a `Send` request.
pub fn encode_send(
    buf: &mut [u8],
    handle: Option<u32>,
    msg_type: u8,
    eid: Option<u8>,
    tag: Option<u8>,
    ic: bool,
    payload: &[u8],
) -> Result<usize, WireError> {
    if payload.len() > MAX_PAYLOAD_SIZE {
        return Err(WireError::PayloadTooLarge);
    }
    let total = MctpRequestHeader::SIZE + payload.len();
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }

    let mut f: u8 = 0;
    if ic {
        f |= flags::IC;
    }
    if handle.is_some() {
        f |= flags::HAS_HANDLE;
    }
    if eid.is_some() {
        f |= flags::HAS_EID;
    }
    if tag.is_some() {
        f |= flags::HAS_TAG;
    }

    let header = MctpRequestHeader {
        op: MctpOp::Send as u8,
        flags: f,
        msg_type,
        eid: eid.unwrap_or(0),
        handle: handle.unwrap_or(NO_HANDLE),
        tag: tag.unwrap_or(0),
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    buf[MctpRequestHeader::SIZE..total].copy_from_slice(payload);
    Ok(total)
}

/// Encode an `Unbind` request.
pub fn encode_unbind(buf: &mut [u8], handle: u32) -> Result<usize, WireError> {
    if buf.len() < MctpRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let header = MctpRequestHeader {
        op: MctpOp::Unbind as u8,
        flags: flags::HAS_HANDLE,
        msg_type: 0,
        eid: 0,
        handle,
        tag: 0,
    };
    buf[..MctpRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    Ok(MctpRequestHeader::SIZE)
}

// ============================================================================
// Response Encoding (server side)
// ============================================================================

/// Encode a success response for `GetEid`.
pub fn encode_get_eid_response(buf: &mut [u8], eid: u8) -> Result<usize, WireError> {
    if buf.len() < MctpResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let mut resp = MctpResponseHeader::success();
    resp.eid = eid;
    buf[..MctpResponseHeader::SIZE].copy_from_slice(&resp.to_bytes());
    Ok(MctpResponseHeader::SIZE)
}

/// Encode a success response for `Listener` or `Req` (returns a handle).
pub fn encode_handle_response(buf: &mut [u8], handle: u32) -> Result<usize, WireError> {
    if buf.len() < MctpResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let mut resp = MctpResponseHeader::success();
    resp.handle = handle;
    buf[..MctpResponseHeader::SIZE].copy_from_slice(&resp.to_bytes());
    Ok(MctpResponseHeader::SIZE)
}

/// Encode a success response for `Send` (returns the tag).
pub fn encode_send_response(buf: &mut [u8], tag: u8) -> Result<usize, WireError> {
    if buf.len() < MctpResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let mut resp = MctpResponseHeader::success();
    resp.tag = tag;
    buf[..MctpResponseHeader::SIZE].copy_from_slice(&resp.to_bytes());
    Ok(MctpResponseHeader::SIZE)
}

/// Encode a success response for `Recv` (returns metadata + payload).
pub fn encode_recv_response(
    buf: &mut [u8],
    msg_type: u8,
    msg_ic: bool,
    eid: u8,
    tag: u8,
    payload: &[u8],
) -> Result<usize, WireError> {
    if payload.len() > MAX_PAYLOAD_SIZE {
        return Err(WireError::PayloadTooLarge);
    }
    let total = MctpResponseHeader::SIZE + payload.len();
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let resp = MctpResponseHeader {
        code: ResponseCode::Success as u8,
        flags: if msg_ic { flags::IC } else { 0 },
        msg_type,
        eid,
        handle: 0,
        payload_len: payload.len() as u16,
        tag,
    };
    buf[..MctpResponseHeader::SIZE].copy_from_slice(&resp.to_bytes());
    buf[MctpResponseHeader::SIZE..total].copy_from_slice(payload);
    Ok(total)
}

/// Encode a simple success response (no data).
pub fn encode_success_response(buf: &mut [u8]) -> Result<usize, WireError> {
    if buf.len() < MctpResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    buf[..MctpResponseHeader::SIZE].copy_from_slice(&MctpResponseHeader::success().to_bytes());
    Ok(MctpResponseHeader::SIZE)
}

/// Encode an error response.
pub fn encode_error_response(buf: &mut [u8], code: ResponseCode) -> Result<usize, WireError> {
    if buf.len() < MctpResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    buf[..MctpResponseHeader::SIZE].copy_from_slice(&MctpResponseHeader::error(code).to_bytes());
    Ok(MctpResponseHeader::SIZE)
}

// ============================================================================
// Response Decoding (client side)
// ============================================================================

/// Decode a response header.
pub fn decode_response_header(buf: &[u8]) -> Result<MctpResponseHeader, WireError> {
    MctpResponseHeader::from_bytes(buf).ok_or(WireError::Truncated)
}

/// Get response payload data (after header).
pub fn get_response_payload<'a>(buf: &'a [u8], header: &MctpResponseHeader) -> Result<&'a [u8], WireError> {
    let end = MctpResponseHeader::SIZE + header.payload_len as usize;
    if buf.len() < end {
        return Err(WireError::Truncated);
    }
    Ok(&buf[MctpResponseHeader::SIZE..end])
}

/// Decode a request header.
pub fn decode_request_header(buf: &[u8]) -> Result<MctpRequestHeader, WireError> {
    MctpRequestHeader::from_bytes(buf).ok_or(WireError::Truncated)
}

/// Get request payload (after header, for Send operations).
pub fn get_request_payload(buf: &[u8]) -> &[u8] {
    if buf.len() > MctpRequestHeader::SIZE {
        &buf[MctpRequestHeader::SIZE..]
    } else {
        &[]
    }
}

/// Extract the `timeout_millis` field from a `Recv` request.
///
/// The 4-byte little-endian timeout follows the 12-byte request header.
/// Returns 0 if the buffer is too short (no timeout → wait forever).
pub fn get_recv_timeout(buf: &[u8]) -> u32 {
    let start = MctpRequestHeader::SIZE;
    if buf.len() < start + 4 {
        return 0;
    }
    u32::from_le_bytes(buf[start..start + 4].try_into().unwrap())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_header_roundtrip() {
        let header = MctpRequestHeader {
            op: MctpOp::Send as u8,
            flags: flags::HAS_HANDLE | flags::IC,
            msg_type: 1,
            eid: 42,
            handle: 0x1234,
            tag: 7,
        };
        let bytes = header.to_bytes();
        let decoded = MctpRequestHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.op, MctpOp::Send as u8);
        assert_eq!(decoded.flags, flags::HAS_HANDLE | flags::IC);
        assert_eq!(decoded.msg_type, 1);
        assert_eq!(decoded.eid, 42);
        assert_eq!(decoded.handle, 0x1234);
        assert_eq!(decoded.tag, 7);
    }

    #[test]
    fn response_header_roundtrip() {
        let header = MctpResponseHeader {
            code: ResponseCode::Success as u8,
            flags: flags::IC,
            msg_type: 1,
            eid: 8,
            handle: 0,
            payload_len: 16,
            tag: 3,
        };
        let bytes = header.to_bytes();
        let decoded = MctpResponseHeader::from_bytes(&bytes).unwrap();

        assert!(decoded.is_success());
        assert_eq!(decoded.flags & flags::IC, flags::IC);
        assert_eq!(decoded.msg_type, 1);
        assert_eq!(decoded.eid, 8);
        assert_eq!(decoded.payload_len, 16);
        assert_eq!(decoded.tag, 3);
    }

    #[test]
    fn encode_send_roundtrip() {
        let mut buf = [0u8; 64];
        let payload = b"hello";
        let len = encode_send(&mut buf, Some(5), 1, Some(8), Some(3), true, payload).unwrap();

        let header = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(header.operation(), Some(MctpOp::Send));
        assert_eq!(header.flags & flags::IC, flags::IC);
        assert_eq!(header.flags & flags::HAS_HANDLE, flags::HAS_HANDLE);
        assert_eq!(header.flags & flags::HAS_EID, flags::HAS_EID);
        assert_eq!(header.flags & flags::HAS_TAG, flags::HAS_TAG);
        assert_eq!(header.msg_type, 1);
        assert_eq!(header.eid, 8);
        assert_eq!(header.handle, 5);
        assert_eq!(header.tag, 3);
        assert_eq!(&buf[MctpRequestHeader::SIZE..len], b"hello");
    }

    #[test]
    fn encode_recv_roundtrip() {
        let mut buf = [0u8; 32];
        let len = encode_recv(&mut buf, 7, 5000).unwrap();
        assert_eq!(len, MctpRequestHeader::SIZE + 4);

        let header = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(header.operation(), Some(MctpOp::Recv));
        assert_eq!(header.handle, 7);

        let timeout = u32::from_le_bytes([
            buf[MctpRequestHeader::SIZE],
            buf[MctpRequestHeader::SIZE + 1],
            buf[MctpRequestHeader::SIZE + 2],
            buf[MctpRequestHeader::SIZE + 3],
        ]);
        assert_eq!(timeout, 5000);
    }

    #[test]
    fn error_response() {
        let mut buf = [0u8; 16];
        let len = encode_error_response(&mut buf, ResponseCode::NoSpace).unwrap();
        assert_eq!(len, MctpResponseHeader::SIZE);

        let header = decode_response_header(&buf).unwrap();
        assert!(!header.is_success());
        assert_eq!(header.response_code(), ResponseCode::NoSpace);
    }

    #[test]
    fn recv_response_with_payload() {
        let mut buf = [0u8; 64];
        let payload = b"echo data";
        let len = encode_recv_response(&mut buf, 1, false, 42, 5, payload).unwrap();

        let header = decode_response_header(&buf).unwrap();
        assert!(header.is_success());
        assert_eq!(header.msg_type, 1);
        assert_eq!(header.eid, 42);
        assert_eq!(header.tag, 5);
        assert_eq!(header.payload_len, 9);

        let data = get_response_payload(&buf[..len], &header).unwrap();
        assert_eq!(data, b"echo data");
    }

    #[test]
    fn send_payload_too_large() {
        let mut buf = [0u8; 2048];
        let oversized = [0u8; MAX_PAYLOAD_SIZE + 1];
        assert_eq!(
            encode_send(&mut buf, None, 1, None, None, false, &oversized),
            Err(WireError::PayloadTooLarge)
        );
    }

    // -------------------------------------------------------------------------
    // encode_set_eid / encode_get_eid
    // -------------------------------------------------------------------------

    #[test]
    fn encode_set_eid_ok() {
        let mut buf = [0u8; 16];
        let len = encode_set_eid(&mut buf, 42).unwrap();
        assert_eq!(len, MctpRequestHeader::SIZE);
        let h = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(h.operation(), Some(MctpOp::SetEid));
        assert_eq!(h.eid, 42);
    }

    #[test]
    fn encode_set_eid_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(encode_set_eid(&mut buf, 1), Err(WireError::BufferTooSmall));
    }

    #[test]
    fn encode_get_eid_ok() {
        let mut buf = [0u8; 16];
        let len = encode_get_eid(&mut buf).unwrap();
        assert_eq!(len, MctpRequestHeader::SIZE);
        let h = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(h.operation(), Some(MctpOp::GetEid));
    }

    #[test]
    fn encode_get_eid_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(encode_get_eid(&mut buf), Err(WireError::BufferTooSmall));
    }

    // -------------------------------------------------------------------------
    // encode_listener / encode_req / encode_unbind
    // -------------------------------------------------------------------------

    #[test]
    fn encode_listener_ok() {
        let mut buf = [0u8; 16];
        let len = encode_listener(&mut buf, 0x05).unwrap();
        assert_eq!(len, MctpRequestHeader::SIZE);
        let h = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(h.operation(), Some(MctpOp::Listener));
        assert_eq!(h.msg_type, 0x05);
    }

    #[test]
    fn encode_req_ok() {
        let mut buf = [0u8; 16];
        let len = encode_req(&mut buf, 7).unwrap();
        assert_eq!(len, MctpRequestHeader::SIZE);
        let h = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(h.operation(), Some(MctpOp::Req));
        assert_eq!(h.eid, 7);
    }

    #[test]
    fn encode_unbind_ok() {
        let mut buf = [0u8; 16];
        let len = encode_unbind(&mut buf, 0xABCD).unwrap();
        assert_eq!(len, MctpRequestHeader::SIZE);
        let h = MctpRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(h.operation(), Some(MctpOp::Unbind));
        assert_eq!(h.handle, 0xABCD);
        assert_ne!(h.flags & flags::HAS_HANDLE, 0);
    }

    #[test]
    fn encode_unbind_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(encode_unbind(&mut buf, 1), Err(WireError::BufferTooSmall));
    }

    // -------------------------------------------------------------------------
    // encode_recv buffer-too-small
    // -------------------------------------------------------------------------

    #[test]
    fn encode_recv_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(encode_recv(&mut buf, 1, 0), Err(WireError::BufferTooSmall));
    }

    // -------------------------------------------------------------------------
    // encode_send buffer-too-small
    // -------------------------------------------------------------------------

    #[test]
    fn encode_send_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(
            encode_send(&mut buf, None, 1, None, None, false, b"hi"),
            Err(WireError::BufferTooSmall)
        );
    }

    // -------------------------------------------------------------------------
    // Response encoders
    // -------------------------------------------------------------------------

    #[test]
    fn encode_get_eid_response_ok() {
        let mut buf = [0u8; 16];
        let len = encode_get_eid_response(&mut buf, 55).unwrap();
        assert_eq!(len, MctpResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
        assert_eq!(h.eid, 55);
    }

    #[test]
    fn encode_handle_response_ok() {
        let mut buf = [0u8; 16];
        let len = encode_handle_response(&mut buf, 0xDEAD).unwrap();
        assert_eq!(len, MctpResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
        assert_eq!(h.handle, 0xDEAD);
    }

    #[test]
    fn encode_send_response_ok() {
        let mut buf = [0u8; 16];
        let len = encode_send_response(&mut buf, 3).unwrap();
        assert_eq!(len, MctpResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
        assert_eq!(h.tag, 3);
    }

    #[test]
    fn encode_success_response_ok() {
        let mut buf = [0u8; 16];
        let len = encode_success_response(&mut buf).unwrap();
        assert_eq!(len, MctpResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
    }

    #[test]
    fn encode_success_response_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(
            encode_success_response(&mut buf),
            Err(WireError::BufferTooSmall)
        );
    }

    #[test]
    fn encode_recv_response_payload_too_large() {
        let mut buf = [0u8; 2048];
        let oversized = [0u8; MAX_PAYLOAD_SIZE + 1];
        assert_eq!(
            encode_recv_response(&mut buf, 1, false, 0, 0, &oversized),
            Err(WireError::PayloadTooLarge)
        );
    }

    #[test]
    fn encode_recv_response_buffer_too_small() {
        let mut buf = [0u8; 4];
        assert_eq!(
            encode_recv_response(&mut buf, 1, false, 0, 0, b"hi"),
            Err(WireError::BufferTooSmall)
        );
    }

    // -------------------------------------------------------------------------
    // decode_request_header / get_request_payload / Truncated
    // -------------------------------------------------------------------------

    #[test]
    fn decode_request_header_ok() {
        let mut buf = [0u8; 16];
        encode_listener(&mut buf, 0x0A).unwrap();
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(MctpOp::Listener));
        assert_eq!(h.msg_type, 0x0A);
    }

    #[test]
    fn decode_request_header_truncated() {
        let buf = [0u8; 4];
        assert_eq!(decode_request_header(&buf), Err(WireError::Truncated));
    }

    #[test]
    fn decode_response_header_truncated() {
        let buf = [0u8; 4];
        assert_eq!(decode_response_header(&buf), Err(WireError::Truncated));
    }

    #[test]
    fn get_request_payload_ok() {
        let mut buf = [0u8; 64];
        let len = encode_send(&mut buf, None, 1, None, None, false, b"data").unwrap();
        let payload = get_request_payload(&buf[..len]);
        assert_eq!(payload, b"data");
    }

    #[test]
    fn get_request_payload_empty() {
        let mut buf = [0u8; 16];
        encode_get_eid(&mut buf).unwrap();
        let payload = get_request_payload(&buf[..MctpRequestHeader::SIZE]);
        assert_eq!(payload, b"");
    }

    #[test]
    fn get_response_payload_truncated() {
        let mut buf = [0u8; 16];
        // Header claims payload_len=100 but buffer is only header-sized
        let mut h = MctpResponseHeader::success();
        h.payload_len = 100;
        buf[..MctpResponseHeader::SIZE].copy_from_slice(&h.to_bytes());
        assert_eq!(
            get_response_payload(&buf[..MctpResponseHeader::SIZE], &h),
            Err(WireError::Truncated)
        );
    }

    // -------------------------------------------------------------------------
    // MctpOp::from_u8 unknown opcode
    // -------------------------------------------------------------------------

    #[test]
    fn mctp_op_unknown() {
        assert_eq!(MctpOp::from_u8(0xFF), None);
    }
}
