// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Crypto IPC wire protocol.
//!
//! Binary protocol for orchestrator-to-crypto-service operations over
//! IPC channels. Same 8-byte header shape as the PLDM IPC protocol.
//!
//! ```text
//! Request (8-byte header + optional args):
//! +----+-------+----------+
//! | op | flags | reserved |  + [args]
//! | 1B |  1B   |    6B    |
//! +----+-------+----------+
//!
//! Response (8-byte header + optional payload):
//! +------+-------+-------------+----------+
//! | code | flags | payload_len | reserved |  + [payload]
//! |  1B  |  1B   |    2B LE    |    4B    |
//! +------+-------+-------------+----------+
//! ```

use crate::error::{ResponseCode, WireError};

// ============================================================================
// Operation codes
// ============================================================================

/// Orchestrator-to-crypto IPC operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CryptoOp {
    /// Begin verifying a flash region. Args: address (4B LE) + length (4B LE).
    /// The address is an absolute flash offset; the crypto service reads
    /// the bytes from the flash service itself. A StartVerify while
    /// hashing is refused with WrongState: one verification at a time.
    StartVerify = 0,
    /// Query the current verify status. No args; the response payload
    /// carries a `VerifyStatus`.
    QueryStatus = 1,
}

impl CryptoOp {
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::StartVerify),
            1 => Some(Self::QueryStatus),
            _ => None,
        }
    }
}

// ============================================================================
// Verify status (response payload for QueryStatus)
// ============================================================================

/// What the crypto service reports when asked for status. A verdict
/// holds until the next StartVerify resets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyStatus {
    /// No verification in progress.
    Idle,
    /// Hashing is in progress. `hashed` and `total` are byte counts.
    Hashing { hashed: u32, total: u32 },
    /// Verification finished: firmware is authentic.
    Authenticated,
    /// Verification finished: firmware was rejected.
    Rejected,
}

impl VerifyStatus {
    pub const MAX_SIZE: usize = 9;

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        match self {
            Self::Idle => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = 0;
                Ok(1)
            }
            Self::Hashing { hashed, total } => {
                if buf.len() < 9 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = 1;
                buf[1..5].copy_from_slice(&hashed.to_le_bytes());
                buf[5..9].copy_from_slice(&total.to_le_bytes());
                Ok(9)
            }
            Self::Authenticated => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = 2;
                Ok(1)
            }
            Self::Rejected => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = 3;
                Ok(1)
            }
        }
    }

    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        if buf.is_empty() {
            return Err(WireError::Truncated);
        }
        match buf[0] {
            0 => Ok(Self::Idle),
            1 => {
                if buf.len() < 9 {
                    return Err(WireError::Truncated);
                }
                Ok(Self::Hashing {
                    hashed: u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]),
                    total: u32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]),
                })
            }
            2 => Ok(Self::Authenticated),
            3 => Ok(Self::Rejected),
            v => Err(WireError::InvalidValue(v)),
        }
    }
}

// ============================================================================
// Request header
// ============================================================================

/// Request header (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestHeader {
    pub op: u8,
    pub flags: u8,
}

impl RequestHeader {
    pub const SIZE: usize = 8;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        [self.op, self.flags, 0, 0, 0, 0, 0, 0]
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            op: bytes[0],
            flags: bytes[1],
        })
    }

    pub fn operation(&self) -> Option<CryptoOp> {
        CryptoOp::from_u8(self.op)
    }
}

// ============================================================================
// Response header
// ============================================================================

/// Response header (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseHeader {
    pub code: u8,
    pub flags: u8,
    pub payload_len: u16,
}

impl ResponseHeader {
    pub const SIZE: usize = 8;

    pub const fn success() -> Self {
        Self {
            code: ResponseCode::Success as u8,
            flags: 0,
            payload_len: 0,
        }
    }

    pub const fn error(code: ResponseCode) -> Self {
        Self {
            code: code as u8,
            flags: 0,
            payload_len: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.code == ResponseCode::Success as u8
    }

    pub fn response_code(&self) -> Result<ResponseCode, WireError> {
        ResponseCode::from_u8(self.code).ok_or(WireError::InvalidValue(self.code))
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let pl = self.payload_len.to_le_bytes();
        [self.code, self.flags, pl[0], pl[1], 0, 0, 0, 0]
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            code: bytes[0],
            flags: bytes[1],
            payload_len: u16::from_le_bytes([bytes[2], bytes[3]]),
        })
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Maximum status payload (Hashing: 9 bytes).
pub const MAX_PAYLOAD_SIZE: usize = VerifyStatus::MAX_SIZE;

/// Maximum total request size (header + StartVerify args).
pub const MAX_REQUEST_SIZE: usize = RequestHeader::SIZE + 8;

/// Maximum total response size (header + QueryStatus payload).
pub const MAX_RESPONSE_SIZE: usize = ResponseHeader::SIZE + MAX_PAYLOAD_SIZE;

// ============================================================================
// Request types
// ============================================================================

/// A flash region to verify: absolute address and byte count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyRegion {
    pub address: u32,
    pub length: u32,
}

// ============================================================================
// Request encoding (client side)
// ============================================================================

/// Encode StartVerify. The crypto service reads the region from the
/// flash service itself.
pub fn encode_start_verify(region: &VerifyRegion) -> [u8; MAX_REQUEST_SIZE] {
    let h = RequestHeader {
        op: CryptoOp::StartVerify as u8,
        flags: 0,
    };
    let mut buf = [0u8; MAX_REQUEST_SIZE];
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE..RequestHeader::SIZE + 4]
        .copy_from_slice(&region.address.to_le_bytes());
    buf[RequestHeader::SIZE + 4..MAX_REQUEST_SIZE].copy_from_slice(&region.length.to_le_bytes());
    buf
}

/// Encode QueryStatus. Header only, no args.
pub fn encode_query_status() -> [u8; RequestHeader::SIZE] {
    RequestHeader {
        op: CryptoOp::QueryStatus as u8,
        flags: 0,
    }
    .to_bytes()
}

// ============================================================================
// Response encoding (server side)
// ============================================================================

/// Encode a success response with no payload.
pub fn encode_success_response(buf: &mut [u8]) -> Result<usize, WireError> {
    if buf.len() < ResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    buf[..ResponseHeader::SIZE].copy_from_slice(&ResponseHeader::success().to_bytes());
    Ok(ResponseHeader::SIZE)
}

/// Encode an error response.
pub fn encode_error_response(buf: &mut [u8], code: ResponseCode) -> Result<usize, WireError> {
    if buf.len() < ResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    buf[..ResponseHeader::SIZE].copy_from_slice(&ResponseHeader::error(code).to_bytes());
    Ok(ResponseHeader::SIZE)
}

/// Encode a QueryStatus success response with the current verify status.
pub fn encode_status_response(buf: &mut [u8], status: &VerifyStatus) -> Result<usize, WireError> {
    let mut payload_buf = [0u8; VerifyStatus::MAX_SIZE];
    let payload_len = status.encode(&mut payload_buf)?;
    let total = ResponseHeader::SIZE + payload_len;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let mut h = ResponseHeader::success();
    h.payload_len = payload_len as u16;
    buf[..ResponseHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[ResponseHeader::SIZE..total].copy_from_slice(&payload_buf[..payload_len]);
    Ok(total)
}

// ============================================================================
// Response decoding (client side)
// ============================================================================

/// Decode a response header.
pub fn decode_response_header(buf: &[u8]) -> Result<ResponseHeader, WireError> {
    ResponseHeader::from_bytes(buf).ok_or(WireError::Truncated)
}

/// Extract the response payload bytes (after the header).
pub fn get_response_payload<'a>(
    buf: &'a [u8],
    header: &ResponseHeader,
) -> Result<&'a [u8], WireError> {
    let end = ResponseHeader::SIZE + header.payload_len as usize;
    if buf.len() < end {
        return Err(WireError::Truncated);
    }
    Ok(&buf[ResponseHeader::SIZE..end])
}

// ============================================================================
// Request decoding (server side)
// ============================================================================

/// Decode a request header.
pub fn decode_request_header(buf: &[u8]) -> Result<RequestHeader, WireError> {
    RequestHeader::from_bytes(buf).ok_or(WireError::Truncated)
}

/// Get the request args (bytes after the header).
pub fn get_request_args(buf: &[u8]) -> &[u8] {
    if buf.len() > RequestHeader::SIZE {
        &buf[RequestHeader::SIZE..]
    } else {
        &[]
    }
}

/// Extract the verify region from a StartVerify request's args.
pub fn get_start_verify_args(args: &[u8]) -> Result<VerifyRegion, WireError> {
    if args.len() < 8 {
        return Err(WireError::Truncated);
    }
    Ok(VerifyRegion {
        address: u32::from_le_bytes([args[0], args[1], args[2], args[3]]),
        length: u32::from_le_bytes([args[4], args[5], args[6], args[7]]),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_header_roundtrip() {
        let h = RequestHeader {
            op: CryptoOp::QueryStatus as u8,
            flags: 0,
        };
        let bytes = h.to_bytes();
        let decoded = RequestHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.operation(), Some(CryptoOp::QueryStatus));
    }

    #[test]
    fn response_header_roundtrip() {
        let h = ResponseHeader {
            code: ResponseCode::Success as u8,
            flags: 0,
            payload_len: 9,
        };
        let bytes = h.to_bytes();
        let decoded = ResponseHeader::from_bytes(&bytes).unwrap();
        assert!(decoded.is_success());
        assert_eq!(decoded.payload_len, 9);
    }

    #[test]
    fn start_verify_roundtrip() {
        let region = VerifyRegion {
            address: 0x2000_0000,
            length: 0x0008_0000,
        };
        let buf = encode_start_verify(&region);
        assert_eq!(buf.len(), 16);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(CryptoOp::StartVerify));
        let args = get_request_args(&buf);
        let decoded = get_start_verify_args(args).unwrap();
        assert_eq!(decoded, region);
    }

    #[test]
    fn query_status_roundtrip() {
        let buf = encode_query_status();
        assert_eq!(buf.len(), RequestHeader::SIZE);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(CryptoOp::QueryStatus));
    }

    #[test]
    fn verify_status_idle_roundtrip() {
        let status = VerifyStatus::Idle;
        let mut payload = [0u8; VerifyStatus::MAX_SIZE];
        let len = status.encode(&mut payload).unwrap();
        assert_eq!(len, 1);
        assert_eq!(VerifyStatus::decode(&payload[..len]).unwrap(), status);
    }

    #[test]
    fn verify_status_hashing_roundtrip() {
        let status = VerifyStatus::Hashing {
            hashed: 0x1000,
            total: 0x8000,
        };
        let mut payload = [0u8; VerifyStatus::MAX_SIZE];
        let len = status.encode(&mut payload).unwrap();
        assert_eq!(len, 9);
        assert_eq!(VerifyStatus::decode(&payload[..len]).unwrap(), status);
    }

    #[test]
    fn verify_status_authenticated_roundtrip() {
        let status = VerifyStatus::Authenticated;
        let mut payload = [0u8; VerifyStatus::MAX_SIZE];
        let len = status.encode(&mut payload).unwrap();
        assert_eq!(len, 1);
        assert_eq!(VerifyStatus::decode(&payload[..len]).unwrap(), status);
    }

    #[test]
    fn verify_status_rejected_roundtrip() {
        let status = VerifyStatus::Rejected;
        let mut payload = [0u8; VerifyStatus::MAX_SIZE];
        let len = status.encode(&mut payload).unwrap();
        assert_eq!(len, 1);
        assert_eq!(VerifyStatus::decode(&payload[..len]).unwrap(), status);
    }

    #[test]
    fn status_response_hashing_roundtrip() {
        let status = VerifyStatus::Hashing {
            hashed: 0x4000,
            total: 0x0010_0000,
        };
        let mut buf = [0u8; 32];
        let len = encode_status_response(&mut buf, &status).unwrap();
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
        assert_eq!(h.payload_len, 9);
        let payload = get_response_payload(&buf[..len], &h).unwrap();
        assert_eq!(VerifyStatus::decode(payload).unwrap(), status);
    }

    #[test]
    fn error_response_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_error_response(&mut buf, ResponseCode::WrongState).unwrap();
        assert_eq!(len, ResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(!h.is_success());
        assert_eq!(h.response_code().unwrap(), ResponseCode::WrongState);
    }

    #[test]
    fn success_response_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_success_response(&mut buf).unwrap();
        assert_eq!(len, ResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
        assert_eq!(h.payload_len, 0);
    }

    #[test]
    fn unknown_opcode() {
        assert_eq!(CryptoOp::from_u8(0xFF), None);
    }

    #[test]
    fn unknown_status_tag() {
        assert_eq!(
            VerifyStatus::decode(&[0xFF]),
            Err(WireError::InvalidValue(0xFF))
        );
    }

    #[test]
    fn decode_request_truncated() {
        assert_eq!(decode_request_header(&[0u8; 4]), Err(WireError::Truncated));
    }

    #[test]
    fn decode_response_truncated() {
        assert_eq!(decode_response_header(&[0u8; 4]), Err(WireError::Truncated));
    }

    #[test]
    fn get_response_payload_truncated() {
        let mut h = ResponseHeader::success();
        h.payload_len = 100;
        let mut buf = [0u8; 16];
        buf[..ResponseHeader::SIZE].copy_from_slice(&h.to_bytes());
        assert_eq!(
            get_response_payload(&buf[..ResponseHeader::SIZE], &h),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn response_buffer_too_small_errors() {
        let mut buf = [0u8; 4];
        assert_eq!(
            encode_success_response(&mut buf),
            Err(WireError::BufferTooSmall)
        );
        assert_eq!(
            encode_error_response(&mut buf, ResponseCode::InternalError),
            Err(WireError::BufferTooSmall)
        );
    }

    #[test]
    fn start_verify_args_truncated() {
        assert_eq!(get_start_verify_args(&[0, 0]), Err(WireError::Truncated));
    }

    #[test]
    fn status_decode_truncated() {
        assert_eq!(VerifyStatus::decode(&[]), Err(WireError::Truncated));
    }

    #[test]
    fn hashing_status_decode_truncated() {
        assert_eq!(VerifyStatus::decode(&[1, 0, 0]), Err(WireError::Truncated));
    }

    #[test]
    fn get_request_args_empty_for_header_only() {
        let buf = encode_query_status();
        assert_eq!(get_request_args(&buf), &[]);
    }
}
