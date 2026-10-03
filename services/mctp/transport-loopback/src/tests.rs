// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The queue contract, plus a round trip between two cross-wired servers.
//!
//! A `Fragmenter` is built by the router rather than by callers, so
//! `send_vectored` is exercised through real servers rather than called
//! directly. The two-server arrangement mirrors
//! `services/mctp/echo/tests/echo_host.rs`, which is where it comes from.

use super::*;

use mctp::Eid;
use openprot_mctp_server::Server;

/// The two endpoints. Values are arbitrary; they only have to differ.
const EID_A: u8 = 8;
const EID_B: u8 = 42;

/// Vendor-defined message type. Nothing here cares what the bytes mean.
const MSG_TYPE: u8 = 0x7E;

/// Moves everything one side queued into the other side, the way a wire
/// would. Returns how many packets crossed.
fn transfer<S: Sender, const N: usize, const DEPTH: usize>(
    from: &LoopbackQueue<DEPTH>,
    to: &mut Server<S, N>,
) -> usize {
    let mut crossed = 0;
    while let Some(pkt) = from.take() {
        to.inbound(&pkt).expect("inbound accepts the packet");
        crossed += 1;
    }
    crossed
}

#[test]
fn a_new_queue_has_nothing_in_it() {
    let q: LoopbackQueue<4> = LoopbackQueue::new();
    assert_eq!(q.pending(), 0);
    assert_eq!(q.dropped(), 0);
}

#[test]
fn the_advertised_mtu_excludes_the_header() {
    let q: LoopbackQueue<4> = LoopbackQueue::new();
    let s = LoopbackSender::new(&q);
    assert_eq!(s.get_mtu(), LOOPBACK_MTU);
    // The packets it queues carry the header on top of that payload.
    assert_eq!(Packet::new().capacity(), LOOPBACK_MTU + MCTP_HEADER_SIZE);
}

#[test]
fn taking_from_an_empty_queue_gives_nothing() {
    let q: LoopbackQueue<4> = LoopbackQueue::new();
    assert!(q.take().is_none());
}

#[test]
fn a_message_crosses_from_one_server_to_the_other() {
    let out_a: LoopbackQueue<16> = LoopbackQueue::new();
    let out_b: LoopbackQueue<16> = LoopbackQueue::new();
    let mut a: Server<_, 16> = Server::new(Eid(EID_A), 0, LoopbackSender::new(&out_a));
    let mut b: Server<_, 16> = Server::new(Eid(EID_B), 0, LoopbackSender::new(&out_b));

    let listener = b.listener(MSG_TYPE).expect("B binds a listener");

    let payload = b"openprot";
    let req = a.req(EID_B).expect("A opens a request to B");
    a.send(Some(req), MSG_TYPE, Some(EID_B), None, false, payload)
        .expect("A sends");

    assert!(transfer(&out_a, &mut b) > 0, "nothing crossed the wire");

    let mut buf = [0u8; LOOPBACK_MTU];
    let meta = b.try_recv(listener, &mut buf).expect("B receives it");
    assert_eq!(&buf[..payload.len()], payload);
    assert_eq!(meta.remote_eid, EID_A);
}

#[test]
fn a_payload_longer_than_the_mtu_arrives_whole() {
    let out_a: LoopbackQueue<16> = LoopbackQueue::new();
    let out_b: LoopbackQueue<16> = LoopbackQueue::new();
    let mut a: Server<_, 16> = Server::new(Eid(EID_A), 0, LoopbackSender::new(&out_a));
    let mut b: Server<_, 16> = Server::new(Eid(EID_B), 0, LoopbackSender::new(&out_b));

    let listener = b.listener(MSG_TYPE).expect("B binds a listener");

    // More than one fragment, so reassembly is actually exercised.
    let payload: std::vec::Vec<u8> = (0..(LOOPBACK_MTU * 2) as u16).map(|i| i as u8).collect();
    let req = a.req(EID_B).expect("A opens a request to B");
    a.send(Some(req), MSG_TYPE, Some(EID_B), None, false, &payload)
        .expect("A sends");

    let fragments = transfer(&out_a, &mut b);
    assert!(fragments > 1, "expected several fragments, got {fragments}");

    let mut buf = [0u8; LOOPBACK_MTU * 3];
    b.try_recv(listener, &mut buf).expect("B receives it");
    assert_eq!(&buf[..payload.len()], &payload[..]);
}

#[test]
fn the_reply_crosses_back() {
    let out_a: LoopbackQueue<16> = LoopbackQueue::new();
    let out_b: LoopbackQueue<16> = LoopbackQueue::new();
    let mut a: Server<_, 16> = Server::new(Eid(EID_A), 0, LoopbackSender::new(&out_a));
    let mut b: Server<_, 16> = Server::new(Eid(EID_B), 0, LoopbackSender::new(&out_b));

    let listener = b.listener(MSG_TYPE).expect("B binds a listener");
    let req = a.req(EID_B).expect("A opens a request to B");
    a.send(Some(req), MSG_TYPE, Some(EID_B), None, false, b"ping")
        .expect("A sends");
    transfer(&out_a, &mut b);

    let mut buf = [0u8; LOOPBACK_MTU];
    let meta = b
        .try_recv(listener, &mut buf)
        .expect("B receives the request");

    // A response carries no handle and reuses the request's tag.
    b.send(
        None,
        MSG_TYPE,
        Some(EID_A),
        Some(meta.msg_tag),
        false,
        b"pong",
    )
    .expect("B replies");
    assert!(transfer(&out_b, &mut a) > 0, "the reply did not cross");

    let mut reply = [0u8; LOOPBACK_MTU];
    a.try_recv(req, &mut reply).expect("A receives the reply");
    assert_eq!(&reply[..4], b"pong");
}
