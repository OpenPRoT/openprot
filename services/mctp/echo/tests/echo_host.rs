// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host integration test for the reusable echo crate.
//!
//! Exercises the echo path via Stack + MctpClient against two in-memory
//! server instances. This validates that openprot_mctp_echo helpers can be
//! used in a host-only environment without IPC/I2C transport.

use core::cell::RefCell;

use mctp::{Eid, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{Handle, MctpClient, MctpError, MctpReqChannel, RecvMetadata, Stack};
use openprot_mctp_echo::{echo_once, prepare_listener, ECHO_MSG_TYPE};
use openprot_mctp_server::Server;

/// MTU for MCTP payload (without header)
const MCTP_MTU: usize = 255;
/// MCTP header size (4 bytes)
const MCTP_HEADER_SIZE: usize = 4;

struct BufferSender<'a> {
    packets: &'a RefCell<Vec<Vec<u8>>>,
}

impl Sender for BufferSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<Tag> {
        loop {
            // Buffer must be MTU + header size
            let mut buf = [0u8; MCTP_MTU + MCTP_HEADER_SIZE];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => self.packets.borrow_mut().push(p.to_vec()),
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_MTU
    }
}

fn transfer<S: Sender, const N: usize>(packets: &RefCell<Vec<Vec<u8>>>, dest: &mut Server<S, N>) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        dest.inbound(pkt).expect("inbound should accept packet");
    }
}

struct DirectClient<'a, S: Sender, const N: usize> {
    server: &'a RefCell<Server<S, N>>,
}

impl<'a, S: Sender, const N: usize> DirectClient<'a, S, N> {
    fn new(server: &'a RefCell<Server<S, N>>) -> Self {
        Self { server }
    }
}

impl<S: Sender, const N: usize> MctpClient for DirectClient<'_, S, N> {
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
            .ok_or(MctpError::from_code(
                openprot_mctp_api::ResponseCode::TimedOut,
            ))
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

#[test]
fn echo_path_roundtrip_via_stack_and_echo_helper() {
    let buf_a = RefCell::new(Vec::new());
    let sender_a = BufferSender { packets: &buf_a };
    let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));

    let buf_b = RefCell::new(Vec::new());
    let sender_b = BufferSender { packets: &buf_b };
    let server_b: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(42), 0, sender_b));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    let stack_a = Stack::new(client_a);
    let stack_b = Stack::new(client_b);

    let mut listener_a = prepare_listener(&stack_a).expect("listener setup should succeed");

    let mut req_b = stack_b.req(8, 0).expect("request channel should open");
    let payload = b"echo from host test";
    req_b
        .send(ECHO_MSG_TYPE, payload)
        .expect("request send should succeed");

    // Deliver request packets from B -> A, run one echo step, then A -> B.
    transfer(&buf_b, &mut server_a.borrow_mut());
    buf_b.borrow_mut().clear();

    let mut echo_buf = [0u8; 255];
    echo_once(&mut listener_a, &mut echo_buf).expect("echo_once should receive and reply");

    transfer(&buf_a, &mut server_b.borrow_mut());
    buf_a.borrow_mut().clear();

    let mut resp_buf = [0u8; 255];
    let (meta, response) = req_b.recv(&mut resp_buf).expect("response should arrive");
    assert_eq!(meta.msg_type, ECHO_MSG_TYPE);
    assert_eq!(meta.remote_eid, 8);
    assert_eq!(response, payload);
}
