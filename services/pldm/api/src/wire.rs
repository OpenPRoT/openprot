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

use crate::error::{RejectReason, ResponseCode, WireError};
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
    /// Tell the FD to run FdOps::verify.
    PerformVerify = 2,
    /// Tell the FD not to verify (e.g. isolated component).
    RejectVerify = 3,
    /// Tell the FD to run FdOps::apply.
    PerformApply = 4,
    /// Tell the FD not to apply.
    RejectApply = 5,
    /// Read the FD's current state (phase, result, error).
    QueryStatus = 6,
    /// Tell the FD to activate, ahead of the UA's request.
    PerformActivate = 7,
    /// Refuse activation, or take back a PerformActivate the FD has stored
    /// and the UA has not yet claimed. FD answers the UA with
    /// INCOMPLETE_UPDATE.
    RejectActivate = 8,
    /// Acknowledge cancel, release orchestrator-side resources.
    AckCancel = 9,
    /// Tell the FD the SVN floor is raised so it can answer the UA.
    PerformSvnCommit = 10,
    /// Tell the FD the floor did not move.
    RejectSvnCommit = 11,
}

impl PldmOp {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::AcceptOffer),
            1 => Some(Self::RejectOffer),
            2 => Some(Self::PerformVerify),
            3 => Some(Self::RejectVerify),
            4 => Some(Self::PerformApply),
            5 => Some(Self::RejectApply),
            6 => Some(Self::QueryStatus),
            7 => Some(Self::PerformActivate),
            8 => Some(Self::RejectActivate),
            9 => Some(Self::AckCancel),
            10 => Some(Self::PerformSvnCommit),
            11 => Some(Self::RejectSvnCommit),
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

/// How long a well-formed request is for a given operation. The decoder
/// reads a fixed header and ignores anything past it, so a frame longer
/// than expected would dispatch as if the extra bytes were not there.
pub const fn expected_request_len(op: PldmOp) -> usize {
    match op {
        PldmOp::AcceptOffer => RequestHeader::SIZE + 4,
        PldmOp::RejectVerify
        | PldmOp::RejectApply
        | PldmOp::RejectActivate
        | PldmOp::RejectSvnCommit => RequestHeader::SIZE + 1,
        PldmOp::RejectOffer
        | PldmOp::PerformVerify
        | PldmOp::PerformApply
        | PldmOp::QueryStatus
        | PldmOp::PerformActivate
        | PldmOp::AckCancel
        | PldmOp::PerformSvnCommit => RequestHeader::SIZE,
    }
}

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

pub fn encode_perform_verify(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::PerformVerify)
}

/// Encode RejectVerify with the reason the orchestrator is blocking.
pub fn encode_reject_verify(buf: &mut [u8], reason: RejectReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::RejectVerify as u8,
        flags: 0,
        generation: 0,
    };
    buf[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
    buf[RequestHeader::SIZE] = reason as u8;
    Ok(total)
}

pub fn encode_perform_apply(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::PerformApply)
}

/// Encode RejectApply with the reason the orchestrator is blocking.
pub fn encode_reject_apply(buf: &mut [u8], reason: RejectReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::RejectApply as u8,
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

pub fn encode_perform_activate(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::PerformActivate)
}

pub fn encode_reject_activate(buf: &mut [u8], reason: RejectReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::RejectActivate as u8,
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

pub fn encode_perform_svn_commit(buf: &mut [u8]) -> Result<usize, WireError> {
    encode_header_only(buf, PldmOp::PerformSvnCommit)
}

/// Encode RejectSvnCommit with the reason the orchestrator is blocking.
pub fn encode_reject_svn_commit(buf: &mut [u8], reason: RejectReason) -> Result<usize, WireError> {
    let total = RequestHeader::SIZE + 1;
    if buf.len() < total {
        return Err(WireError::BufferTooSmall);
    }
    let h = RequestHeader {
        op: PldmOp::RejectSvnCommit as u8,
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

/// Extract the reject reason from a Reject* request's args.
pub fn get_reject_reason(args: &[u8]) -> Result<RejectReason, WireError> {
    if args.is_empty() {
        return Err(WireError::Truncated);
    }
    RejectReason::from_u8(args[0]).ok_or(WireError::InvalidValue(args[0]))
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
    fn encode_reject_verify_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_reject_verify(&mut buf, RejectReason::Isolated).unwrap();
        assert_eq!(len, 9);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::RejectVerify));
        let args = get_request_args(&buf[..len]);
        assert_eq!(get_reject_reason(args).unwrap(), RejectReason::Isolated);
    }

    #[test]
    fn encode_reject_apply_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_reject_apply(&mut buf, RejectReason::PolicyViolation).unwrap();
        let args = get_request_args(&buf[..len]);
        assert_eq!(
            get_reject_reason(args).unwrap(),
            RejectReason::PolicyViolation
        );
    }

    #[test]
    fn encode_reject_svn_commit_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_reject_svn_commit(&mut buf, RejectReason::PolicyViolation).unwrap();
        assert_eq!(len, 9);
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::RejectSvnCommit));
        let args = get_request_args(&buf[..len]);
        assert_eq!(
            get_reject_reason(args).unwrap(),
            RejectReason::PolicyViolation
        );
    }

    #[test]
    fn encode_reject_activate_roundtrip() {
        let mut buf = [0u8; 16];
        let len = encode_reject_activate(&mut buf, RejectReason::UnknownTarget).unwrap();
        let h = decode_request_header(&buf).unwrap();
        assert_eq!(h.operation(), Some(PldmOp::RejectActivate));
        let args = get_request_args(&buf[..len]);
        assert_eq!(
            get_reject_reason(args).unwrap(),
            RejectReason::UnknownTarget
        );
    }

    #[test]
    fn header_only_ops_roundtrip() {
        let ops = [
            (
                encode_reject_offer as fn(&mut [u8]) -> _,
                PldmOp::RejectOffer,
            ),
            (encode_perform_verify, PldmOp::PerformVerify),
            (encode_perform_apply, PldmOp::PerformApply),
            (encode_query_status, PldmOp::QueryStatus),
            (encode_perform_activate, PldmOp::PerformActivate),
            (encode_ack_cancel, PldmOp::AckCancel),
            (encode_perform_svn_commit, PldmOp::PerformSvnCommit),
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
            encode_reject_verify(&mut buf, RejectReason::Busy),
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
        assert_eq!(get_reject_reason(&[]), Err(WireError::Truncated));
    }
}
