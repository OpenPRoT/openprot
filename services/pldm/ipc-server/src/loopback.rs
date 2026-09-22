// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! In-process loopback transport for host testing.
//!
//! Calls `dispatch` directly, no kernel, no IPC channel. The client
//! crate's encoders and decoders exercise the real server dispatch
//! path through this transport.

use pldm_ipc_api::transport::{Transport, TransportError};
use pldm_ipc_api::wire::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};

use crate::{dispatch, FdHandler};

/// Loopback transport that dispatches in-process.
///
/// Owns the `FdHandler` so the client and server share a single
/// process with no concurrency. Useful for host-side integration
/// tests and for validating wire format round-trips.
///
/// Dispatch runs inside `start`, so the response is ready on the first
/// `poll`. The split-phase shape is kept so client code written against
/// a real channel runs unchanged here.
pub struct LoopbackTransport<F> {
    handler: F,
    response: [u8; MAX_RESPONSE_SIZE],
    /// Response length, set while a round-trip is in flight.
    pending: Option<usize>,
}

impl<F> LoopbackTransport<F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            response: [0u8; MAX_RESPONSE_SIZE],
            pending: None,
        }
    }
}

impl<F: FdHandler> Transport for LoopbackTransport<F> {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError> {
        if self.pending.is_some() {
            return Err(TransportError::WrongState);
        }
        if req.len() > MAX_REQUEST_SIZE {
            return Err(TransportError::TooLarge);
        }

        let n = dispatch(&mut self.handler, req, &mut self.response);
        if n == 0 {
            return Err(TransportError::Failed);
        }
        self.pending = Some(n);
        Ok(())
    }

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
        let n = self.pending.ok_or(TransportError::WrongState)?;
        if resp.len() < n {
            self.pending = None;
            return Err(TransportError::TooLarge);
        }
        resp[..n].copy_from_slice(&self.response[..n]);
        self.pending = None;
        Ok(Some(n))
    }

    fn cancel(&mut self) -> Result<(), TransportError> {
        if self.pending.take().is_none() {
            return Err(TransportError::WrongState);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Start one request and poll the response out, as the client layer
    /// does; the loopback always has it ready on the first poll.
    fn round_trip<F: FdHandler>(
        transport: &mut LoopbackTransport<F>,
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
            Ok(())
        }
        fn deny_verify(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn grant_apply(&mut self) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn deny_apply(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn query_status(&mut self) -> Result<FdStatus, ResponseCode> {
            Ok(self.status)
        }
        fn grant_activate(&mut self) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn deny_activate(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn ack_cancel(&mut self) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn grant_svn_commit(&mut self) -> Result<(), ResponseCode> {
            Ok(())
        }
        fn deny_svn_commit(&mut self, _reason: DenyReason) -> Result<(), ResponseCode> {
            Ok(())
        }
    }

    #[test]
    fn query_status_through_loopback() {
        let mut transport = LoopbackTransport::new(StubFd::idle());
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
        let mut transport = LoopbackTransport::new(StubFd::with_offer());

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
        let mut transport = LoopbackTransport::new(StubFd::with_offer());

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
        let mut transport = LoopbackTransport::new(StubFd::idle());
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
        let mut transport = LoopbackTransport::new(StubFd::idle());
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        assert_eq!(transport.poll(&mut resp), Err(TransportError::WrongState));
    }

    #[test]
    fn cancel_releases_the_round_trip() {
        let mut transport = LoopbackTransport::new(StubFd::idle());
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
        let mut transport = LoopbackTransport::new(StubFd::idle());
        let mut req = [0u8; 16];
        let req_len = wire::encode_query_status(&mut req).unwrap();

        transport.start(&req[..req_len]).unwrap();
        let mut resp = [0u8; 1];
        assert_eq!(transport.poll(&mut resp), Err(TransportError::TooLarge));
        // The failed poll ended the round-trip.
        assert_eq!(transport.poll(&mut resp), Err(TransportError::WrongState));
    }

    #[test]
    fn start_with_an_oversized_request_is_too_large() {
        let mut transport = LoopbackTransport::new(StubFd::idle());
        let req = [0u8; MAX_REQUEST_SIZE + 1];
        assert_eq!(transport.start(&req), Err(TransportError::TooLarge));
        // Nothing was started, so there is nothing to poll.
        let mut resp = [0u8; MAX_RESPONSE_SIZE];
        assert_eq!(transport.poll(&mut resp), Err(TransportError::WrongState));
    }
}
