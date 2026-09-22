// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM IPC wire protocol.
//!
//! Binary wire protocol for orchestrator-to-FD operations over IPC
//! channels. Uses manual byte encoding for `no_std` compatibility.
//!
//! ```text
//! Request (8-byte header + optional args):
//! +----+-------+-----+----------+
//! | op | flags | gen | reserved |  + [args]
//! | 1B |  1B   | 2B  |    4B    |
//! +----+-------+-----+----------+
//!
//! Response (8-byte header + optional payload):
//! +------+-------+-----+-------------+----------+
//! | code | flags | gen | payload_len | reserved |  + [payload]
//! |  1B  |  1B   | 2B  |    2B LE    |    2B    |
//! +------+-------+-----+-------------+----------+
//! ```

use crate::error::{DenyReason, ResponseCode, WireError};
use crate::status::FdStatus;

// ============================================================================
// Operation codes
// ============================================================================

/// Orchestrator-to-FD IPC operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PldmOp {
    /// Approve the pending offer, provide staging flash base address.
    AcceptOffer = 0,
    /// Reject the pending offer, FD sends TransferComplete.
    RejectOffer = 1,
    /// Authorize the FD to run FdOps::verify.
    GrantVerify = 2,
    /// Block verify (e.g. isolated component).
    DenyVerify = 3,
    /// Authorize the FD to run FdOps::apply.
    GrantApply = 4,
    /// Block apply.
    DenyApply = 5,
    /// Read the FD's current state (phase, result, error).
    QueryStatus = 6,
    /// Authorize activation ahead of the UA's request.
    GrantActivate = 7,
    /// Refuse activation; FD answers the UA with INCOMPLETE_UPDATE.
    DenyActivate = 8,
    /// Acknowledge cancel, release orchestrator-side resources.
    AckCancel = 9,
    /// Tell the FD the SVN floor is raised so it can answer the UA.
    GrantSvnCommit = 10,
    /// Block the SVN commit.
    DenySvnCommit = 11,
}

impl PldmOp {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::AcceptOffer),
            1 => Some(Self::RejectOffer),
            2 => Some(Self::GrantVerify),
            3 => Some(Self::DenyVerify),
            4 => Some(Self::GrantApply),
            5 => Some(Self::DenyApply),
            6 => Some(Self::QueryStatus),
            7 => Some(Self::GrantActivate),
            8 => Some(Self::DenyActivate),
            9 => Some(Self::AckCancel),
            10 => Some(Self::GrantSvnCommit),
            11 => Some(Self::DenySvnCommit),
            _ => None,
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
    /// Phase generation. Reserved, set to 0.
    pub generation: u16,
}

impl RequestHeader {
    pub const SIZE: usize = 8;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let g = self.generation.to_le_bytes();
        [self.op, self.flags, g[0], g[1], 0, 0, 0, 0]
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            op: bytes[0],
            flags: bytes[1],
            generation: u16::from_le_bytes([bytes[2], bytes[3]]),
        })
    }

    pub fn operation(&self) -> Option<PldmOp> {
        PldmOp::from_u8(self.op)
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
    /// Phase generation. Reserved, set to 0.
    pub generation: u16,
    pub payload_len: u16,
}

impl ResponseHeader {
    pub const SIZE: usize = 8;

    pub const fn success() -> Self {
        Self {
            code: ResponseCode::Success as u8,
            flags: 0,
            generation: 0,
            payload_len: 0,
        }
    }

    pub const fn error(code: ResponseCode) -> Self {
        Self {
            code: code as u8,
            flags: 0,
            generation: 0,
            payload_len: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.code == ResponseCode::Success as u8
    }

    pub fn response_code(&self) -> ResponseCode {
        ResponseCode::from_u8(self.code).unwrap_or(ResponseCode::InternalError)
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let g = self.generation.to_le_bytes();
        let pl = self.payload_len.to_le_bytes();
        [self.code, self.flags, g[0], g[1], pl[0], pl[1], 0, 0]
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            code: bytes[0],
            flags: bytes[1],
            generation: u16::from_le_bytes([bytes[2], bytes[3]]),
            payload_len: u16::from_le_bytes([bytes[4], bytes[5]]),
        })
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Maximum status payload (OfferPending: 9 bytes).
pub const MAX_PAYLOAD_SIZE: usize = FdStatus::MAX_SIZE;

/// Maximum total request size (header + AcceptOffer args).
pub const MAX_REQUEST_SIZE: usize = RequestHeader::SIZE + 4;

/// Maximum total response size (header + QueryStatus payload).
pub const MAX_RESPONSE_SIZE: usize = ResponseHeader::SIZE + MAX_PAYLOAD_SIZE;

// ============================================================================
// Request encoding
// ============================================================================

fn encode_header_only(buf: &mut [u8], op: PldmOp) -> Result<usize, WireError> {
    if buf.len() < RequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: op as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    Ok(RequestHeader::SIZE)
}

/// Encode AcceptOffer. `staging_base` is the flash address the FD
/// should stage firmware to.
pub fn encode_accept_offer(buf: &mut [u8], staging_base: u32) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 4;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::AcceptOffer as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE..total].copy_from_slice(&staging_base.to_le_bytes());
    Ok(total)
}

pub fn encode_reject_offer(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::RejectOffer)
}

pub fn encode_grant_verify(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::GrantVerify)
}

/// Encode DenyVerify with the reason the orchestrator is blocking.
pub fn encode_deny_verify(buf: &mut [u8], reason: DenyReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::DenyVerify as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE] = reason as u8;
    Ok(total)
}

pub fn encode_grant_apply(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::GrantApply)
}

/// Encode DenyApply with the reason the orchestrator is blocking.
pub fn encode_deny_apply(buf: &mut [u8], reason: DenyReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::DenyApply as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE] = reason as u8;
    Ok(total)
}

pub fn encode_query_status(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::QueryStatus)
}

pub fn encode_grant_activate(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::GrantActivate)
}

pub fn encode_deny_activate(buf: &mut [u8], reason: DenyReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::DenyActivate as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE] = reason as u8;
    Ok(total)
}

pub fn encode_ack_cancel(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::AckCancel)
}

pub fn encode_grant_svn_commit(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::GrantSvnCommit)
}

/// Encode DenySvnCommit with the reason the orchestrator is blocking.
pub fn encode_deny_svn_commit(buf: &mut [u8], reason: DenyReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::DenySvnCommit as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE] = reason as u8;
    Ok(total)
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

/// Encode a QueryStatus success response with the FD's current status.
pub fn encode_status_response(buf: &mut [u8], status: &FdStatus) -> Result<usize, WireError> {
    let mut payload_buf = [0u8; FdStatus::MAX_SIZE];
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

/// Extract the staging base address from an AcceptOffer request's args.
pub fn get_accept_offer_base(args: &[u8]) -> Result<u32, WireError> {
    if args.len() < 4 {
        return Err(WireError::Truncated);
    }
    Ok(u32::from_le_bytes([args[0], args[1], args[2], args[3]]))
}

/// Extract the deny reason from a Deny* request's args.
pub fn get_deny_reason(args: &[u8]) -> Result<DenyReason, WireError> {
    if args.is_empty() {
        return Err(WireError::Truncated);
    }
    DenyReason::from_u8(args[0]).ok_or(WireError::InvalidValue(args[0]))
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
            op: PldmOp::QueryStatus as u8,
            flags: 0,
            generation: 0x1234,
        };
        let bytes = h.to_bytes();
        let decoded = RequestHeader::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.op, PldmOp::QueryStatus as u8);
        assert_eq!(decoded.generation, 0x1234);
    }

    #[test]
    fn response_header_roundtrip() {
        let h = ResponseHeader {
            code: ResponseCode::Success as u8,
            flags: 0,
            generation: 0,
            payload_len: 8,
        };
        let bytes = h.to_bytes();
        let decoded = ResponseHeader::from_bytes(&bytes).unwrap();
        assert!(decoded.is_success());
        assert_eq!(decoded.payload_len, 8);
    }

    #[test]
    fn encode_accept_offer_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_accept_offer(&mut buf, 0x2000_0000).unwrap();
        assert_eq!(len, 12);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::AcceptOffer));
        let args = get_request_args(&buf[..len]);
        assert_eq!(get_accept_offer_base(args).unwrap(), 0x2000_0000);
    }

    #[test]
    fn encode_deny_verify_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_deny_verify(&mut buf, DenyReason::Isolated).unwrap();
        assert_eq!(len, 9);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::DenyVerify));
        let args = get_request_args(&buf[..len]);
        assert_eq!(get_deny_reason(args).unwrap(), DenyReason::Isolated);
    }

    #[test]
    fn encode_deny_apply_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_deny_apply(&mut buf, DenyReason::PolicyViolation).unwrap();
        let args = get_request_args(&buf[..len]);
        assert_eq!(get_deny_reason(args).unwrap(), DenyReason::PolicyViolation);
    }

    #[test]
    fn encode_deny_svn_commit_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_deny_svn_commit(&mut buf, DenyReason::PolicyViolation).unwrap();
        assert_eq!(len, 9);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::DenySvnCommit));
        let args = get_request_args(&buf[..len]);
        assert_eq!(get_deny_reason(args).unwrap(), DenyReason::PolicyViolation);
    }

    #[test]
    fn encode_deny_activate_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_deny_activate(&mut buf, DenyReason::UnknownTarget).unwrap();
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::DenyActivate));
        let args = get_request_args(&buf[..len]);
        assert_eq!(get_deny_reason(args).unwrap(), DenyReason::UnknownTarget);
    }

    #[test]
    fn header_only_ops_roundtrip() {
        let ops = [
            (
                encode_reject_offer as fn(&mut [u8]) -> _,
                PldmOp::RejectOffer,
            ),
            (encode_grant_verify, PldmOp::GrantVerify),
            (encode_grant_apply, PldmOp::GrantApply),
            (encode_query_status, PldmOp::QueryStatus),
            (encode_grant_activate, PldmOp::GrantActivate),
            (encode_ack_cancel, PldmOp::AckCancel),
            (encode_grant_svn_commit, PldmOp::GrantSvnCommit),
        ];
        for (encode_fn, expected_op) in ops {
            let mut buf = [0u8; 16];
            let len = encode_fn(&mut buf).unwrap();
            assert_eq!(len, RequestHeader::SIZE);
            let h = decode_request_header(&buf).unwrap();
            assert_eq!(h.operation(), Some(expected_op));
        }
    }

    #[test]
    fn status_response_roundtrip() {
        let status = FdStatus::OfferPending {
            target: 0x0001,
            total: 0x0008_0000,
            mode: crate::status::TransferMode::InTransport,
            svn_delayed: true,
        };
        let mut buf = [0u8; 32];
        let len = encode_status_response(&mut buf, &status).unwrap();
        let h = decode_response_header(&buf).unwrap();
        assert!(h.is_success());
        assert_eq!(h.payload_len, 9);
        let payload = get_response_payload(&buf[..len], &h).unwrap();
        let decoded = FdStatus::decode(payload).unwrap();
        assert_eq!(decoded, status);
    }

    #[test]
    fn error_response_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_error_response(&mut buf, ResponseCode::WrongPhase).unwrap();
        assert_eq!(len, ResponseHeader::SIZE);
        let h = decode_response_header(&buf).unwrap();
        assert!(!h.is_success());
        assert_eq!(h.response_code(), ResponseCode::WrongPhase);
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
        assert_eq!(PldmOp::from_u8(0xFF), None);
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
    fn buffer_too_small_errors() {
        let mut buf = [0u8; 4];
        assert_eq!(
            encode_accept_offer(&mut buf, 0),
            Err(WireError::BufferTooSmall)
        );
        assert_eq!(
            encode_reject_offer(&mut buf),
            Err(WireError::BufferTooSmall)
        );
        assert_eq!(
            encode_deny_verify(&mut buf, DenyReason::Busy),
            Err(WireError::BufferTooSmall)
        );
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
    fn get_request_args_empty_for_header_only() {
        let mut buf = [0u8; 16];
        encode_query_status(&mut buf).unwrap();
        assert_eq!(get_request_args(&buf[..RequestHeader::SIZE]), &[]);
    }

    #[test]
    fn get_accept_offer_base_truncated() {
        assert_eq!(get_accept_offer_base(&[0, 0]), Err(WireError::Truncated));
    }

    #[test]
    fn get_deny_reason_truncated() {
        assert_eq!(get_deny_reason(&[]), Err(WireError::Truncated));
    }
}
