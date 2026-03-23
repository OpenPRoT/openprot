// Licensed under the Apache-2.0 license

//! SPDM over MCTP transport integration test.
//!
//! This test verifies the full SPDM request/response flow over MCTP transport
//! using mock MCTP infrastructure. It exercises:
//!
//! 1. Requester sends a request via `send_request`
//! 2. Responder receives the request via `receive_request`
//! 3. Responder sends a response via `send_response`
//! 4. Requester receives the response via `receive_response`

use std::cell::RefCell;

use mctp::{Eid, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use openprot_mctp_server::{Server, ServerConfig};
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use spdm_lib::codec::MessageBuf;
use spdm_lib::platform::transport::SpdmTransport;

// ---------------------------------------------------------------------------
// Mock transport (copied from mctp-server tests)
// ---------------------------------------------------------------------------

/// A mock sender that captures outbound packets in a buffer.
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
            let mut buf = [0u8; 255];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    self.packets.borrow_mut().push(p.to_vec());
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        255
    }
}

/// Transfer packets from one server's outbound buffer to another server.
fn transfer<S: Sender, const N: usize>(
    packets: &RefCell<Vec<Vec<u8>>>,
    dest: &mut Server<S, N>,
) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        dest.inbound(pkt).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Mock MCTP client (copied from mctp-server tests)
// ---------------------------------------------------------------------------

/// A direct (in-process) MCTP client that wraps a `Server` via `RefCell`.
///
/// This provides the `MctpClient` trait interface for testing without IPC.
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
// Tests
// ---------------------------------------------------------------------------

/// Test SPDM request/response roundtrip over MCTP transport.
///
/// This test:
/// 1. Creates two MCTP endpoints (requester EID 10, responder EID 20)
/// 2. Creates SPDM transports for each endpoint
/// 3. Sends a request buffer with pattern 0xAA
/// 4. Verifies the responder receives the same pattern
/// 5. Sends a response buffer with pattern 0x55
/// 6. Verifies the requester receives the same pattern
#[test]
fn spdm_mctp_request_response_roundtrip() {
    // Test configuration constants
    const REQUESTER_EID: u8 = 10;
    const RESPONDER_EID: u8 = 20;
    const REQUEST_PATTERN_SIZE: usize = 64;
    const RESPONSE_PATTERN_SIZE: usize = 128;
    const MESSAGE_BUFFER_SIZE: usize = 2048;

    // -- Setup MCTP infrastructure --

    // Requester MCTP server (EID 10)
    let buf_requester = RefCell::new(Vec::new());
    let sender_requester = BufferSender {
        packets: &buf_requester,
    };
    let server_requester: RefCell<Server<_, { ServerConfig::MAX_OUTSTANDING }>> =
        RefCell::new(Server::new(Eid(REQUESTER_EID), 0, sender_requester));

    // Responder MCTP server (EID 20)
    let buf_responder = RefCell::new(Vec::new());
    let sender_responder = BufferSender {
        packets: &buf_responder,
    };
    let server_responder: RefCell<Server<_, { ServerConfig::MAX_OUTSTANDING }>> =
        RefCell::new(Server::new(Eid(RESPONDER_EID), 0, sender_responder));

    // -- Setup SPDM transports --

    let client_requester = DirectClient::new(&server_requester);
    let mut transport_requester = MctpSpdmTransport::new_requester(client_requester, RESPONDER_EID);

    let client_responder = DirectClient::new(&server_responder);
    let mut transport_responder = MctpSpdmTransport::new_responder(client_responder);

    // Initialize transports
    transport_requester
        .init_sequence()
        .expect("Requester init should succeed");
    transport_responder
        .init_sequence()
        .expect("Responder init should succeed");

    // -- Phase 1: Send request, receive request --

    // Create request buffer with pattern 0xAA
    let request_pattern: [u8; REQUEST_PATTERN_SIZE] = [0xAA; REQUEST_PATTERN_SIZE];
    let mut request_backing = [0u8; MESSAGE_BUFFER_SIZE];
    let mut request_buf = MessageBuf::new(&mut request_backing);
    request_buf
        .put_data(request_pattern.len())
        .expect("Should allocate request buffer");
    request_buf
        .data_mut(request_pattern.len())
        .expect("Should get request buffer")
        .copy_from_slice(&request_pattern);

    // Requester sends request
    transport_requester
        .send_request(RESPONDER_EID, &mut request_buf)
        .expect("Should send request");

    // Transfer packets from requester to responder
    transfer(&buf_requester, &mut server_responder.borrow_mut());

    // Responder receives request
    let mut received_request_backing = [0u8; MESSAGE_BUFFER_SIZE];
    let mut received_request_buf = MessageBuf::new(&mut received_request_backing);
    transport_responder
        .receive_request(&mut received_request_buf)
        .expect("Should receive request");

    // Verify received request matches sent request
    let received_request_len = received_request_buf.data_len();
    assert_eq!(
        received_request_len,
        request_pattern.len(),
        "Request length mismatch"
    );
    let received_request_data = received_request_buf
        .data(received_request_len)
        .expect("Should get received request data");
    assert_eq!(
        received_request_data, &request_pattern,
        "Request pattern should match"
    );

    // -- Phase 2: Send response, receive response --

    // Create response buffer with pattern 0x55
    let response_pattern: [u8; RESPONSE_PATTERN_SIZE] = [0x55; RESPONSE_PATTERN_SIZE];
    let mut response_backing = [0u8; MESSAGE_BUFFER_SIZE];
    let mut response_buf = MessageBuf::new(&mut response_backing);
    response_buf
        .put_data(response_pattern.len())
        .expect("Should allocate response buffer");
    response_buf
        .data_mut(response_pattern.len())
        .expect("Should get response buffer")
        .copy_from_slice(&response_pattern);

    // Responder sends response
    transport_responder
        .send_response(&mut response_buf)
        .expect("Should send response");

    // Transfer packets from responder to requester
    buf_requester.borrow_mut().clear(); // Clear previous packets
    transfer(&buf_responder, &mut server_requester.borrow_mut());

    // Requester receives response
    let mut received_response_backing = [0u8; MESSAGE_BUFFER_SIZE];
    let mut received_response_buf = MessageBuf::new(&mut received_response_backing);
    transport_requester
        .receive_response(&mut received_response_buf)
        .expect("Should receive response");

    // Verify received response matches sent response
    let received_response_len = received_response_buf.data_len();
    assert_eq!(
        received_response_len,
        response_pattern.len(),
        "Response length mismatch"
    );
    let received_response_data = received_response_buf
        .data(received_response_len)
        .expect("Should get received response data");
    assert_eq!(
        received_response_data, &response_pattern,
        "Response pattern should match"
    );
}
