// Licensed under the Apache-2.0 license

//! Wire-protocol dispatch integration test.
//!
//! Exercises the full IPC path: encode request → `dispatch_mctp_op` → decode
//! response. This verifies that the wire protocol + dispatch layer + Server
//! work together correctly, simulating what happens when `IpcMctpClient`
//! talks to the MCTP server process over a Pigweed IPC channel.

mod common;

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::wire;
use openprot_mctp_server::{dispatch::dispatch_mctp_op, Server};

use common::{transfer, BufferSender};


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test SetEid + GetEid via dispatch.
#[test]
fn dispatch_set_get_eid() {
    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(0), 0, sender);

    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    // SetEid(42)
    let req_len = wire::encode_set_eid(&mut req, 42).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());

    // GetEid → 42
    let req_len = wire::encode_get_eid(&mut req).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
    assert_eq!(header.eid, 42);
}

/// Test Listener + Req + Send + Recv via dispatch (full echo round-trip).
#[test]
fn dispatch_echo_roundtrip() {
    // Server A (echo responder, EID 8)
    let buf_a = RefCell::new(Vec::new());
    let sender_a = BufferSender { packets: &buf_a };
    let mut server_a: Server<_, 16> = Server::new(Eid(8), 0, sender_a);

    // Server B (requester, EID 42)
    let buf_b = RefCell::new(Vec::new());
    let sender_b = BufferSender { packets: &buf_b };
    let mut server_b: Server<_, 16> = Server::new(Eid(42), 0, sender_b);

    let mut req = [0u8; 128];
    let mut resp = [0u8; 128];
    let mut recv_buf = [0u8; 255];

    // Register listener on A for MsgType(1)
    let req_len = wire::encode_listener(&mut req, 1).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
    let listener_handle = header.handle;

    // Register req on B targeting EID 8
    let req_len = wire::encode_req(&mut req, 8).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
    let req_handle = header.handle;

    // B sends a message via dispatch
    let payload = b"dispatch echo!";
    let req_len = wire::encode_send(
        &mut req,
        Some(req_handle),
        1,
        None,
        None,
        false,
        payload,
    )
    .unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());

    // Transfer B → A
    transfer(&buf_b, &mut server_a);

    // A receives via dispatch (message already queued by transfer above)
    let req_len = wire::encode_recv(&mut req, listener_handle, 0).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
    assert_eq!(header.msg_type, 1);
    assert_eq!(header.eid, 42); // remote EID
    let recv_payload = wire::get_response_payload(&resp[..resp_len], &header).unwrap();
    assert_eq!(recv_payload, payload);

    // A echoes back via dispatch (response: no handle, set eid + tag)
    let req_len = wire::encode_send(
        &mut req,
        None,
        header.msg_type,
        Some(header.eid),
        Some(header.tag),
        false,
        recv_payload,
    )
    .unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf).unwrap();
    let send_header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(send_header.is_success());

    // Transfer A → B
    transfer(&buf_a, &mut server_b);

    // B receives the echo via dispatch (message already queued by transfer above)
    let req_len = wire::encode_recv(&mut req, req_handle, 0).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
    assert_eq!(header.msg_type, 1);
    assert_eq!(header.eid, 8); // from server A
    let echo_payload = wire::get_response_payload(&resp[..resp_len], &header).unwrap();
    assert_eq!(echo_payload, payload);
}

// ---------------------------------------------------------------------------
// Malformed request → BadArgument
// ---------------------------------------------------------------------------

/// A request buffer shorter than the header size must return `BadArgument`.
#[test]
fn dispatch_malformed_request_returns_bad_argument() {
    use openprot_mctp_api::ResponseCode;

    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    // Two bytes — shorter than MctpRequestHeader::SIZE (12)
    let bad_request = [0u8; 2];
    let resp_len = dispatch_mctp_op(&bad_request, &mut resp, &mut server, &mut recv_buf).unwrap();

    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(!header.is_success());
    assert_eq!(header.response_code(), ResponseCode::BadArgument);
}

/// An opcode byte that is not a known `MctpOp` must return `BadArgument`.
#[test]
fn dispatch_unknown_opcode_returns_bad_argument() {
    use openprot_mctp_api::ResponseCode;

    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    // 12-byte header with opcode 0xFF (unrecognised)
    let mut bad_request = [0u8; 12];
    bad_request[0] = 0xFF;
    let resp_len = dispatch_mctp_op(&bad_request, &mut resp, &mut server, &mut recv_buf).unwrap();

    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(!header.is_success());
    assert_eq!(header.response_code(), ResponseCode::BadArgument);
}

// ---------------------------------------------------------------------------
// MctpOp::Recv deferred behaviour
// ---------------------------------------------------------------------------

/// `Recv` dispatched when no message has arrived must return `None` (deferred):
/// the client is parked in the server's outstanding table rather than
/// immediately answered with `TimedOut`.
#[test]
fn dispatch_recv_no_message_parks_client() {
    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    // Register a listener
    let req_len = wire::encode_listener(&mut req, 1).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf).unwrap();
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(h.is_success());
    let listener_handle = h.handle;

    // Attempt Recv immediately — no inbound packet → must be deferred
    let req_len = wire::encode_recv(&mut req, listener_handle, 0).unwrap();
    let result = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
    assert!(result.is_none(), "expected deferred recv (None)");
}

/// After parking a `Recv`, feeding an inbound packet via `server.inbound()` +
/// `server.update()` must fulfil the outstanding recv and make the message
/// retrievable.
#[test]
fn dispatch_recv_deferred_then_fulfilled() {
    use openprot_mctp_server::RecvResult;
    use common::BufferSender;

    // Server A: listener (EID 8)
    let buf_a = RefCell::new(Vec::new());
    let mut server_a: Server<_, 16> = Server::new(Eid(8), 0, BufferSender { packets: &buf_a });

    // Server B: requester (EID 42) — used only to generate a valid MCTP packet
    let buf_b = RefCell::new(Vec::new());
    let mut server_b: Server<_, 16> = Server::new(Eid(42), 0, BufferSender { packets: &buf_b });

    let mut req = [0u8; 128];
    let mut resp = [0u8; 128];
    let mut recv_buf = [0u8; 255];

    // A registers listener for MsgType(1)
    let req_len = wire::encode_listener(&mut req, 1).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf).unwrap();
    let listener_handle = wire::decode_response_header(&resp[..resp_len]).unwrap().handle;

    // B allocates a req handle → sends a message → packet lands in buf_b
    let req_len = wire::encode_req(&mut req, 8).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf).unwrap();
    let req_handle = wire::decode_response_header(&resp[..resp_len]).unwrap().handle;

    let payload = b"hello deferred";
    let req_len = wire::encode_send(&mut req, Some(req_handle), 1, None, None, false, payload).unwrap();
    dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf).unwrap();

    // A calls Recv before any transfer — must park (None)
    let req_len = wire::encode_recv(&mut req, listener_handle, 5000).unwrap();
    let result = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf);
    assert!(result.is_none(), "expected deferred recv");

    // Now deliver the packet from B to A's router
    transfer(&buf_b, &mut server_a);

    // update() must fulfil the parked recv
    let (_, ready) = server_a.update(0, &mut recv_buf);
    assert_eq!(ready.len(), 1);
    match ready[0].1 {
        RecvResult::Message(ref meta) => {
            assert_eq!(meta.msg_type, 1);
            assert_eq!(meta.remote_eid, 42);
            assert_eq!(&recv_buf[..meta.payload_size], payload);
        }
        RecvResult::TimedOut => panic!("unexpected timeout"),
    }
}

// ---------------------------------------------------------------------------
// MctpOp::Unbind
// ---------------------------------------------------------------------------

/// `Unbind` on a valid handle returns success.
#[test]
fn dispatch_unbind_valid_handle() {
    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    // Allocate a listener handle
    let req_len = wire::encode_listener(&mut req, 1).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf).unwrap();
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(h.is_success());
    let listener_handle = h.handle;

    // Unbind it
    let req_len = wire::encode_unbind(&mut req, listener_handle).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
}

/// `Unbind` on a handle that was never allocated still returns success.
/// (The server's `unbind` is idempotent — it ignores unknown handles.)
#[test]
fn dispatch_unbind_unknown_handle_is_idempotent() {
    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    let req_len = wire::encode_unbind(&mut req, 0xDEAD_BEEF).unwrap();
    let resp_len = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf).unwrap();
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
}
