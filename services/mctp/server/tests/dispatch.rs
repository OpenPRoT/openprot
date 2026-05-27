// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Wire-protocol dispatch integration test.
//!
//! Exercises the full request path: encode request → `dispatch_mctp_op` → decode
//! response. This verifies that the wire protocol + dispatch layer + Server
//! work together correctly for a client/server integration boundary.

mod common;

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::{wire, Handle};
use openprot_mctp_server::{dispatch::{dispatch_mctp_op, drive_pending, DispatchOutcome}, Server};

use common::{transfer, BufferSender};

// Convenience wrapper: calls dispatch_mctp_op with now_millis=0 and panics on Pending.
fn dispatch_reply<S: openprot_mctp_server::Sender, const N: usize>(
    request: &[u8],
    response: &mut [u8],
    server: &mut Server<S, N>,
    recv_buf: &mut [u8],
) -> usize {
    match dispatch_mctp_op(request, response, server, recv_buf, 0) {
        DispatchOutcome::Reply(n) => n,
        DispatchOutcome::Pending { handle } => {
            panic!("unexpected Pending for handle {:?}", handle)
        }
    }
}

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
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());

    // GetEid → 42
    let req_len = wire::encode_get_eid(&mut req).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
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
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf);
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
    let listener_handle = header.handle;

    // Register req on B targeting EID 8
    let req_len = wire::encode_req(&mut req, 8).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
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
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());

    // Transfer B → A
    transfer(&buf_b, &mut server_a);

    // A receives via dispatch
    let req_len = wire::encode_recv(&mut req, listener_handle, 0).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf);
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
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf);
    let send_header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(send_header.is_success());

    // Transfer A → B
    transfer(&buf_a, &mut server_b);

    // B receives the echo via dispatch
    let req_len = wire::encode_recv(&mut req, req_handle, 0).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
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
    let resp_len = dispatch_reply(&bad_request, &mut resp, &mut server, &mut recv_buf);

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
    let resp_len = dispatch_reply(&bad_request, &mut resp, &mut server, &mut recv_buf);

    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(!header.is_success());
    assert_eq!(header.response_code(), ResponseCode::BadArgument);
}

// ---------------------------------------------------------------------------
// MctpOp::Recv — deferred path
// ---------------------------------------------------------------------------

/// `Recv` when no message is ready returns `Pending`, not an immediate error.
#[test]
fn dispatch_recv_returns_pending_when_no_message() {
    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    // Register a listener
    let req_len = wire::encode_listener(&mut req, 1).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    let listener_handle = Handle(h.handle);

    // Attempt Recv immediately — no inbound packet → Pending
    let req_len = wire::encode_recv(&mut req, listener_handle.0, 500).unwrap();
    let outcome = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf, 0);
    assert!(
        matches!(outcome, DispatchOutcome::Pending { handle } if handle == listener_handle),
        "expected Pending"
    );
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
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(h.is_success());
    let listener_handle = h.handle;

    // Unbind it
    let req_len = wire::encode_unbind(&mut req, listener_handle).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
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
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
    let header = wire::decode_response_header(&resp[..resp_len]).unwrap();
    assert!(header.is_success());
}

// ---------------------------------------------------------------------------
// drive_pending — deferred recv delivery
// ---------------------------------------------------------------------------

/// After a `Pending` recv, `drive_pending` fires `on_ready` once a packet arrives.
#[test]
fn dispatch_recv_resolved_by_drive_pending() {
    let buf_a = RefCell::new(Vec::new());
    let sender_a = BufferSender { packets: &buf_a };
    let mut server_a: Server<_, 16> = Server::new(Eid(8), 0, sender_a);

    let buf_b = RefCell::new(Vec::new());
    let sender_b = BufferSender { packets: &buf_b };
    let mut server_b: Server<_, 16> = Server::new(Eid(42), 0, sender_b);

    let mut req = [0u8; 128];
    let mut resp = [0u8; 128];
    let mut recv_buf = [0u8; 255];

    // Register listener on A
    let req_len = wire::encode_listener(&mut req, 7).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    let listener_handle = Handle(h.handle);

    // A tries recv before any message arrives → Pending
    let req_len = wire::encode_recv(&mut req, listener_handle.0, 1000).unwrap();
    let outcome = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf, 0);
    assert!(matches!(outcome, DispatchOutcome::Pending { .. }));

    // B allocates req handle and sends a message to A
    let req_len = wire::encode_req(&mut req, 8).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    let req_handle = h.handle;

    let payload = b"hello pending";
    let req_len =
        wire::encode_send(&mut req, Some(req_handle), 7, None, None, false, payload).unwrap();
    dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
    transfer(&buf_b, &mut server_a);

    // drive_pending should deliver the message
    let mut fired_handle: Option<Handle> = None;
    let mut fired_len = 0usize;
    drive_pending(&mut server_a, 0, &mut recv_buf, &mut resp, |h, n| {
        fired_handle = Some(h);
        fired_len = n;
    });

    assert_eq!(fired_handle, Some(listener_handle));
    let header = wire::decode_response_header(&resp[..fired_len]).unwrap();
    assert!(header.is_success());
    assert_eq!(header.msg_type, 7);
    assert_eq!(header.eid, 42);
    let recv_payload = wire::get_response_payload(&resp[..fired_len], &header).unwrap();
    assert_eq!(recv_payload, payload);
}

/// After a `Pending` recv with a timeout, `drive_pending` at `now > deadline`
/// fires `on_ready` with a `TimedOut` response.
#[test]
fn dispatch_recv_timeout() {
    use openprot_mctp_api::ResponseCode;

    let buf = RefCell::new(Vec::new());
    let sender = BufferSender { packets: &buf };
    let mut server: Server<_, 16> = Server::new(Eid(8), 0, sender);

    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    let mut recv_buf = [0u8; 255];

    let req_len = wire::encode_listener(&mut req, 1).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    let listener_handle = Handle(h.handle);

    // Register recv with 100ms timeout at now=0
    let req_len = wire::encode_recv(&mut req, listener_handle.0, 100).unwrap();
    let outcome = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server, &mut recv_buf, 0);
    assert!(matches!(outcome, DispatchOutcome::Pending { .. }));

    // Advance time past deadline — no message ever arrives
    let mut fired_handle: Option<Handle> = None;
    let mut fired_len = 0usize;
    drive_pending(&mut server, 200, &mut recv_buf, &mut resp, |h, n| {
        fired_handle = Some(h);
        fired_len = n;
    });

    assert_eq!(fired_handle, Some(listener_handle));
    let header = wire::decode_response_header(&resp[..fired_len]).unwrap();
    assert!(!header.is_success());
    assert_eq!(header.response_code(), ResponseCode::TimedOut);
}

/// If a message arrives *before* `Recv` is dispatched, `dispatch_mctp_op`
/// returns `Reply` immediately without registering a pending entry.
#[test]
fn dispatch_recv_immediate_if_message_waiting() {
    let buf_a = RefCell::new(Vec::new());
    let sender_a = BufferSender { packets: &buf_a };
    let mut server_a: Server<_, 16> = Server::new(Eid(8), 0, sender_a);

    let buf_b = RefCell::new(Vec::new());
    let sender_b = BufferSender { packets: &buf_b };
    let mut server_b: Server<_, 16> = Server::new(Eid(42), 0, sender_b);

    let mut req = [0u8; 128];
    let mut resp = [0u8; 128];
    let mut recv_buf = [0u8; 255];

    // Register listener on A
    let req_len = wire::encode_listener(&mut req, 3).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    let listener_handle = h.handle;

    // B sends a message to A *before* Recv is dispatched
    let req_len = wire::encode_req(&mut req, 8).unwrap();
    let resp_len = dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
    let h = wire::decode_response_header(&resp[..resp_len]).unwrap();
    let req_handle = h.handle;

    let payload = b"early";
    let req_len =
        wire::encode_send(&mut req, Some(req_handle), 3, None, None, false, payload).unwrap();
    dispatch_reply(&req[..req_len], &mut resp, &mut server_b, &mut recv_buf);
    transfer(&buf_b, &mut server_a);

    // Recv dispatched after message is already in the router → Reply, not Pending
    let req_len = wire::encode_recv(&mut req, listener_handle, 1000).unwrap();
    let outcome = dispatch_mctp_op(&req[..req_len], &mut resp, &mut server_a, &mut recv_buf, 0);
    let n = match outcome {
        DispatchOutcome::Reply(n) => n,
        DispatchOutcome::Pending { .. } => panic!("expected Reply, got Pending"),
    };

    let header = wire::decode_response_header(&resp[..n]).unwrap();
    assert!(header.is_success());
    assert_eq!(header.msg_type, 3);
    assert_eq!(header.eid, 42);
    let recv_payload = wire::get_response_payload(&resp[..n], &header).unwrap();
    assert_eq!(recv_payload, payload);
}
