// Licensed under the Apache-2.0 license

//! MCTP integration tests — multi-fragment, multi-listener, MctpListener trait.
//!
//! Tests in this file exercise:
//! - Multi-fragment reassembly (small MTU sender)
//! - Multiple concurrent listeners with no cross-talk
//! - `MctpListener` + `MctpRespChannel` trait path (mirrors real echo application)
//! - `MctpReqChannel` trait path
//! - `drop_handle` mid-flight clears the outstanding entry
//!
//! No I2C transport is used anywhere in this file.

mod common;

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::stack::Stack;
use openprot_mctp_api::{MctpClient, MctpListener, MctpReqChannel, MctpRespChannel};
use openprot_mctp_server::Server;

use common::{transfer, BufferSender, DirectClient, DirectListener, DirectReqChannel, SmallMtuBufferSender};

// ---------------------------------------------------------------------------
// Multi-fragment roundtrip
// ---------------------------------------------------------------------------

/// Send a 200-byte payload through a server whose sender MTU is 64 bytes.
///
/// The fragmenter must split it into multiple packets. The receiving server
/// must reassemble them before delivering to the listener.
#[test]
fn multi_fragment_roundtrip() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    // Server A: small MTU sender (forces fragmentation)
    let sender_a = SmallMtuBufferSender {
        packets: &buf_a,
        mtu: 64,
    };
    let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));

    // Server B: normal MTU (sends the request)
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let listener = client_a.listener(1).unwrap();
    let req = client_b.req(8).unwrap();

    // 200-byte payload — exceeds a single 64-byte MTU fragment
    let payload: Vec<u8> = (0u8..200).collect();
    client_b
        .send(Some(req), 1, None, None, false, &payload)
        .unwrap();

    // Transfer B → A (may be multiple packets)
    transfer(&buf_b, &mut server_a.borrow_mut());

    // A should have reassembled and delivered to the listener
    let mut recv_buf = [0u8; 512];
    let meta = client_a
        .recv(listener, 0, &mut recv_buf)
        .expect("A should receive the reassembled message");

    assert_eq!(meta.payload_size, payload.len());
    assert_eq!(&recv_buf[..meta.payload_size], payload.as_slice());
    assert_eq!(meta.remote_eid, 42);
}

// ---------------------------------------------------------------------------
// Multiple concurrent listeners — no cross-talk
// ---------------------------------------------------------------------------

/// Two listeners on the same server for different msg_types each receive only
/// their own messages.
#[test]
fn multiple_listeners_no_crosstalk() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    // Two listeners: type 1 and type 2
    let listener_type1 = client_a.listener(1).unwrap();
    let listener_type2 = client_a.listener(2).unwrap();

    // B sends type 2
    let req = client_b.req(8).unwrap();
    let msg_type2 = b"for type 2";
    client_b
        .send(Some(req), 2, None, None, false, msg_type2)
        .unwrap();
    transfer(&buf_b, &mut server_a.borrow_mut());

    // Type 1 listener should see nothing
    let mut buf = [0u8; 255];
    assert!(
        client_a.recv(listener_type1, 0, &mut buf).is_err(),
        "type-1 listener should not receive a type-2 message"
    );

    // Type 2 listener should see the message
    let meta = client_a
        .recv(listener_type2, 0, &mut buf)
        .expect("type-2 listener should receive the message");
    assert_eq!(&buf[..meta.payload_size], msg_type2);
}

// ---------------------------------------------------------------------------
// MctpListener + MctpRespChannel trait path (real echo application shape)
// ---------------------------------------------------------------------------

/// Exercises the `MctpListener` / `MctpRespChannel` traits — the same interface
/// used by the real echo application — with `BufferSender` as the transport.
///
/// This is the key test that lets the echo application logic run without I2C:
/// ```
/// fn echo_app(listener: &mut impl MctpListener) {
///     let (meta, msg, mut resp) = listener.recv(&mut buf).unwrap();
///     resp.send(msg).unwrap();
/// }
/// ```
#[test]
fn echo_via_mctplistener_trait() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let listener_handle = client_a.listener(1).unwrap();
    let req_handle = client_b.req(8).unwrap();

    // B sends a request
    let request = b"echo via trait";
    client_b
        .send(Some(req_handle), 1, None, None, false, request)
        .unwrap();
    transfer(&buf_b, &mut server_a.borrow_mut());

    // A echoes back using the MctpListener + MctpRespChannel trait path
    // (same shape as the real echo application in hubris/task/mctp-echo)
    let mut listener = DirectListener::new(&client_a, listener_handle);
    let mut recv_buf = [0u8; 255];
    let (meta, payload, mut resp) = listener
        .recv(&mut recv_buf)
        .expect("listener should have a message ready");

    assert_eq!(payload, request);
    assert_eq!(meta.remote_eid, 42);

    resp.send(payload).expect("response send should succeed");

    // Transfer A → B and verify
    transfer(&buf_a, &mut server_b.borrow_mut());

    let mut resp_buf = [0u8; 255];
    let resp_meta = client_b
        .recv(req_handle, 0, &mut resp_buf)
        .expect("B should receive the echo");

    assert_eq!(&resp_buf[..resp_meta.payload_size], request);
    assert_eq!(resp_meta.remote_eid, 8);
    assert_eq!(resp_meta.msg_type, 1);
}

// ---------------------------------------------------------------------------
// MctpReqChannel trait path
// ---------------------------------------------------------------------------

/// Exercises `MctpReqChannel::send` + `MctpReqChannel::recv` trait methods.
#[test]
fn req_channel_send_recv() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let listener_handle = client_a.listener(1).unwrap();
    let req_handle = client_b.req(8).unwrap();

    // B sends via MctpReqChannel
    let mut req_channel = DirectReqChannel::new(&client_b, req_handle, 1, 8);
    req_channel
        .send(1, b"req channel test")
        .expect("req channel send should succeed");
    assert_eq!(req_channel.remote_eid(), 8);

    transfer(&buf_b, &mut server_a.borrow_mut());

    // A echoes manually (through MctpClient)
    let mut echo_buf = [0u8; 255];
    let meta = client_a
        .recv(listener_handle, 0, &mut echo_buf)
        .unwrap();
    client_a
        .send(
            None,
            meta.msg_type,
            Some(meta.remote_eid),
            Some(meta.msg_tag),
            false,
            &echo_buf[..meta.payload_size],
        )
        .unwrap();

    transfer(&buf_a, &mut server_b.borrow_mut());

    // B receives via MctpReqChannel
    let mut resp_buf = [0u8; 255];
    let (resp_meta, resp_payload) = req_channel
        .recv(&mut resp_buf)
        .expect("req channel recv should succeed");

    assert_eq!(resp_payload, b"req channel test");
    assert_eq!(resp_meta.remote_eid, 8);
}

// ---------------------------------------------------------------------------
// drop_handle mid-flight
// ---------------------------------------------------------------------------

/// Dropping a listener handle while a recv is outstanding clears the entry.
/// After `unbind`, `try_recv` no longer panics and the handle is gone.
#[test]
fn drop_handle_mid_flight_clears_entry() {
    let sender = common::DroppingBufferSender;
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let handle = server.listener(1).unwrap();

    // Register a pending recv
    server
        .register_recv(handle, 1000, 0)
        .expect("register_recv should succeed");

    // Drop the handle before any message or timeout
    server.unbind(handle).expect("unbind should succeed");

    // update should return nothing for that handle
    let mut recv_buf = [0u8; 255];
    let (_, ready) = server.update(500, &mut recv_buf);
    assert!(
        ready.iter().all(|(h, _)| *h != handle),
        "dropped handle should not appear in update results"
    );
}

// ---------------------------------------------------------------------------
// Response-without-handle: tag & EID threading
// ---------------------------------------------------------------------------

/// A response sent without a handle (the reply path) correctly threads the
/// remote EID and tag back so the requester receives it.
#[test]
fn response_without_handle_eid_tag_threading() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let listener = client_a.listener(5).unwrap();
    let req = client_b.req(8).unwrap();

    // B sends a type-5 request
    client_b
        .send(Some(req), 5, None, None, false, b"ping")
        .unwrap();
    transfer(&buf_b, &mut server_a.borrow_mut());

    // A receives and replies — no handle, explicit EID + tag
    let mut buf = [0u8; 255];
    let meta = client_a.recv(listener, 0, &mut buf).unwrap();
    client_a
        .send(
            None,
            meta.msg_type,
            Some(meta.remote_eid),
            Some(meta.msg_tag),
            false,
            b"pong",
        )
        .unwrap();

    transfer(&buf_a, &mut server_b.borrow_mut());

    // B receives the response and verifies metadata
    let mut resp_buf = [0u8; 255];
    let resp = client_b
        .recv(req, 0, &mut resp_buf)
        .expect("B should receive pong");

    assert_eq!(&resp_buf[..resp.payload_size], b"pong");
    assert_eq!(resp.remote_eid, 8);
    assert_eq!(resp.msg_type, 5);
}

// ---------------------------------------------------------------------------
// Stack facade (openprot-mctp-api::stack)
// ---------------------------------------------------------------------------
//
// These tests exercise `Stack<DirectClient>` — the same code path used by the
// real application (`mctp_echo.rs` with `Stack<IpcMctpClient>`), but running
// entirely on the host with no IPC or embedded target.

/// Echo via `Stack::listener` → `StackListener::recv` → `StackRespChannel::send`.
///
/// This is the exact sequence used by `mctp_echo.rs`:
/// ```ignore
/// let mut listener = stack.listener(ECHO_MSG_TYPE, 0).unwrap();
/// let (meta, msg, mut resp) = listener.recv(&mut buf).unwrap();
/// resp.send(msg).unwrap();
/// ```
#[test]
fn stack_listener_echo() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    // Application A uses Stack — the same API as in production.
    let stack_a = Stack::new(DirectClient::new(&server_a));
    // Application B uses a raw client to send the request and check the reply.
    let client_b = DirectClient::new(&server_b);

    let mut listener = stack_a.listener(1, 0).expect("listener alloc");
    let req_handle = client_b.req(8).unwrap();

    // B sends a request
    client_b
        .send(Some(req_handle), 1, None, None, false, b"hello from B")
        .unwrap();
    transfer(&buf_b, &mut server_a.borrow_mut());

    // A echoes back via Stack facade
    let mut recv_buf = [0u8; 255];
    let (meta, payload, mut resp) = listener
        .recv(&mut recv_buf)
        .expect("stack listener should receive the message");

    assert_eq!(payload, b"hello from B");
    assert_eq!(meta.remote_eid, 42);

    resp.send(payload).expect("stack resp send");

    // Deliver A → B and verify
    transfer(&buf_a, &mut server_b.borrow_mut());

    let mut resp_buf = [0u8; 255];
    let resp_meta = client_b
        .recv(req_handle, 0, &mut resp_buf)
        .expect("B should receive the echo");

    assert_eq!(&resp_buf[..resp_meta.payload_size], b"hello from B");
    assert_eq!(resp_meta.remote_eid, 8);
    assert_eq!(resp_meta.msg_type, 1);
}

/// Echo via `Stack::req` → `StackReqChannel::send` + `StackReqChannel::recv`.
#[test]
fn stack_req_channel_roundtrip() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let client_a = DirectClient::new(&server_a);
    // B uses Stack for the request side.
    let stack_b = Stack::new(DirectClient::new(&server_b));

    let listener_handle = client_a.listener(1).unwrap();

    let mut req = stack_b.req(8, 0).expect("req channel alloc");
    req.send(1, b"stack req test").expect("req send");
    assert_eq!(req.remote_eid(), 8);

    transfer(&buf_b, &mut server_a.borrow_mut());

    // A echoes back manually
    let mut echo_buf = [0u8; 255];
    let meta = client_a.recv(listener_handle, 0, &mut echo_buf).unwrap();
    client_a
        .send(
            None,
            meta.msg_type,
            Some(meta.remote_eid),
            Some(meta.msg_tag),
            false,
            &echo_buf[..meta.payload_size],
        )
        .unwrap();

    transfer(&buf_a, &mut server_b.borrow_mut());

    let mut resp_buf = [0u8; 255];
    let (resp_meta, resp_payload) = req.recv(&mut resp_buf).expect("req channel recv");

    assert_eq!(resp_payload, b"stack req test");
    assert_eq!(resp_meta.remote_eid, 8);
    assert_eq!(resp_meta.msg_type, 1);
}

/// Both sides use `Stack` — listener on A, request channel on B.
#[test]
fn stack_both_sides_echo() {
    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let stack_a = Stack::new(DirectClient::new(&server_a));
    let stack_b = Stack::new(DirectClient::new(&server_b));

    let mut listener = stack_a.listener(1, 0).expect("listener alloc");
    let mut req = stack_b.req(8, 0).expect("req alloc");

    // B sends
    req.send(1, b"both sides").expect("req send");
    transfer(&buf_b, &mut server_a.borrow_mut());

    // A receives and replies via Stack
    let mut recv_buf = [0u8; 255];
    let (_, payload, mut resp) = listener.recv(&mut recv_buf).expect("listener recv");
    assert_eq!(payload, b"both sides");
    resp.send(payload).expect("resp send");
    transfer(&buf_a, &mut server_b.borrow_mut());

    // B receives via Stack req channel
    let mut resp_buf = [0u8; 255];
    let (meta, data) = req.recv(&mut resp_buf).expect("req recv");
    assert_eq!(data, b"both sides");
    assert_eq!(meta.remote_eid, 8);
}

/// `Stack::get_eid` and `Stack::set_eid` delegate correctly.
#[test]
fn stack_eid_accessors() {
    let server: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, common::DroppingBufferSender));
    let stack = Stack::new(DirectClient::new(&server));

    assert_eq!(stack.get_eid(), 8);
    stack.set_eid(99).expect("set_eid should succeed");
    assert_eq!(stack.get_eid(), 99);
}

