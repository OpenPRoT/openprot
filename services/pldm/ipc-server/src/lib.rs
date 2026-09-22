// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM IPC server: dispatch and loopback.
//!
//! The FD side of the IPC channel. Decodes orchestrator requests,
//! dispatches to an `FdHandler` implementation, and encodes responses.
//! The loopback transport calls dispatch directly in-process so the
//! same encode/decode paths exercise real dispatch with no kernel.

#![no_std]

mod loopback;

pub use loopback::LoopbackTransport;
use pldm_ipc_api::wire::{self, PldmOp};
use pldm_ipc_api::{DenyReason, FdStatus, ResponseCode, WireError};

/// What the FD does in response to each orchestrator operation.
///
/// One method per opcode. All return `Result<(), ResponseCode>` except
/// `query_status`, which returns the FD's current condition. The server
/// encodes the result into the response buffer; the handler never
/// touches wire bytes.
pub trait FdHandler {
    fn accept_offer(&mut self, staging_base: u32) -> Result<(), ResponseCode>;
    fn reject_offer(&mut self) -> Result<(), ResponseCode>;
    fn grant_verify(&mut self) -> Result<(), ResponseCode>;
    fn deny_verify(&mut self, reason: DenyReason) -> Result<(), ResponseCode>;
    fn grant_apply(&mut self) -> Result<(), ResponseCode>;
    fn deny_apply(&mut self, reason: DenyReason) -> Result<(), ResponseCode>;
    fn query_status(&mut self) -> Result<FdStatus, ResponseCode>;
    fn grant_activate(&mut self) -> Result<(), ResponseCode>;
    fn deny_activate(&mut self, reason: DenyReason) -> Result<(), ResponseCode>;
    fn ack_cancel(&mut self) -> Result<(), ResponseCode>;
    fn grant_svn_commit(&mut self) -> Result<(), ResponseCode>;
    fn deny_svn_commit(&mut self, reason: DenyReason) -> Result<(), ResponseCode>;
}

/// Decode one request, call the handler, encode the response.
///
/// Returns the number of bytes written to `response`. On any wire
/// error the response is an `InternalError`; on an unknown opcode it
/// is `InvalidOp`. The caller does not need to inspect the return
/// value beyond passing `&response[..n]` to the transport.
pub fn dispatch<F: FdHandler>(handler: &mut F, request: &[u8], response: &mut [u8]) -> usize {
    match dispatch_inner(handler, request, response) {
        Ok(n) => n,
        Err(WireError::InvalidOpcode(_)) => {
            wire::encode_error_response(response, ResponseCode::InvalidOp).unwrap_or(0)
        }
        Err(_) => wire::encode_error_response(response, ResponseCode::InternalError).unwrap_or(0),
    }
}

fn dispatch_inner<F: FdHandler>(
    handler: &mut F,
    request: &[u8],
    response: &mut [u8],
) -> Result<usize, WireError> {
    let header = wire::decode_request_header(request)?;
    let args = wire::get_request_args(request);

    let op = header
        .operation()
        .ok_or(WireError::InvalidOpcode(header.op))?;

    match op {
        PldmOp::AcceptOffer => {
            let base = wire::get_accept_offer_base(args)?;
            encode_unit_result(response, handler.accept_offer(base))
        }
        PldmOp::RejectOffer => encode_unit_result(response, handler.reject_offer()),
        PldmOp::GrantVerify => encode_unit_result(response, handler.grant_verify()),
        PldmOp::DenyVerify => {
            let reason = wire::get_deny_reason(args)?;
            encode_unit_result(response, handler.deny_verify(reason))
        }
        PldmOp::GrantApply => encode_unit_result(response, handler.grant_apply()),
        PldmOp::DenyApply => {
            let reason = wire::get_deny_reason(args)?;
            encode_unit_result(response, handler.deny_apply(reason))
        }
        PldmOp::QueryStatus => match handler.query_status() {
            Ok(status) => wire::encode_status_response(response, &status),
            Err(code) => wire::encode_error_response(response, code),
        },
        PldmOp::GrantActivate => encode_unit_result(response, handler.grant_activate()),
        PldmOp::DenyActivate => {
            let reason = wire::get_deny_reason(args)?;
            encode_unit_result(response, handler.deny_activate(reason))
        }
        PldmOp::AckCancel => encode_unit_result(response, handler.ack_cancel()),
        PldmOp::GrantSvnCommit => encode_unit_result(response, handler.grant_svn_commit()),
        PldmOp::DenySvnCommit => {
            let reason = wire::get_deny_reason(args)?;
            encode_unit_result(response, handler.deny_svn_commit(reason))
        }
    }
}

fn encode_unit_result(
    response: &mut [u8],
    result: Result<(), ResponseCode>,
) -> Result<usize, WireError> {
    match result {
        Ok(()) => wire::encode_success_response(response),
        Err(code) => wire::encode_error_response(response, code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pldm_ipc_api::status::TransferMode;
    use pldm_ipc_api::wire::{RequestHeader, MAX_RESPONSE_SIZE};

    /// Mock handler that records calls and returns canned responses.
    struct MockFd {
        last_op: Option<&'static str>,
        status: FdStatus,
        next_error: Option<ResponseCode>,
    }

    impl MockFd {
        fn new() -> Self {
            Self {
                last_op: None,
                status: FdStatus::Idle { reason: 0 },
                next_error: None,
            }
        }

        fn returning_error(code: ResponseCode) -> Self {
            Self {
                last_op: None,
                status: FdStatus::Idle { reason: 0 },
                next_error: Some(code),
            }
        }

        fn check(&mut self, name: &'static str) -> Result<(), ResponseCode> {
            self.last_op = Some(name);
            match self.next_error.take() {
                Some(code) => Err(code),
                None => Ok(()),
            }
        }
    }

    impl FdHandler for MockFd {
        fn accept_offer(&mut self, _base: u32) -> Result<(), ResponseCode> {
            self.check("accept_offer")
        }
        fn reject_offer(&mut self) -> Result<(), ResponseCode> {
            self.check("reject_offer")
        }
        fn grant_verify(&mut self) -> Result<(), ResponseCode> {
            self.check("grant_verify")
        }
        fn deny_verify(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            self.check("deny_verify")
        }
        fn grant_apply(&mut self) -> Result<(), ResponseCode> {
            self.check("grant_apply")
        }
        fn deny_apply(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            self.check("deny_apply")
        }
        fn query_status(&mut self) -> Result<FdStatus, ResponseCode> {
            self.last_op = Some("query_status");
            match self.next_error.take() {
                Some(code) => Err(code),
                None => Ok(self.status),
            }
        }
        fn grant_activate(&mut self) -> Result<(), ResponseCode> {
            self.check("grant_activate")
        }
        fn deny_activate(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            self.check("deny_activate")
        }
        fn ack_cancel(&mut self) -> Result<(), ResponseCode> {
            self.check("ack_cancel")
        }
        fn grant_svn_commit(&mut self) -> Result<(), ResponseCode> {
            self.check("grant_svn_commit")
        }
        fn deny_svn_commit(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            self.check("deny_svn_commit")
        }
    }

    fn roundtrip_success(
        encode: impl FnOnce(&mut [u8]) -> Result<usize, WireError>,
        expected_op: &'static str,
    ) {
        let mut req = [0u8; 16];
        let req_len = encode(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req[..req_len], &mut resp);
        assert_eq!(fd.last_op, Some(expected_op));
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success(), "expected success for {expected_op}");
    }

    #[test]
    fn accept_offer_dispatches() {
        roundtrip_success(
            |buf| wire::encode_accept_offer(buf, 0x2000_0000),
            "accept_offer",
        );
    }

    #[test]
    fn reject_offer_dispatches() {
        roundtrip_success(|buf| wire::encode_reject_offer(buf), "reject_offer");
    }

    #[test]
    fn grant_verify_dispatches() {
        roundtrip_success(|buf| wire::encode_grant_verify(buf), "grant_verify");
    }

    #[test]
    fn deny_verify_dispatches() {
        roundtrip_success(
            |buf| wire::encode_deny_verify(buf, DenyReason::Isolated),
            "deny_verify",
        );
    }

    #[test]
    fn grant_apply_dispatches() {
        roundtrip_success(|buf| wire::encode_grant_apply(buf), "grant_apply");
    }

    #[test]
    fn deny_apply_dispatches() {
        roundtrip_success(
            |buf| wire::encode_deny_apply(buf, DenyReason::PolicyViolation),
            "deny_apply",
        );
    }

    #[test]
    fn grant_activate_dispatches() {
        roundtrip_success(|buf| wire::encode_grant_activate(buf), "grant_activate");
    }

    #[test]
    fn deny_activate_dispatches() {
        roundtrip_success(
            |buf| wire::encode_deny_activate(buf, DenyReason::Busy),
            "deny_activate",
        );
    }

    #[test]
    fn ack_cancel_dispatches() {
        roundtrip_success(|buf| wire::encode_ack_cancel(buf), "ack_cancel");
    }

    #[test]
    fn grant_svn_commit_dispatches() {
        roundtrip_success(|buf| wire::encode_grant_svn_commit(buf), "grant_svn_commit");
    }

    #[test]
    fn deny_svn_commit_dispatches() {
        roundtrip_success(
            |buf| wire::encode_deny_svn_commit(buf, DenyReason::PolicyViolation),
            "deny_svn_commit",
        );
    }

    #[test]
    fn query_status_returns_fd_state() {
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        fd.status = FdStatus::OfferPending {
            target: 0x0001,
            total: 0x0010_0000,
            mode: TransferMode::InTransport,
            svn_delayed: true,
        };
        let resp_len = dispatch(&mut fd, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success());
        let payload = wire::get_response_payload(&resp[..resp_len], &h).unwrap();
        let status = FdStatus::decode(payload).unwrap();
        assert_eq!(status, fd.status);
    }

    #[test]
    fn handler_error_becomes_error_response() {
        let mut req = [0u8; 16];
        let req_len = wire::encode_grant_verify(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::returning_error(ResponseCode::WrongPhase);
        let resp_len = dispatch(&mut fd, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(!h.is_success());
        assert_eq!(h.response_code(), ResponseCode::WrongPhase);
    }

    #[test]
    fn unknown_opcode_returns_invalid_op() {
        let mut req = [0u8; 16];
        let h = RequestHeader {
            op: 0xFF,
            flags: 0,
            generation: 0,
        };
        req[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE], &mut resp);
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(!rh.is_success());
        assert_eq!(rh.response_code(), ResponseCode::InvalidOp);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn truncated_request_returns_internal_error() {
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &[0u8; 2], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(h.response_code(), ResponseCode::InternalError);
    }

    #[test]
    fn deny_verify_missing_reason_returns_internal_error() {
        let mut req = [0u8; 16];
        let h = RequestHeader {
            op: PldmOp::DenyVerify as u8,
            flags: 0,
            generation: 0,
        };
        req[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE], &mut resp);
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(rh.response_code(), ResponseCode::InternalError);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn deny_verify_bad_reason_returns_internal_error() {
        let mut req = [0u8; 16];
        let h = RequestHeader {
            op: PldmOp::DenyVerify as u8,
            flags: 0,
            generation: 0,
        };
        req[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
        req[RequestHeader::SIZE] = 0xFF;
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE + 1], &mut resp);
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(rh.response_code(), ResponseCode::InternalError);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn accept_offer_truncated_args_returns_internal_error() {
        let mut req = [0u8; 16];
        let h = RequestHeader {
            op: PldmOp::AcceptOffer as u8,
            flags: 0,
            generation: 0,
        };
        req[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
        // Only 1 byte of args instead of the 4 needed for the base address.
        req[RequestHeader::SIZE] = 0x20;
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE + 1], &mut resp);
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(rh.response_code(), ResponseCode::InternalError);
        assert_eq!(fd.last_op, None);
    }
}
