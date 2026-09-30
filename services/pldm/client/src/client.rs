// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The client handle and the answers it collects.

use pldm_api::wire::{self, PldmOp, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use pldm_api::{FdStatus, RejectReason, WireError};
use util_service::AsyncTransport;

use crate::ClientError;

/// What the firmware device answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    /// The FD took the request. Every operation but `QueryStatus`
    /// answers this or a refusal. An acknowledgement, not a verdict:
    /// what the request sets in motion runs afterwards, and
    /// `QueryStatus` is what reports how it went.
    Acked,
    /// The FD's current condition, from `QueryStatus`.
    Status(FdStatus),
}

/// The orchestrator's handle on one firmware device.
pub struct FdIpcClient<T> {
    transport: T,
    /// What was asked, while the answer is outstanding. The response
    /// frame does not name the operation, so this is what says how to
    /// read the payload.
    in_flight: Option<PldmOp>,
    response: [u8; MAX_RESPONSE_SIZE],
}

impl<T> FdIpcClient<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            in_flight: None,
            response: [0u8; MAX_RESPONSE_SIZE],
        }
    }

    /// The operation waiting for an answer, if any. The event loop reads
    /// it to decide whether this device still owes it a poll.
    pub fn in_flight(&self) -> Option<PldmOp> {
        self.in_flight
    }

    /// The transport, to register its channel with a wait group.
    pub fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: AsyncTransport> FdIpcClient<T> {
    /// Accept the offer and tell the FD where to stage.
    pub fn accept_offer(&mut self, staging_base: u32) -> Result<(), ClientError> {
        self.start(PldmOp::AcceptOffer, |buf| {
            wire::encode_accept_offer(buf, staging_base)
        })
    }

    /// Refuse the offer, with the reason the requester is owed.
    pub fn reject_offer(&mut self, reason: RejectReason) -> Result<(), ClientError> {
        self.start(PldmOp::RejectOffer, |buf| {
            wire::encode_reject_offer(buf, reason)
        })
    }

    /// Tell the FD to verify what it staged.
    pub fn perform_verify(&mut self) -> Result<(), ClientError> {
        self.start(PldmOp::PerformVerify, wire::encode_perform_verify)
    }

    /// Tell the FD not to verify, with the reason the requester is owed.
    pub fn reject_verify(&mut self, reason: RejectReason) -> Result<(), ClientError> {
        self.start(PldmOp::RejectVerify, |buf| {
            wire::encode_reject_verify(buf, reason)
        })
    }

    /// Tell the FD to apply what it verified.
    pub fn perform_apply(&mut self) -> Result<(), ClientError> {
        self.start(PldmOp::PerformApply, wire::encode_perform_apply)
    }

    /// Tell the FD not to apply.
    pub fn reject_apply(&mut self, reason: RejectReason) -> Result<(), ClientError> {
        self.start(PldmOp::RejectApply, |buf| {
            wire::encode_reject_apply(buf, reason)
        })
    }

    /// Ask what the FD is doing.
    pub fn query_status(&mut self) -> Result<(), ClientError> {
        self.start(PldmOp::QueryStatus, wire::encode_query_status)
    }

    /// Tell the FD to activate the applied image.
    ///
    /// Sent when the FD reports apply complete, not when the UA asks to
    /// activate: `FdOps::activate` answers the UA synchronously, so the
    /// FD stores this verdict and replies from it. The gap before the UA
    /// asks is unbounded, so this stays revocable until then.
    pub fn perform_activate(&mut self) -> Result<(), ClientError> {
        self.start(PldmOp::PerformActivate, wire::encode_perform_activate)
    }

    /// Refuse the activation, or take back one the FD has stored.
    ///
    /// A revocation loses the race once the UA has asked: by then the FD
    /// has answered and activation is under way.
    pub fn reject_activate(&mut self, reason: RejectReason) -> Result<(), ClientError> {
        self.start(PldmOp::RejectActivate, |buf| {
            wire::encode_reject_activate(buf, reason)
        })
    }

    /// Acknowledge the FD's cancel.
    pub fn ack_cancel(&mut self) -> Result<(), ClientError> {
        self.start(PldmOp::AckCancel, wire::encode_ack_cancel)
    }

    /// Tell the FD the SVN floor is raised, so it can answer the UA.
    pub fn perform_svn_commit(&mut self) -> Result<(), ClientError> {
        self.start(PldmOp::PerformSvnCommit, wire::encode_perform_svn_commit)
    }

    /// Tell the FD the floor did not move.
    pub fn reject_svn_commit(&mut self, reason: RejectReason) -> Result<(), ClientError> {
        self.start(PldmOp::RejectSvnCommit, |buf| {
            wire::encode_reject_svn_commit(buf, reason)
        })
    }

    /// Collect the answer, if it has arrived.
    ///
    /// `Ok(None)` means the FD has not answered yet and the caller polls
    /// again. Anything else ends the round-trip, refusals included, so
    /// the next call is another request.
    pub fn poll(&mut self) -> Result<Option<Reply>, ClientError> {
        let op = self.in_flight.ok_or(ClientError::Idle)?;
        let polled = self.transport.poll(&mut self.response);
        // Any error from poll ends the round-trip and discards the
        // response, so the client must not keep waiting for one.
        let Some(len) = polled.inspect_err(|_| self.in_flight = None)? else {
            return Ok(None);
        };
        self.in_flight = None;
        self.decode(op, len).map(Some)
    }

    /// Abandon the round-trip in flight.
    ///
    /// The client is idle afterwards either way, so a failed cancel is
    /// reported but not retried here. With the channel transport a
    /// failed cancel has lost the channel for good, and every later
    /// `start` answers `WrongState`.
    pub fn cancel(&mut self) -> Result<(), ClientError> {
        if self.in_flight.is_none() {
            return Err(ClientError::Idle);
        }
        self.in_flight = None;
        Ok(self.transport.cancel()?)
    }

    /// Encode one request and hand it to the transport. The operation is
    /// recorded only once the transport has taken it, so a refused start
    /// leaves the client idle.
    fn start(
        &mut self,
        op: PldmOp,
        encode: impl FnOnce(&mut [u8]) -> Result<usize, WireError>,
    ) -> Result<(), ClientError> {
        if self.in_flight.is_some() {
            return Err(ClientError::Busy);
        }
        let mut request = [0u8; MAX_REQUEST_SIZE];
        let len = encode(&mut request)?;
        self.transport.start(&request[..len])?;
        self.in_flight = Some(op);
        Ok(())
    }

    fn decode(&self, op: PldmOp, len: usize) -> Result<Reply, ClientError> {
        let frame = &self.response[..len];
        let header = wire::decode_response_header(frame)?;
        if !header.is_success() {
            return Err(ClientError::Refused(header.response_code()));
        }
        match op {
            PldmOp::QueryStatus => {
                let payload = wire::get_response_payload(frame, &header)?;
                Ok(Reply::Status(FdStatus::decode(payload)?))
            }
            _ => Ok(Reply::Acked),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pldm_api::status::TransferMode;
    use pldm_api::ResponseCode;
    use pldm_server::{FdIpcHandler, FdIpcServer};
    use util_service::{Delayed, Loopback, TransportError};

    /// Answers every operation, with the verdict and status the test
    /// chooses. Records the last call so a test can prove the request
    /// reached the far side.
    struct StubFd {
        last: Option<&'static str>,
        status: FdStatus,
        refuse_with: Option<ResponseCode>,
    }

    impl StubFd {
        fn new() -> Self {
            Self {
                last: None,
                status: FdStatus::Idle { reason: 0 },
                refuse_with: None,
            }
        }

        fn refusing(code: ResponseCode) -> Self {
            Self {
                refuse_with: Some(code),
                ..Self::new()
            }
        }

        fn holding(status: FdStatus) -> Self {
            Self {
                status,
                ..Self::new()
            }
        }

        fn answer(&mut self, name: &'static str) -> Result<(), ResponseCode> {
            self.last = Some(name);
            match self.refuse_with {
                Some(code) => Err(code),
                None => Ok(()),
            }
        }
    }

    impl FdIpcHandler for StubFd {
        fn accept_offer(&mut self, _staging_base: u32) -> Result<(), ResponseCode> {
            self.answer("accept_offer")
        }
        fn reject_offer(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
            self.answer("reject_offer")
        }
        fn perform_verify(&mut self) -> Result<(), ResponseCode> {
            self.answer("perform_verify")
        }
        fn reject_verify(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
            self.answer("reject_verify")
        }
        fn perform_apply(&mut self) -> Result<(), ResponseCode> {
            self.answer("perform_apply")
        }
        fn reject_apply(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
            self.answer("reject_apply")
        }
        fn query_status(&mut self) -> Result<FdStatus, ResponseCode> {
            self.last = Some("query_status");
            match self.refuse_with {
                Some(code) => Err(code),
                None => Ok(self.status),
            }
        }
        fn perform_activate(&mut self) -> Result<(), ResponseCode> {
            self.answer("perform_activate")
        }
        fn reject_activate(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
            self.answer("reject_activate")
        }
        fn ack_cancel(&mut self) -> Result<(), ResponseCode> {
            self.answer("ack_cancel")
        }
        fn perform_svn_commit(&mut self) -> Result<(), ResponseCode> {
            self.answer("perform_svn_commit")
        }
        fn reject_svn_commit(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
            self.answer("reject_svn_commit")
        }
    }

    type Direct = FdIpcClient<Loopback<FdIpcServer<StubFd>, MAX_RESPONSE_SIZE>>;

    fn client(fd: StubFd) -> Direct {
        FdIpcClient::new(Loopback::new(FdIpcServer::new(fd)))
    }

    /// The handler behind the loopback, to assert on what reached it.
    fn handler(client: &Direct) -> &StubFd {
        client.transport().server().handler()
    }

    #[test]
    fn a_granted_operation_is_acked() {
        let mut c = client(StubFd::new());

        c.perform_verify().unwrap();
        assert_eq!(c.in_flight(), Some(PldmOp::PerformVerify));
        assert_eq!(c.poll(), Ok(Some(Reply::Acked)));

        assert_eq!(handler(&c).last, Some("perform_verify"));
        assert_eq!(c.in_flight(), None);
    }

    #[test]
    fn an_offer_carries_the_staging_base() {
        let mut c = client(StubFd::new());

        c.accept_offer(0x2000_0000).unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Acked)));

        assert_eq!(handler(&c).last, Some("accept_offer"));
    }

    #[test]
    fn a_denial_carries_its_reason() {
        let mut c = client(StubFd::new());

        c.reject_verify(RejectReason::Isolated).unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Acked)));

        assert_eq!(handler(&c).last, Some("reject_verify"));
    }

    #[test]
    fn query_status_answers_the_devices_condition() {
        let offered = FdStatus::OfferPending {
            target: 0x0001,
            total: 0x0010_0000,
            mode: TransferMode::InTransport,
            svn_delayed: false,
        };
        let mut c = client(StubFd::holding(offered));

        c.query_status().unwrap();

        assert_eq!(c.poll(), Ok(Some(Reply::Status(offered))));
    }

    // A refusal is the FD's decision and ends the round-trip like any
    // other answer: the channel is fine, the request is not.
    #[test]
    fn a_refusal_ends_the_round_trip_with_the_code() {
        let mut c = client(StubFd::refusing(ResponseCode::WrongPhase));

        c.perform_apply().unwrap();

        assert_eq!(
            c.poll(),
            Err(ClientError::Refused(ResponseCode::WrongPhase))
        );
        assert_eq!(c.in_flight(), None, "the round-trip is over");
    }

    #[test]
    fn a_second_request_while_one_is_in_flight_is_refused() {
        let mut c = client(StubFd::new());

        c.perform_verify().unwrap();

        assert_eq!(c.perform_apply(), Err(ClientError::Busy));
        assert_eq!(c.in_flight(), Some(PldmOp::PerformVerify));
    }

    #[test]
    fn polling_with_nothing_in_flight_is_refused() {
        let mut c = client(StubFd::new());

        assert_eq!(c.poll(), Err(ClientError::Idle));
    }

    #[test]
    fn cancel_frees_the_client_for_the_next_request() {
        let mut c = client(StubFd::new());
        c.perform_verify().unwrap();

        c.cancel().unwrap();

        assert_eq!(c.in_flight(), None);
        c.perform_apply().unwrap();
        assert_eq!(c.poll(), Ok(Some(Reply::Acked)));
        assert_eq!(handler(&c).last, Some("perform_apply"));
    }

    #[test]
    fn cancel_with_nothing_in_flight_is_refused() {
        let mut c = client(StubFd::new());

        assert_eq!(c.cancel(), Err(ClientError::Idle));
    }

    // The production transport answers over a kernel channel, so the
    // first poll usually finds nothing. The loopback alone cannot reach
    // that path.
    #[test]
    fn a_response_that_is_not_ready_yet_polls_again() {
        let mut c = FdIpcClient::new(Delayed::new(
            Loopback::<_, MAX_RESPONSE_SIZE>::new(FdIpcServer::new(StubFd::new())),
            2,
        ));

        c.perform_verify().unwrap();

        assert_eq!(c.poll(), Ok(None));
        assert_eq!(c.poll(), Ok(None));
        assert_eq!(c.in_flight(), Some(PldmOp::PerformVerify), "still waiting");
        assert_eq!(c.poll(), Ok(Some(Reply::Acked)));
        assert_eq!(c.in_flight(), None);
    }

    // A start the transport refuses leaves the client idle, so the
    // caller can try something else. Whether the request reached the
    // far side is the transport's business: the loopback dispatches
    // before it finds the response does not fit.
    #[test]
    fn a_refused_start_leaves_the_client_idle() {
        // A response buffer of zero makes the loopback answer TooLarge.
        let mut c: FdIpcClient<Loopback<FdIpcServer<StubFd>, 0>> =
            FdIpcClient::new(Loopback::new(FdIpcServer::new(StubFd::new())));

        assert_eq!(
            c.perform_verify(),
            Err(ClientError::Transport(TransportError::TooLarge))
        );
        assert_eq!(c.in_flight(), None);
    }

    /// Answers every poll with a failure, the way a dead channel does.
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

    // A failed poll ends the round-trip: the response is gone, so a
    // client that kept waiting for it would never send anything again.
    #[test]
    fn a_failed_poll_ends_the_round_trip() {
        let mut c = FdIpcClient::new(DeadChannel);
        c.perform_verify().unwrap();

        assert_eq!(
            c.poll(),
            Err(ClientError::Transport(TransportError::Failed))
        );
        assert_eq!(c.in_flight(), None);
        // The client is free to try again rather than wedged.
        c.perform_verify().unwrap();
    }

    /// Answers with a frame too short to be a response header.
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

    // An answer that does not decode still ends the round-trip: the
    // client forgets what it asked before it reads the frame, so a bad
    // response cannot leave it waiting forever.
    #[test]
    fn a_response_that_does_not_decode_ends_the_round_trip() {
        let mut c = FdIpcClient::new(GarbageChannel);
        c.perform_verify().unwrap();

        assert_eq!(c.poll(), Err(ClientError::Wire(WireError::Truncated)));
        assert_eq!(c.in_flight(), None);
        c.perform_verify().unwrap();
    }

    /// Refuses every cancel, the way a channel that cannot take its
    /// buffers back does.
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

    // A refused cancel is reported, and the client is idle either way:
    // it has abandoned the round-trip and has nothing left to poll for.
    #[test]
    fn a_failed_cancel_is_reported_and_leaves_the_client_idle() {
        let mut c = FdIpcClient::new(UncancellableChannel);
        c.perform_verify().unwrap();

        assert_eq!(
            c.cancel(),
            Err(ClientError::Transport(TransportError::Failed))
        );
        assert_eq!(c.in_flight(), None);
    }
}
