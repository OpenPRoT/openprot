// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The real `I3cClient` driven against the real `dispatch`, with no kernel.
//!
//! `LoopbackTransport` stands in for the IPC channel, so these tests exercise
//! the same encoders, the same `dispatch`, and the same latch semantics the
//! on-target build uses — the point of the transport seam.

use i3c_api::{I3cStatus, MAX_PAYLOAD};
use i3c_client::{ClientError, I3cClient};
use i3c_server::{Inbound, LoopbackTransport, Server};
use openprot_hal_blocking::i3c_hardware::{DynamicAddress, I3cTarget, TargetEvent};

/// A target whose inbound queue and outbound record are both visible to tests.
struct FakeTarget {
    addr: Option<u8>,
    inbound: Option<([u8; MAX_PAYLOAD], usize)>,
    sent: [u8; MAX_PAYLOAD],
    sent_len: usize,
}

impl FakeTarget {
    fn new(addr: Option<u8>) -> Self {
        Self {
            addr,
            inbound: None,
            sent: [0u8; MAX_PAYLOAD],
            sent_len: 0,
        }
    }

    fn queue(&mut self, frame: &[u8]) {
        let mut buf = [0u8; MAX_PAYLOAD];
        let n = frame.len().min(MAX_PAYLOAD);
        buf[..n].copy_from_slice(&frame[..n]);
        self.inbound = Some((buf, n));
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FakeError;

impl I3cTarget for FakeTarget {
    type Error = FakeError;

    fn enable(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error> {
        if self.inbound.is_some() {
            Ok(TargetEvent::InboundReady)
        } else {
            Ok(TargetEvent::None)
        }
    }

    fn read_frame(&mut self, buf: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        match self.inbound.take() {
            Some((frame, n)) if n <= buf.len() => {
                buf[..n].copy_from_slice(&frame[..n]);
                Ok(Some(n))
            }
            Some(_) => Err(FakeError),
            None => Ok(None),
        }
    }

    fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        let n = data.len().min(MAX_PAYLOAD);
        self.sent[..n].copy_from_slice(&data[..n]);
        self.sent_len = n;
        Ok(())
    }

    fn dynamic_address(&self) -> Option<DynamicAddress> {
        self.addr.and_then(|a| DynamicAddress::try_from(a).ok())
    }
}

fn client(addr: Option<u8>) -> I3cClient<LoopbackTransport<FakeTarget>> {
    I3cClient::new(LoopbackTransport::new(Server::new(
        1,
        2,
        FakeTarget::new(addr),
    )))
}

#[test]
fn send_reaches_the_target() {
    let mut c = client(Some(0x08));
    c.send(b"pong").expect("send");

    let target = &c.transport().server().target;
    assert_eq!(&target.sent[..target.sent_len], b"pong");
}

#[test]
fn recv_on_an_idle_bus_is_none() {
    let mut c = client(Some(0x08));
    let mut buf = [0u8; MAX_PAYLOAD];
    assert_eq!(c.recv(&mut buf).expect("recv"), None);
}

#[test]
fn recv_returns_the_latched_frame_once() {
    let mut c = client(Some(0x08));
    c.transport()
        .server()
        .target
        .queue(&[0x01, 0x02, 0x03, 0x04]);
    assert_eq!(
        c.transport().server().latch_inbound().expect("latch"),
        Inbound::Latched
    );

    let mut buf = [0u8; MAX_PAYLOAD];
    assert_eq!(c.recv(&mut buf).expect("recv"), Some(4));
    assert_eq!(&buf[..4], &[0x01, 0x02, 0x03, 0x04]);

    // The latch is consumed: a second read sees an idle bus.
    assert_eq!(c.recv(&mut buf).expect("recv"), None);
}

#[test]
fn dynamic_address_reads_back() {
    let mut c = client(Some(0x08));
    assert_eq!(c.dynamic_address().expect("addr"), Some(0x08));
}

#[test]
fn dynamic_address_before_entdaa_is_none() {
    let mut c = client(None);
    assert_eq!(c.dynamic_address().expect("addr"), None);
}

#[test]
fn oversize_send_is_rejected_client_side() {
    let mut c = client(Some(0x08));
    let big = [0u8; MAX_PAYLOAD + 1];
    assert_eq!(c.send(&big), Err(ClientError::BufferTooSmall));

    // Rejected before the transport ran, so the target saw nothing.
    assert_eq!(c.transport().server().target.sent_len, 0);
}

#[test]
fn recv_into_a_short_buffer_is_rejected_without_losing_the_frame() {
    let mut c = client(Some(0x08));
    c.transport().server().target.queue(&[0xaa; 16]);
    assert_eq!(
        c.transport().server().latch_inbound().expect("latch"),
        Inbound::Latched
    );

    // A buffer too small to hold the frame is rejected before the round-trip.
    let mut small = [0u8; 4];
    assert_eq!(c.recv(&mut small), Err(ClientError::BufferTooSmall));

    // Crucially, the latch was not consumed: a proper read still gets the frame.
    let mut buf = [0u8; MAX_PAYLOAD];
    assert_eq!(c.recv(&mut buf).expect("recv"), Some(16));
    assert_eq!(&buf[..16], &[0xaa; 16]);
}

#[test]
fn a_full_payload_round_trips() {
    let mut c = client(Some(0x08));
    let payload = [0x5au8; MAX_PAYLOAD];

    c.send(&payload).expect("send");
    let target = &c.transport().server().target;
    assert_eq!(target.sent_len, MAX_PAYLOAD);
    assert_eq!(&target.sent[..], &payload[..]);
}

#[test]
fn server_status_surfaces_as_an_error() {
    // `dispatch` answers an unknown opcode with InvalidOp; the client reports
    // it rather than silently treating it as success.
    struct BadOp;
    impl i3c_api::Transport for BadOp {
        fn transact(
            &mut self,
            _req: &[u8],
            resp: &mut [u8],
        ) -> Result<usize, i3c_api::TransportError> {
            resp[0] = I3cStatus::Internal as u8;
            Ok(1)
        }
    }

    let mut c = I3cClient::new(BadOp);
    assert_eq!(c.send(b"x"), Err(ClientError::Server(I3cStatus::Internal)));
}
