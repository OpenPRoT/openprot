// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM IPC server: the FD side of the IPC channel.
//!
//! Decodes orchestrator requests, dispatches to an `FdHandler`
//! implementation, and encodes responses. `FdServer` is the
//! `util_service::Dispatch` impl, so the same server answers behind a
//! kernel channel in production and inside `util_service::Loopback` in
//! host tests.

#![no_std]

use pldm_ipc_api::wire::{self, PldmOp};
use pldm_ipc_api::{DenyReason, FdStatus, ResponseCode, WireError};
use util_service::{Dispatch, DispatchError};

/// What the FD does in response to each orchestrator operation.
///
/// One method per opcode. All return `Result<(), ResponseCode>` except
/// `query_status`, which returns the FD's current condition. The server
/// encodes the result into the response buffer; the handler never
/// touches wire bytes.
///
/// Every method must return immediately. Record the decision, flip
/// state, return. The FD shares its loop with MCTP traffic from the
/// UA, so a handler that blocks stalls UA traffic.
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
/// Returns the number of bytes written to `response`. An unknown opcode
/// answers `InvalidOp` and a request frame that does not decode answers
/// `MalformedRequest`, so a caller error is never reported as a fault in
/// the FD. `InternalError` is left for the one server-side case: the
/// response does not fit the buffer the caller gave, but an error frame
/// does. A `response` too small for even that error frame has nothing to
/// send back.
pub fn dispatch<F: FdHandler>(
    handler: &mut F,
    request: &[u8],
    response: &mut [u8],
) -> Result<usize, DispatchError> {
    let code = match dispatch_inner(handler, request, response) {
        Ok(n) => return Ok(n),
        Err(WireError::InvalidOpcode(_)) => ResponseCode::InvalidOp,
        Err(WireError::Truncated | WireError::PayloadTooLarge | WireError::InvalidValue(_)) => {
            ResponseCode::MalformedRequest
        }
        Err(WireError::BufferTooSmall) => ResponseCode::InternalError,
    };
    wire::encode_error_response(response, code).map_err(|_| DispatchError::ResponseTooLarge)
}

/// An `FdHandler` as a server the shared transports can drive.
///
/// The newtype exists because `Dispatch` is a foreign trait: it cannot
/// be implemented for every `F: FdHandler` directly.
pub struct FdServer<F> {
    handler: F,
}

impl<F> FdServer<F> {
    pub const fn new(handler: F) -> Self {
        Self { handler }
    }

    /// The handler, to assert on what the requests did to it.
    pub fn handler(&self) -> &F {
        &self.handler
    }
}

impl<F: FdHandler> Dispatch for FdServer<F> {
    fn dispatch(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, DispatchError> {
        dispatch(&mut self.handler, request, response)
    }
}

fn dispatch_inner<F: FdHandler>(
    handler: &mut F,
    request: &[u8],
    response: &mut [u8],
) -> Result<usize, WireError> {
    let header = wire::decode_request_header(request)?;
    let op = header
        .operation()
        .ok_or(WireError::InvalidOpcode(header.op))?;
    if request.len() > wire::expected_request_len(op) {
        return Err(WireError::PayloadTooLarge);
    }
    let args = wire::get_request_args(request);

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
        let resp_len = dispatch(&mut fd, &req[..req_len], &mut resp).unwrap();
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
        let resp_len = dispatch(&mut fd, &req[..req_len], &mut resp).unwrap();
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
        let resp_len = dispatch(&mut fd, &req[..req_len], &mut resp).unwrap();
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
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE], &mut resp).unwrap();
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(!rh.is_success());
        assert_eq!(rh.response_code(), ResponseCode::InvalidOp);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn truncated_request_returns_malformed_request() {
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &[0u8; 2], &mut resp).unwrap();
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(h.response_code(), ResponseCode::MalformedRequest);
    }

    #[test]
    fn deny_verify_missing_reason_returns_malformed_request() {
        let mut req = [0u8; 16];
        let h = RequestHeader {
            op: PldmOp::DenyVerify as u8,
            flags: 0,
            generation: 0,
        };
        req[..RequestHeader::SIZE].copy_from_slice(&h.to_bytes());
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE], &mut resp).unwrap();
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(rh.response_code(), ResponseCode::MalformedRequest);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn deny_verify_bad_reason_returns_malformed_request() {
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
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE + 1], &mut resp).unwrap();
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(rh.response_code(), ResponseCode::MalformedRequest);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn accept_offer_truncated_args_returns_malformed_request() {
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
        let resp_len = dispatch(&mut fd, &req[..RequestHeader::SIZE + 1], &mut resp).unwrap();
        let rh = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert_eq!(rh.response_code(), ResponseCode::MalformedRequest);
        assert_eq!(fd.last_op, None);
    }

    #[test]
    fn a_query_status_frame_with_trailing_slack_is_rejected() {
        // 12 bytes is MAX_REQUEST_SIZE, so the old length gate let this
        // through and the decoder ignored the four extra bytes.
        let mut req = [0u8; wire::MAX_REQUEST_SIZE];
        let req_len = wire::encode_query_status(&mut req).unwrap();
        assert!(req_len < wire::MAX_REQUEST_SIZE);

        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let mut fd = MockFd::new();
        let resp_len = dispatch(&mut fd, &req, &mut resp).unwrap();

        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(!h.is_success());
        assert_eq!(h.response_code(), ResponseCode::MalformedRequest);
        assert_eq!(fd.last_op, None);
    }
}

#[cfg(test)]
mod loopback_tests {
    use super::*;
    use pldm_ipc_api::wire::MAX_REQUEST_SIZE;
    use util_service::{AsyncTransport, Loopback, TransportError};

    /// Start one request and poll the response out, as the client layer
    /// does; the loopback always has it ready on the first poll.
    fn round_trip<F: FdHandler>(
        transport: &mut Loopback<FdServer<F>, MAX_RESPONSE_SIZE>,
        req: &[u8],
        resp: &mut [u8],
    ) -> usize {
        transport.start(req).unwrap();
        transport.poll(resp).unwrap().unwrap()
    }
    use pldm_ipc_api::status::TransferMode;
    use pldm_ipc_api::wire::{self, MAX_RESPONSE_SIZE};
    use pldm_ipc_api::{DenyReason, FdStatus, ResponseCode};

    /// Minimal handler for loopback tests.
    struct StubFd {
        status: FdStatus,
    }

    impl StubFd {
        fn idle() -> Self {
            Self {
                status: FdStatus::Idle { reason: 0 },
            }
        }

        fn with_offer() -> Self {
            Self {
                status: FdStatus::OfferPending {
                    target: 0x0001,
                    total: 0x0010_0000,
                    mode: TransferMode::InTransport,
                    svn_delayed: false,
                },
            }
        }

        fn at(status: FdStatus) -> Self {
            Self { status }
        }
    }

    impl FdHandler for StubFd {
        fn accept_offer(&mut self, _base: u32) -> Result<(), ResponseCode> {
            self.status = FdStatus::ReadyXfer;
            Ok(())
        }
        fn reject_offer(&mut self) -> Result<(), ResponseCode> {
            self.status = FdStatus::Idle { reason: 0 };
            Ok(())
        }
        fn grant_verify(&mut self) -> Result<(), ResponseCode> {
            self.status = FdStatus::ApplyPending;
            Ok(())
        }
        fn deny_verify(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn grant_apply(&mut self) -> Result<(), ResponseCode> {
            self.status = FdStatus::ActivationPending;
            Ok(())
        }
        fn deny_apply(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn query_status(&mut self) -> Result<FdStatus, ResponseCode> {
            Ok(self.status)
        }
        fn grant_activate(&mut self) -> Result<(), ResponseCode> {
            self.status = FdStatus::Idle { reason: 0 };
            Ok(())
        }
        fn deny_activate(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn ack_cancel(&mut self) -> Result<(), ResponseCode> {
            self.status = FdStatus::Idle { reason: 0 };
            Ok(())
        }
        fn grant_svn_commit(&mut self) -> Result<(), ResponseCode> {
            self.status = FdStatus::Idle { reason: 0 };
            Ok(())
        }
        fn deny_svn_commit(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
    }

    #[test]
    fn query_status_through_loopback() {
        let mut transport = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::idle()));
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let resp_len = round_trip(&mut transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success());
        let payload = wire::get_response_payload(&resp[..resp_len], &h).unwrap();
        let status = FdStatus::decode(payload).unwrap();
        assert_eq!(status, FdStatus::Idle { reason: 0 });
    }

    #[test]
    fn accept_offer_then_query_shows_ready_xfer() {
        let mut transport =
            Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::with_offer()));

        // Accept the offer.
        let mut req = [0u8; 16];
        let req_len = wire::encode_accept_offer(&mut req, 0x2000_0000).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let resp_len = round_trip(&mut transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success());

        // Query status: should now be ReadyXfer.
        let req_len = wire::encode_query_status(&mut req).unwrap();
        let resp_len = round_trip(&mut transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        let payload = wire::get_response_payload(&resp[..resp_len], &h).unwrap();
        let status = FdStatus::decode(payload).unwrap();
        assert_eq!(status, FdStatus::ReadyXfer);
    }

    #[test]
    fn reject_offer_then_query_shows_idle() {
        let mut transport =
            Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::with_offer()));

        let mut req = [0u8; 16];
        let req_len = wire::encode_reject_offer(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let resp_len = round_trip(&mut transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success());

        let req_len = wire::encode_query_status(&mut req).unwrap();
        let resp_len = round_trip(&mut transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        let payload = wire::get_response_payload(&resp[..resp_len], &h).unwrap();
        let status = FdStatus::decode(payload).unwrap();
        assert_eq!(status, FdStatus::Idle { reason: 0 });
    }

    #[test]
    fn start_while_pending_is_wrong_state() {
        let mut transport = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::idle()));
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();

        transport.start(&req[..req_len]).unwrap();
        assert_eq!(
            transport.start(&req[..req_len]),
            Err(TransportError::WrongState)
        );
    }

    #[test]
    fn poll_without_start_is_wrong_state() {
        let mut transport = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::idle()));
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        assert_eq!(transport.poll(&mut resp), Err(TransportError::WrongState));
    }

    #[test]
    fn cancel_releases_the_round_trip() {
        let mut transport = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::idle()));
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();

        transport.start(&req[..req_len]).unwrap();
        transport.cancel().unwrap();
        assert_eq!(transport.cancel(), Err(TransportError::WrongState));

        // The transport is usable again after a cancel.
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let resp_len = round_trip(&mut transport, &req[..req_len], &mut resp);
        assert!(wire::decode_response_header(&resp[..resp_len])
            .unwrap()
            .is_success());
    }

    #[test]
    fn poll_into_a_short_buffer_is_too_large() {
        let mut transport = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::idle()));
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();

        transport.start(&req[..req_len]).unwrap();
        let mut resp = [0u8; 1];
        assert_eq!(transport.poll(&mut resp), Err(TransportError::TooLarge));
        // The failed poll ended the round-trip.
        assert_eq!(transport.poll(&mut resp), Err(TransportError::WrongState));
    }

    /// Send a request and assert the response is success.
    fn send_ok<F: FdHandler>(
        transport: &mut Loopback<FdServer<F>, MAX_RESPONSE_SIZE>,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, WireError>,
    ) {
        let mut req = [0u8; 16];
        let req_len = encode(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let resp_len = round_trip(transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success());
    }

    /// Send QueryStatus and return the decoded FdStatus.
    fn query_status<F: FdHandler>(
        transport: &mut Loopback<FdServer<F>, MAX_RESPONSE_SIZE>,
    ) -> FdStatus {
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        let resp_len = round_trip(transport, &req[..req_len], &mut resp);
        let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
        assert!(h.is_success());
        let payload = wire::get_response_payload(&resp[..resp_len], &h).unwrap();
        FdStatus::decode(payload).unwrap()
    }

    #[test]
    fn offer_accept_reaches_ready_xfer() {
        let mut t = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::with_offer()));

        assert_eq!(
            query_status(&mut t),
            FdStatus::OfferPending {
                target: 0x0001,
                total: 0x0010_0000,
                mode: TransferMode::InTransport,
                svn_delayed: false,
            }
        );

        send_ok(&mut t, |b| wire::encode_accept_offer(b, 0x2000_0000));
        assert_eq!(query_status(&mut t), FdStatus::ReadyXfer);
    }

    // Transfer is UA-driven (no IPC op). The FD enters VerifyPending
    // when the transfer completes, so the grant sequence starts there.
    #[test]
    fn grant_sequence_verify_through_idle() {
        let mut t = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::at(
            FdStatus::VerifyPending,
        )));

        send_ok(&mut t, |b| wire::encode_grant_verify(b));
        assert_eq!(query_status(&mut t), FdStatus::ApplyPending);

        send_ok(&mut t, |b| wire::encode_grant_apply(b));
        assert_eq!(query_status(&mut t), FdStatus::ActivationPending);

        send_ok(&mut t, |b| wire::encode_grant_activate(b));
        assert_eq!(query_status(&mut t), FdStatus::Idle { reason: 0 });
    }

    #[test]
    fn svn_commit_after_activation() {
        let mut t = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::at(
            FdStatus::SvnCommitPending { component: 1 },
        )));

        send_ok(&mut t, |b| wire::encode_grant_svn_commit(b));
        assert_eq!(query_status(&mut t), FdStatus::Idle { reason: 0 });
    }

    #[test]
    fn ack_cancel_returns_to_idle() {
        let mut t =
            Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::at(FdStatus::Cancelled)));

        send_ok(&mut t, |b| wire::encode_ack_cancel(b));
        assert_eq!(query_status(&mut t), FdStatus::Idle { reason: 0 });
    }

    #[test]
    fn an_oversized_request_is_answered_by_dispatch_not_the_transport() {
        // A loopback has no request buffer to overflow, so nothing caps the
        // request at the transport. The decoder rejects it instead and the
        // caller gets an error frame, the same answer a malformed request of
        // any length gets.
        let mut transport = Loopback::<_, MAX_RESPONSE_SIZE>::new(FdServer::new(StubFd::idle()));
        let req = [0u8; MAX_REQUEST_SIZE + 1];
        let mut resp = [0u8; MAX_RESPONSE_SIZE];

        transport.start(&req).unwrap();
        let len = transport.poll(&mut resp).unwrap().unwrap();

        let h = wire::decode_response_header(&resp[..len]).unwrap();
        assert!(!h.is_success());
    }
}
