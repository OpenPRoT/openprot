// Licensed under the Apache-2.0 license

//! MCTP loopback transport integration test.
//!
//! Verifies that two MCTP servers can communicate through the loopback
//! transport without any physical transport encoding.

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use openprot_mctp_server::Server;
use openprot_mctp_transport_loopback::{LoopbackPair, LoopbackSender};

// ---------------------------------------------------------------------------
// Client-side wrapper (same as echo test)
// ---------------------------------------------------------------------------

struct DirectClient<'a, S: mctp_lib::Sender, const N: usize> {
    server: &'a RefCell<Server<S, N>>,
}

impl<'a, S: mctp_lib::Sender, const N: usize> DirectClient<'a, S, N> {
    fn new(server: &'a RefCell<Server<S, N>>) -> Self {
        Self { server }
    }
}

impl<S: mctp_lib::Sender, const N: usize> MctpClient for DirectClient<'_, S, N> {
    fn req(&self, eid: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().req(eid)
    }

    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().listener(msg_type)
    }

    fn get_eid(&self) -> u8 {
        self.server.borrow().get_eid()
    }

    fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        self.server.borrow_mut().set_eid(eid)
    }

    fn recv(
        &self,
        handle: Handle,
        _timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError> {
        self.server
            .borrow_mut()
            .try_recv(handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))
    }

    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError> {
        self.server
            .borrow_mut()
            .send(handle, msg_type, eid, tag, integrity_check, buf)
    }

    fn drop_handle(&self, handle: Handle) {
        let _ = self.server.borrow_mut().unbind(handle);
    }
}

// ---------------------------------------------------------------------------
// Echo application logic (same as echo.rs)
// ---------------------------------------------------------------------------

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

/// Basic loopback: send a message from A to B and verify receipt.
#[test]
fn loopback_simple_send() {
    let pair = LoopbackPair::new();
    let sender_a = LoopbackSender::new(&pair.a_to_b);
    let sender_b = LoopbackSender::new(&pair.b_to_a);

    let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));
    let server_b: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(42), 0, sender_b));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    // B registers a listener for type 1
    let listener_b = client_b.listener(1).unwrap();

    // A gets a request handle to send to EID 42
    let req_a = client_a.req(42).unwrap();

    // A sends a message
    let payload = b"Hello from A!";
    client_a
        .send(Some(req_a), 1, None, None, false, payload)
        .unwrap();

    // Transfer packets A→B
    while let Some(pkt) = pair.pop_a_to_b() {
        server_b.borrow_mut().inbound(&pkt).unwrap();
    }

    // B receives the message
    let mut recv_buf = [0u8; 255];
    let meta = client_b
        .recv(listener_b, 0, &mut recv_buf)
        .expect("B should receive message");

    let received = &recv_buf[..meta.payload_size];
    assert_eq!(received, payload);
    assert_eq!(meta.msg_type, 1);
    assert_eq!(meta.remote_eid, 8);

    client_a.drop_handle(req_a);
    client_b.drop_handle(listener_b);
}

/// MCTP echo roundtrip through loopback transport.
///
/// Server A listens for type-1 messages and echoes them.
/// Server B sends a request and verifies the echo response.
#[test]
fn loopback_echo_roundtrip() {
    let pair = LoopbackPair::new();
    let sender_a = LoopbackSender::new(&pair.a_to_b);
    let sender_b = LoopbackSender::new(&pair.b_to_a);

    let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));
    let server_b: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(42), 0, sender_b));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    // A registers listener for type 1 (echo responder)
    let listener_a = client_a.listener(1).unwrap();

    // B gets request handle to send to EID 8
    let req_b = client_b.req(8).unwrap();

    // B sends a request
    let payload = b"Hello MCTP loopback!";
    let _tag = client_b
        .send(Some(req_b), 1, None, None, false, payload)
        .unwrap();

    // Transfer B→A
    while let Some(pkt) = pair.pop_b_to_a() {
        server_a.borrow_mut().inbound(&pkt).unwrap();
    }

    // A echoes the message
    echo_once(&client_a, listener_a);

    // Transfer A→B
    while let Some(pkt) = pair.pop_a_to_b() {
        server_b.borrow_mut().inbound(&pkt).unwrap();
    }

    // B receives the echo response
    let mut resp_buf = [0u8; 255];
    let resp_meta = client_b
        .recv(req_b, 0, &mut resp_buf)
        .expect("B should receive echo response");

    let response = &resp_buf[..resp_meta.payload_size];
    assert_eq!(response, payload, "Echo response should match original");
    assert_eq!(resp_meta.msg_type, 1);
    assert_eq!(resp_meta.remote_eid, 8);

    client_a.drop_handle(listener_a);
    client_b.drop_handle(req_b);
}

/// Multiple echo roundtrips.
#[test]
fn loopback_echo_multiple() {
    let pair = LoopbackPair::new();
    let sender_a = LoopbackSender::new(&pair.a_to_b);
    let sender_b = LoopbackSender::new(&pair.b_to_a);

    let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));
    let server_b: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(42), 0, sender_b));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let listener = client_a.listener(1).unwrap();
    let req = client_b.req(8).unwrap();

    for i in 0..5u8 {
        let msg = [i; 32];

        // B sends request
        client_b
            .send(Some(req), 1, None, None, false, &msg)
            .unwrap();
        while let Some(pkt) = pair.pop_b_to_a() {
            server_a.borrow_mut().inbound(&pkt).unwrap();
        }

        // A echoes
        echo_once(&client_a, listener);
        while let Some(pkt) = pair.pop_a_to_b() {
            server_b.borrow_mut().inbound(&pkt).unwrap();
        }

        // B verifies echo
        let mut resp_buf = [0u8; 255];
        let resp = client_b
            .recv(req, 0, &mut resp_buf)
            .unwrap_or_else(|_| panic!("iteration {i}: no response"));
        assert_eq!(&resp_buf[..resp.payload_size], &msg);
    }

    client_a.drop_handle(listener);
    client_b.drop_handle(req);
}
