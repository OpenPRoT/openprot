// Licensed under the Apache-2.0 license

//! MCTP echo integration test.
//!
//! This test exercises the full MCTP server stack with a mock transport,
//! replicating the Hubris `mctp-echo` task behavior:
//!
//! 1. Server A (echo responder): listens for MCTP type-1 messages, echoes payload back
//! 2. Server B (requester): sends a request to A and verifies the echo response
//!
//! The test uses a **client/server partition**: the echo application logic
//! interacts exclusively through the `MctpClient` trait (client side), while
//! the `Server` + transport plumbing is the server side. This mirrors the
//! real deployment where the echo task is an IPC client of the MCTP server.

mod common;

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::{Handle, MctpClient};
use openprot_mctp_server::Server;

use common::{transfer, BufferSender, DirectClient};


// ---------------------------------------------------------------------------
// Echo application logic (client side — mirrors Hubris mctp-echo task)
// ---------------------------------------------------------------------------

/// Echo one message: receive on the listener, send the payload back.
///
/// This is the same logic as `hubris/task/mctp-echo/src/main.rs`:
/// ```ignore
/// let (_, _, msg, mut resp) = listener.recv(&mut recv_buf).unwrap_lite();
/// resp.send(msg).unwrap();
/// ```
/// but expressed through the `MctpClient` trait instead of Hubris IPC types.
fn echo_once(client: &impl MctpClient, listener_handle: Handle) {
    let mut recv_buf = [0u8; 255];
    let meta = client
        .recv(listener_handle, 0, &mut recv_buf)
        .expect("echo: should receive a message");

    let payload = &recv_buf[..meta.payload_size];
    client
        .send(
            None,
            meta.msg_type,
            Some(meta.remote_eid),
            Some(meta.msg_tag),
            false,
            payload,
        )
        .expect("echo: should send response");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// MCTP echo: send a request, receive it on a listener, echo back, verify.
///
/// This replicates the Hubris mctp-echo task behavior:
/// - EID 8 listens for MsgType(1) and echoes the payload
/// - EID 42 sends a request and checks the response matches
#[test]
fn mctp_echo_roundtrip() {
    // -- Server side: set up two MCTP server instances with mock transport --
    let buf_a = RefCell::new(Vec::new());
    let sender_a = BufferSender { packets: &buf_a };
    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, sender_a));

    let buf_b = RefCell::new(Vec::new());
    let sender_b = BufferSender { packets: &buf_b };
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, sender_b));

    // -- Client side: wrap servers in DirectClient to use MctpClient trait --
    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    // Client A: register listener for MsgType(1) — same as echo task
    let listener_handle = client_a.listener(1).unwrap();

    // Client B: get a request handle targeting EID 8
    let req_handle = client_b.req(8).unwrap();

    // Client B: send a request with MsgType(1)
    let payload = b"Hello MCTP echo!";
    let _tag = client_b
        .send(Some(req_handle), 1, None, None, false, payload)
        .unwrap();

    // Server side: transfer B's outbound packets to A
    transfer(&buf_b, &mut server_a.borrow_mut());

    // Client A: echo the message back (uses MctpClient trait)
    echo_once(&client_a, listener_handle);

    // Server side: transfer A's outbound packets to B
    transfer(&buf_a, &mut server_b.borrow_mut());

    // Client B: receive the echo response (uses MctpClient trait)
    let mut resp_buf = [0u8; 255];
    let resp_meta = client_b
        .recv(req_handle, 0, &mut resp_buf)
        .expect("Client B should have received the echo response");

    let response = &resp_buf[..resp_meta.payload_size];
    assert_eq!(response, payload, "Echo response should match original payload");
    assert_eq!(resp_meta.msg_type, 1);
    assert_eq!(resp_meta.remote_eid, 8);

    // Clean up
    client_a.drop_handle(listener_handle);
    client_b.drop_handle(req_handle);
}

/// Test that multiple messages can be echoed in sequence.
#[test]
fn mctp_echo_multiple() {
    let buf_a = RefCell::new(Vec::new());
    let sender_a = BufferSender { packets: &buf_a };
    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, sender_a));

    let buf_b = RefCell::new(Vec::new());
    let sender_b = BufferSender { packets: &buf_b };
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, sender_b));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let listener = client_a.listener(1).unwrap();
    let req = client_b.req(8).unwrap();

    for i in 0..5u8 {
        let msg = [i; 32];

        // Client B: send request
        client_b
            .send(Some(req), 1, None, None, false, &msg)
            .unwrap();
        transfer(&buf_b, &mut server_a.borrow_mut());
        buf_b.borrow_mut().clear();

        // Client A: echo (uses MctpClient trait)
        echo_once(&client_a, listener);
        transfer(&buf_a, &mut server_b.borrow_mut());
        buf_a.borrow_mut().clear();

        // Client B: verify echo response
        let mut resp_buf = [0u8; 255];
        let resp = client_b
            .recv(req, 0, &mut resp_buf)
            .unwrap_or_else(|_| panic!("iteration {i}: no response"));
        assert_eq!(&resp_buf[..resp.payload_size], &msg);
    }

    client_a.drop_handle(listener);
    client_b.drop_handle(req);
}
