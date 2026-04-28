// Licensed under the Apache-2.0 license

//! Server unit tests — exercise `Server` methods directly without I2C.
//!
//! Each test constructs a `Server` with `DroppingBufferSender` (or
//! `BufferSender` when outbound packets are needed) and calls the
//! server API directly. No transport hardware is involved.

mod common;

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::{ResponseCode};
use openprot_mctp_server::{RecvResult, Server, ServerConfig};

use common::{BufferSender, DroppingBufferSender, transfer};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deliver a message from a sender EID to a receiver server by routing it
/// through a real `Server::send()` call, avoiding any direct Fragmenter API.
///
/// Creates a temporary sender server (EID `src`) with a `BufferSender`, sends
/// one message of `msg_type` to `dst_eid`, then feeds the captured packets
/// into `dest` via `inbound`.
fn deliver_to<S: mctp_lib::Sender, const N: usize>(
    src: u8,
    dst_eid: u8,
    msg_type: u8,
    payload: &[u8],
    dest: &mut Server<S, N>,
) {
    let buf = RefCell::new(Vec::new());
    let mut sender_server: Server<BufferSender<'_>, 16> =
        Server::new(Eid(src), 0, BufferSender { packets: &buf });

    let req_handle = sender_server.req(dst_eid).unwrap();
    sender_server
        .send(Some(req_handle), msg_type, None, None, false, payload)
        .unwrap();

    transfer(&buf, dest);
}

// ---------------------------------------------------------------------------
// EID management
// ---------------------------------------------------------------------------

/// `get_eid` returns the EID passed to `Server::new`.
#[test]
fn eid_initial_value() {
    let sender = DroppingBufferSender;
    let server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    assert_eq!(server.get_eid(), 8);
}

/// `set_eid` + `get_eid` round-trip.
#[test]
fn eid_set_get_roundtrip() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(0), 0, sender);
    server.set_eid(42).expect("set_eid should succeed");
    assert_eq!(server.get_eid(), 42);
}

// ---------------------------------------------------------------------------
// Handle allocation / deallocation
// ---------------------------------------------------------------------------

/// `req()` succeeds and `unbind()` releases the handle cleanly.
#[test]
fn req_handle_alloc_and_unbind() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let handle = server.req(42).expect("req should succeed");
    server.unbind(handle).expect("unbind should succeed");
}

/// `listener()` succeeds and `unbind()` releases the handle cleanly.
#[test]
fn listener_handle_alloc_and_unbind() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let handle = server.listener(1).expect("listener should succeed");
    server.unbind(handle).expect("unbind should succeed");
}

/// Registering a second listener for the same `msg_type` returns `AddrInUse`.
#[test]
fn listener_duplicate_msg_type_returns_addr_in_use() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    server.listener(1).expect("first listener should succeed");
    let err = server.listener(1).expect_err("duplicate listener should fail");
    assert_eq!(err.code, ResponseCode::AddrInUse);
}

/// Two listeners for *different* `msg_type` values both succeed.
#[test]
fn listener_different_types_both_succeed() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let h1 = server.listener(1).expect("listener type 1 should succeed");
    let h2 = server.listener(2).expect("listener type 2 should succeed");
    assert_ne!(h1, h2);
}

// ---------------------------------------------------------------------------
// try_recv before inbound
// ---------------------------------------------------------------------------

/// `try_recv` returns `None` when no message has been fed via `inbound`.
#[test]
fn try_recv_before_inbound_returns_none() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let handle = server.listener(1).unwrap();
    let mut buf = [0u8; 255];
    assert!(server.try_recv(handle, &mut buf).is_none());
}

// ---------------------------------------------------------------------------
// inbound → try_recv routing
// ---------------------------------------------------------------------------

/// A raw packet fed via `inbound` is delivered to the matching listener.
#[test]
fn inbound_then_try_recv_delivers_message() {
    let buf_out = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf_out };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let listener = server.listener(1).unwrap();

    let payload = b"hello";
    deliver_to(42, 8, 1, payload, &mut server);

    let mut recv_buf = [0u8; 255];
    let meta = server
        .try_recv(listener, &mut recv_buf)
        .expect("message should be available after inbound");

    assert_eq!(meta.msg_type, 1);
    assert_eq!(meta.remote_eid, 42);
    assert_eq!(meta.payload_size, payload.len());
    assert_eq!(&recv_buf[..meta.payload_size], payload);
}

/// A packet for msg_type 2 is not delivered to a listener for msg_type 1.
#[test]
fn inbound_wrong_type_not_delivered() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let listener = server.listener(1).unwrap();

    deliver_to(42, 8, 2, b"wrong type", &mut server);

    let mut buf = [0u8; 255];
    assert!(server.try_recv(listener, &mut buf).is_none());
}

// ---------------------------------------------------------------------------
// send with oversized payload
// ---------------------------------------------------------------------------

/// `send` with a payload larger than `MAX_PAYLOAD` returns `NoSpace`.
#[test]
fn send_oversized_payload_returns_no_space() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let req_handle = server.req(42).unwrap();

    let big_payload = vec![0u8; ServerConfig::MAX_PAYLOAD + 1];
    let err = server
        .send(Some(req_handle), 1, None, None, false, &big_payload)
        .expect_err("oversized send should fail");
    assert_eq!(err.code, ResponseCode::NoSpace);
}

// ---------------------------------------------------------------------------
// register_recv + update timeout
// ---------------------------------------------------------------------------

/// A registered recv with a timeout fires `RecvResult::TimedOut` after the deadline.
#[test]
fn pending_recv_times_out() {
    let sender = DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let listener = server.listener(1).unwrap();

    // Register a recv with a 100 ms timeout, starting at t=0.
    server
        .register_recv(listener, 100, 0)
        .expect("register_recv should succeed");

    let mut recv_buf = [0u8; 255];

    // At t=50 ms: not yet timed out, no message.
    let (_, ready) = server.update(50, &mut recv_buf);
    assert!(ready.is_empty(), "should not fire before deadline");

    // At t=100 ms: deadline reached.
    let (_, ready) = server.update(100, &mut recv_buf);
    assert_eq!(ready.len(), 1);
    assert!(
        matches!(ready[0], (h, RecvResult::TimedOut) if h == listener),
        "expected TimedOut for listener handle"
    );
}

/// A registered recv that receives a message before the deadline resolves with
/// `RecvResult::Message`, not a timeout.
#[test]
fn pending_recv_fulfilled_before_timeout() {
    let buf_out = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf_out };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);
    let listener = server.listener(1).unwrap();

    server
        .register_recv(listener, 1000, 0)
        .expect("register_recv should succeed");

    deliver_to(42, 8, 1, b"data", &mut server);

    let mut recv_buf = [0u8; 255];
    let (_, ready) = server.update(50, &mut recv_buf);

    assert_eq!(ready.len(), 1);
    assert!(
        matches!(ready[0], (h, RecvResult::Message(_)) if h == listener),
        "expected Message result"
    );
    if let (_, RecvResult::Message(meta)) = ready[0] {
        assert_eq!(meta.remote_eid, 42);
        assert_eq!(meta.msg_type, 1);
    }
}
