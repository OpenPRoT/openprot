// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The client handle and the answers it collects.

use crypto_api::wire::{self, CryptoOp, VerifyRegion, VerifyStatus, MAX_RESPONSE_SIZE};
use util_service::AsyncTransport;

use crate::ClientError;

/// What the crypto service answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    /// The service took the request. QueryStatus reports how the
    /// hash goes.
    Accepted,
    /// Current verify status, from QueryStatus.
    Status(VerifyStatus),
}

/// The orchestrator's handle on the crypto service.
///
/// Generic over the transport so the same encode/decode paths run
/// behind a kernel channel in production and inside a Loopback in
/// host tests.
pub struct CryptoIpcClient<T> {
    transport: T,
    in_flight: Option<CryptoOp>,
    response: [u8; MAX_RESPONSE_SIZE],
}

impl<T> CryptoIpcClient<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            in_flight: None,
            response: [0u8; MAX_RESPONSE_SIZE],
        }
    }

    pub fn in_flight(&self) -> Option<CryptoOp> {
        self.in_flight
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: AsyncTransport> CryptoIpcClient<T> {
    /// Begin verifying a flash region. The crypto service reads the
    /// bytes from the flash service itself.
    pub fn start_verify(&mut self, region: &VerifyRegion) -> Result<(), ClientError> {
        self.send(CryptoOp::StartVerify, &wire::encode_start_verify(region))
    }

    /// Ask what the service is doing.
    pub fn query_status(&mut self) -> Result<(), ClientError> {
        self.send(CryptoOp::QueryStatus, &wire::encode_query_status())
    }

    /// Collect the answer, if it has arrived.
    ///
    /// `Ok(None)` means the service has not answered yet. Anything else
    /// ends the round-trip, refusals included.
    pub fn poll(&mut self) -> Result<Option<Reply>, ClientError> {
        let op = self.in_flight.ok_or(ClientError::Idle)?;
        let polled = self.transport.poll(&mut self.response);
        let Some(len) = polled.inspect_err(|_| self.in_flight = None)? else {
            return Ok(None);
        };
        self.in_flight = None;
        self.decode(op, len).map(Some)
    }

    /// Abandon the round-trip in flight.
    pub fn cancel(&mut self) -> Result<(), ClientError> {
        if self.in_flight.is_none() {
            return Err(ClientError::Idle);
        }
        self.in_flight = None;
        Ok(self.transport.cancel()?)
    }

    fn send(&mut self, op: CryptoOp, request: &[u8]) -> Result<(), ClientError> {
        if self.in_flight.is_some() {
            return Err(ClientError::Busy);
        }
        self.transport.start(request)?;
        self.in_flight = Some(op);
        Ok(())
    }

    fn decode(&self, op: CryptoOp, len: usize) -> Result<Reply, ClientError> {
        let frame = &self.response[..len];
        let header = wire::decode_response_header(frame)?;
        if !header.is_success() {
            return Err(ClientError::Refused(header.response_code()?));
        }
        match op {
            CryptoOp::QueryStatus => {
                let payload = wire::get_response_payload(frame, &header)?;
                Ok(Reply::Status(VerifyStatus::decode(payload)?))
            }
            CryptoOp::StartVerify => Ok(Reply::Accepted),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_api::{ResponseCode, WireError};
    use util_service::{Delayed, Dispatch, DispatchError, Loopback, TransportError};

    /// Minimal inline dispatcher that speaks the crypto wire protocol.
    struct StubCrypto {
        status: VerifyStatus,
        refuse_with: Option<ResponseCode>,
        last_address: Option<u32>,
        last_length: Option<u32>,
    }

    impl StubCrypto {
        fn new() -> Self {
            Self {
                status: VerifyStatus::Idle,
                refuse_with: None,
                last_address: None,
                last_length: None,
            }
        }

        fn refusing(code: ResponseCode) -> Self {
            Self {
                refuse_with: Some(code),
                ..Self::new()
            }
        }

        fn holding(status: VerifyStatus) -> Self {
            Self {
                status,
                ..Self::new()
            }
        }
    }

    impl Dispatch for StubCrypto {
        fn dispatch(
            &mut self,
            request: &[u8],
            response: &mut [u8],
        ) -> Result<usize, DispatchError> {
            let header = wire::decode_request_header(request)
                .map_err(|_| DispatchError::ResponseTooLarge)?;
            let Some(op) = header.operation() else {
                return wire::encode_error_response(response, ResponseCode::InvalidOp)
                    .map_err(|_| DispatchError::ResponseTooLarge);
            };

            if let Some(code) = self.refuse_with {
                return wire::encode_error_response(response, code)
                    .map_err(|_| DispatchError::ResponseTooLarge);
            }

            match op {
                CryptoOp::StartVerify => {
                    let args = wire::get_request_args(request);
                    let Ok(region) = wire::get_start_verify_args(args) else {
                        return wire::encode_error_response(
                            response,
                            ResponseCode::MalformedRequest,
                        )
                        .map_err(|_| DispatchError::ResponseTooLarge);
                    };
                    self.last_address = Some(region.address);
                    self.last_length = Some(region.length);
                    wire::encode_success_response(response)
                        .map_err(|_| DispatchError::ResponseTooLarge)
                }
                CryptoOp::QueryStatus => wire::encode_status_response(response, &self.status)
                    .map_err(|_| DispatchError::ResponseTooLarge),
            }
        }
    }

    const REGION: VerifyRegion = VerifyRegion {
        address: 0x2000_0000,
        length: 0x0008_0000,
    };
    const ZERO_REGION: VerifyRegion = VerifyRegion {
        address: 0,
        length: 0,
    };

    type Direct = CryptoIpcClient<Loopback<StubCrypto, MAX_RESPONSE_SIZE>>;

    fn client(stub: StubCrypto) -> Direct {
        CryptoIpcClient::new(Loopback::new(stub))
    }

    fn handler(client: &Direct) -> &StubCrypto {
        client.transport().server()
    }

    #[test]
    fn start_verify_carries_the_region() {
        let mut c = client(StubCrypto::new());

        c.start_verify(&REGION).unwrap();
        assert_eq!(c.in_flight(), Some(CryptoOp::StartVerify));
        assert_eq!(c.poll(), Ok(Some(Reply::Accepted)));

        assert_eq!(handler(&c).last_address, Some(0x2000_0000));
        assert_eq!(handler(&c).last_length, Some(0x0008_0000));
        assert_eq!(c.in_flight(), None);
    }

    #[test]
    fn query_status_reports_hashing_progress() {
        let status = VerifyStatus::Hashing {
            hashed: 0x4000,
            total: 0x0010_0000,
        };
        let mut c = client(StubCrypto::holding(status));

        c.query_status().unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Status(status))));
    }

    #[test]
    fn query_status_reports_idle() {
        let mut c = client(StubCrypto::new());

        c.query_status().unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Status(VerifyStatus::Idle))));
    }

    #[test]
    fn query_status_reports_authenticated() {
        let mut c = client(StubCrypto::holding(VerifyStatus::Authenticated));

        c.query_status().unwrap();
        assert_eq!(
            c.poll(),
            Ok(Some(Reply::Status(VerifyStatus::Authenticated)))
        );
    }

    #[test]
    fn query_status_reports_rejected() {
        let mut c = client(StubCrypto::holding(VerifyStatus::Rejected));

        c.query_status().unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Status(VerifyStatus::Rejected))));
    }

    #[test]
    fn a_refusal_ends_the_round_trip_with_the_code() {
        let mut c = client(StubCrypto::refusing(ResponseCode::WrongState));

        c.start_verify(&ZERO_REGION).unwrap();
        assert_eq!(
            c.poll(),
            Err(ClientError::Refused(ResponseCode::WrongState))
        );
        assert_eq!(c.in_flight(), None);
    }

    #[test]
    fn a_second_request_while_one_is_in_flight_is_refused() {
        let mut c = client(StubCrypto::new());

        c.start_verify(&ZERO_REGION).unwrap();
        assert_eq!(c.query_status(), Err(ClientError::Busy));
        assert_eq!(c.in_flight(), Some(CryptoOp::StartVerify));
    }

    #[test]
    fn polling_with_nothing_in_flight_is_refused() {
        let mut c = client(StubCrypto::new());
        assert_eq!(c.poll(), Err(ClientError::Idle));
    }

    #[test]
    fn cancel_frees_the_client_for_the_next_request() {
        let mut c = client(StubCrypto::new());
        c.start_verify(&ZERO_REGION).unwrap();

        c.cancel().unwrap();

        assert_eq!(c.in_flight(), None);
        c.query_status().unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Status(VerifyStatus::Idle))));
    }

    #[test]
    fn cancel_with_nothing_in_flight_is_refused() {
        let mut c = client(StubCrypto::new());
        assert_eq!(c.cancel(), Err(ClientError::Idle));
    }

    #[test]
    fn a_response_that_is_not_ready_yet_polls_again() {
        let mut c = CryptoIpcClient::new(Delayed::new(
            Loopback::<_, MAX_RESPONSE_SIZE>::new(StubCrypto::new()),
            2,
        ));

        c.start_verify(&ZERO_REGION).unwrap();

        assert_eq!(c.poll(), Ok(None));
        assert_eq!(c.poll(), Ok(None));
        assert_eq!(c.in_flight(), Some(CryptoOp::StartVerify), "still waiting");
        assert_eq!(c.poll(), Ok(Some(Reply::Accepted)));
        assert_eq!(c.in_flight(), None);
    }

    #[test]
    fn a_failed_start_leaves_the_client_idle() {
        let mut c: CryptoIpcClient<Loopback<StubCrypto, 0>> =
            CryptoIpcClient::new(Loopback::new(StubCrypto::new()));

        assert_eq!(
            c.start_verify(&ZERO_REGION),
            Err(ClientError::Transport(TransportError::TooLarge))
        );
        assert_eq!(c.in_flight(), None);
    }

    struct BareSuccessChannel;

    impl AsyncTransport for BareSuccessChannel {
        fn start(&mut self, _req: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
            let h = wire::ResponseHeader::success();
            resp[..wire::ResponseHeader::SIZE].copy_from_slice(&h.to_bytes());
            Ok(Some(wire::ResponseHeader::SIZE))
        }
        fn cancel(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[test]
    fn query_status_with_empty_payload_is_a_wire_error() {
        let mut c = CryptoIpcClient::new(BareSuccessChannel);
        c.query_status().unwrap();

        assert_eq!(c.poll(), Err(ClientError::Wire(WireError::Truncated)));
        assert_eq!(c.in_flight(), None);
    }

    struct DeadChannel;

    impl AsyncTransport for DeadChannel {
        fn start(&mut self, _req: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn poll(&mut self, _resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
            Err(TransportError::Failed)
        }
        fn cancel(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[test]
    fn a_failed_poll_ends_the_round_trip() {
        let mut c = CryptoIpcClient::new(DeadChannel);
        c.start_verify(&ZERO_REGION).unwrap();

        assert_eq!(
            c.poll(),
            Err(ClientError::Transport(TransportError::Failed))
        );
        assert_eq!(c.in_flight(), None);
        c.start_verify(&ZERO_REGION).unwrap();
    }

    struct GarbageChannel;

    impl AsyncTransport for GarbageChannel {
        fn start(&mut self, _req: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
            resp[..2].copy_from_slice(&[0xff, 0xff]);
            Ok(Some(2))
        }
        fn cancel(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[test]
    fn a_response_that_does_not_decode_ends_the_round_trip() {
        let mut c = CryptoIpcClient::new(GarbageChannel);
        c.start_verify(&ZERO_REGION).unwrap();

        assert_eq!(c.poll(), Err(ClientError::Wire(WireError::Truncated)));
        assert_eq!(c.in_flight(), None);
        c.start_verify(&ZERO_REGION).unwrap();
    }

    struct UncancellableChannel;

    impl AsyncTransport for UncancellableChannel {
        fn start(&mut self, _req: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn poll(&mut self, _resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
            Ok(None)
        }
        fn cancel(&mut self) -> Result<(), TransportError> {
            Err(TransportError::Failed)
        }
    }

    #[test]
    fn a_failed_cancel_is_reported_and_leaves_the_client_idle() {
        let mut c = CryptoIpcClient::new(UncancellableChannel);
        c.start_verify(&ZERO_REGION).unwrap();

        assert_eq!(
            c.cancel(),
            Err(ClientError::Transport(TransportError::Failed))
        );
        assert_eq!(c.in_flight(), None);
    }
}
