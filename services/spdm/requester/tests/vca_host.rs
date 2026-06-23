// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM VCA (Version, Capabilities, Algorithms) host integration test.
//!
//! Exercises the full SPDM requester ↔ responder flow using the same
//! in-memory transport pattern as the MCTP echo tests. No IPC, no QEMU —
//! runs entirely on the host.
//!
//! This test demonstrates how to use the `SpdmRequester` and `SpdmResponder`
//! APIs with custom capabilities passed via configuration.
//!
//! ## Architecture
//!
//! ```text
//! ┌─ Requester (EID 8) ────────┐      ┌─ Responder (EID 42) ───────┐
//! │ Stack<DirectClient>        │      │ Stack<DirectClient>        │
//! │   └→ MctpSpdmTransport     │      │   └→ MctpSpdmTransport     │
//! │       └→ SpdmRequester     │      │       └→ SpdmResponder     │
//! └────────────────────────────┘      └────────────────────────────┘
//!              │                                   │
//!              └──── BufferSender ←─ transfer() ───┘
//! ```

mod common;

use std::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::stack::Stack;
#[allow(unused_imports)]
use openprot_mctp_api::MctpClient;
use openprot_mctp_server::Server;
use openprot_spdm_requester::{RequesterConfig, SpdmRequester};
use openprot_spdm_responder::{ResponderConfig, SpdmResponder};
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use spdm_lib::codec::MessageBuf;
use spdm_lib::commands::algorithms::request::generate_negotiate_algorithms_request;
use spdm_lib::commands::capabilities::request::generate_capabilities_request_local;
use spdm_lib::commands::version::request::generate_get_version;
use spdm_lib::commands::version::VersionReqPayload;
use spdm_lib::platform::transport::SpdmTransport;

use common::{
    transfer, BufferSender, DemoPeerCertStore, DirectClient, MockCertStore, MockEvidence, MockHash,
    MockRng,
};

/// Requester EID
const REQUESTER_EID: u8 = 8;

/// Responder EID
const RESPONDER_EID: u8 = 42;

/// Sanity check: verify the MCTP layer works before testing SPDM.
#[test]
fn mctp_sanity_check() {
    use openprot_mctp_api::{MctpListener, MctpReqChannel};

    let buf_a = RefCell::new(Vec::new());
    let buf_b = RefCell::new(Vec::new());

    let server_a: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(8), 0, BufferSender { packets: &buf_a }));
    let server_b: RefCell<Server<_, 16>> =
        RefCell::new(Server::new(Eid(42), 0, BufferSender { packets: &buf_b }));

    let stack_a = Stack::new(DirectClient::new(&server_a));
    let stack_b = Stack::new(DirectClient::new(&server_b));

    // A registers listener FIRST (so router retains matching messages)
    let mut listener = stack_a.listener(0x05, 0).expect("listener");

    // B sends a request to A
    let mut req = stack_b.req(8, 0).expect("req channel alloc");
    req.send(0x05, b"hello SPDM").expect("req send");

    // Transfer B -> A
    transfer(&buf_b, &mut server_a.borrow_mut());

    // A receives
    let mut recv_buf = [0u8; 256];
    let (meta, payload, _resp) = listener.recv(&mut recv_buf).expect("listener recv");

    assert_eq!(payload, b"hello SPDM");
    assert_eq!(meta.msg_type, 0x05);
    eprintln!("MCTP sanity check passed!");
}

/// SPDM VCA (Version, Capabilities, Algorithms) roundtrip test.
///
/// This test exercises the full VCA flow between a requester and responder,
/// using in-memory MCTP transport with no platform dependencies.
///
/// Demonstrates usage of `SpdmRequester` and `SpdmResponder` APIs with
/// custom capabilities passed via `RequesterConfig` and `ResponderConfig`.
#[test]
fn spdm_vca_roundtrip() {
    // -- Set up MCTP servers with in-memory transport --
    let buf_req = RefCell::new(Vec::new());
    let buf_resp = RefCell::new(Vec::new());

    let server_req: RefCell<Server<_, 16>> = RefCell::new(Server::new(
        Eid(REQUESTER_EID),
        0,
        BufferSender { packets: &buf_req },
    ));
    let server_resp: RefCell<Server<_, 16>> = RefCell::new(Server::new(
        Eid(RESPONDER_EID),
        0,
        BufferSender { packets: &buf_resp },
    ));

    // -- Create Stack facades (same API as production) --
    let stack_req = Stack::new(DirectClient::new(&server_req));
    let stack_resp = Stack::new(DirectClient::new(&server_resp));

    // Set EIDs on stacks
    stack_req.set_eid(REQUESTER_EID).expect("set requester EID");
    stack_resp
        .set_eid(RESPONDER_EID)
        .expect("set responder EID");

    // -- Create SPDM transports --
    let mut transport_req = MctpSpdmTransport::new_requester(&stack_req, RESPONDER_EID);
    let mut transport_resp = MctpSpdmTransport::new_responder(&stack_resp);

    // Initialize transports
    transport_req
        .init_sequence()
        .expect("requester transport init");
    transport_resp
        .init_sequence()
        .expect("responder transport init");

    // -- Create mock platform implementations --
    // Requester side
    let mut req_cert_store = MockCertStore::new();
    let mut req_hash = MockHash::new();
    let mut req_m1_hash = MockHash::new();
    let mut req_l1_hash = MockHash::new();
    let mut req_rng = MockRng::new();
    let req_evidence = MockEvidence::new();
    let mut req_peer_cert_store = DemoPeerCertStore::new();

    // Responder side
    let mut resp_cert_store = MockCertStore::new();
    let mut resp_hash = MockHash::new();
    let mut resp_m1_hash = MockHash::new();
    let mut resp_l1_hash = MockHash::new();
    let mut resp_rng = MockRng::new();
    let resp_evidence = MockEvidence::new();

    // -- Create SPDM requester and responder using library APIs --
    // This demonstrates how to use the SpdmRequester/SpdmResponder with
    // custom capabilities and algorithms passed via configuration.
    //
    // Pattern: Get defaults, modify as needed, pass to new()
    // Here we just use the defaults which apply DEFAULT_DTS and DEFAULT_SMS.
    let req_caps = RequesterConfig::default_capabilities();
    let req_algos = RequesterConfig::default_algorithms();

    let requester_config = RequesterConfig {
        capabilities: Some(req_caps),
        algorithms: Some(req_algos),
    };

    let resp_caps = ResponderConfig::default_capabilities();
    let resp_algos = ResponderConfig::default_algorithms();

    let responder_config = ResponderConfig {
        capabilities: Some(resp_caps),
        algorithms: Some(resp_algos),
    };

    let mut requester = SpdmRequester::new(
        &mut transport_req,
        &mut req_cert_store,
        &mut req_peer_cert_store,
        &mut req_hash,
        &mut req_m1_hash,
        &mut req_l1_hash,
        &mut req_rng,
        &req_evidence,
        Some(requester_config),
    )
    .expect("requester creation");

    let mut responder = SpdmResponder::new(
        &mut transport_resp,
        &mut resp_cert_store,
        &mut resp_hash,
        &mut resp_m1_hash,
        &mut resp_l1_hash,
        &mut resp_rng,
        &resp_evidence,
        Some(responder_config),
    )
    .expect("responder creation");

    // -- Message buffers (one per context, reused via reset) --
    let mut req_buf_storage = [0u8; 4096];
    let mut resp_buf_storage = [0u8; 4096];
    let mut req_buf = MessageBuf::new(&mut req_buf_storage);
    let mut resp_buf = MessageBuf::new(&mut resp_buf_storage);

    // ══════════════════════════════════════════════════════════════════════
    // Step 1: GET_VERSION → VERSION
    // ══════════════════════════════════════════════════════════════════════

    // Requester: generate and send GET_VERSION
    generate_get_version(
        requester.context_mut(),
        &mut req_buf,
        VersionReqPayload::new(0, 0),
    )
    .expect("generate GET_VERSION");

    requester
        .context_mut()
        .requester_send_request(&mut req_buf, RESPONDER_EID)
        .expect("send GET_VERSION");

    // Transfer: requester → responder
    transfer(&buf_req, &mut server_resp.borrow_mut());
    buf_req.borrow_mut().clear();

    // Responder: process request and send VERSION response
    responder
        .context_mut()
        .responder_process_message(&mut resp_buf)
        .expect("responder process GET_VERSION");

    // Transfer: responder → requester
    transfer(&buf_resp, &mut server_req.borrow_mut());
    buf_resp.borrow_mut().clear();

    // Requester: process VERSION response
    req_buf.reset();
    requester
        .context_mut()
        .requester_process_message(&mut req_buf)
        .expect("requester process VERSION");

    // ══════════════════════════════════════════════════════════════════════
    // Step 2: GET_CAPABILITIES → CAPABILITIES
    // ══════════════════════════════════════════════════════════════════════

    // Requester: generate and send GET_CAPABILITIES
    req_buf.reset();
    generate_capabilities_request_local(requester.context_mut(), &mut req_buf)
        .expect("generate GET_CAPABILITIES");
    requester
        .context_mut()
        .requester_send_request(&mut req_buf, RESPONDER_EID)
        .expect("send GET_CAPABILITIES");

    // Transfer: requester → responder
    transfer(&buf_req, &mut server_resp.borrow_mut());
    buf_req.borrow_mut().clear();

    // Responder: process request and send CAPABILITIES response
    resp_buf.reset();
    responder
        .context_mut()
        .responder_process_message(&mut resp_buf)
        .expect("responder process GET_CAPABILITIES");

    // Transfer: responder → requester
    transfer(&buf_resp, &mut server_req.borrow_mut());
    buf_resp.borrow_mut().clear();

    // Requester: process CAPABILITIES response
    req_buf.reset();
    requester
        .context_mut()
        .requester_process_message(&mut req_buf)
        .expect("requester process CAPABILITIES");

    // ══════════════════════════════════════════════════════════════════════
    // Step 3: NEGOTIATE_ALGORITHMS → ALGORITHMS
    // ══════════════════════════════════════════════════════════════════════

    // Requester: generate and send NEGOTIATE_ALGORITHMS
    req_buf.reset();
    generate_negotiate_algorithms_request(
        requester.context_mut(),
        &mut req_buf,
        None,
        None,
        None,
        None,
    )
    .expect("generate NEGOTIATE_ALGORITHMS");
    requester
        .context_mut()
        .requester_send_request(&mut req_buf, RESPONDER_EID)
        .expect("send NEGOTIATE_ALGORITHMS");

    // Transfer: requester → responder
    transfer(&buf_req, &mut server_resp.borrow_mut());
    buf_req.borrow_mut().clear();

    // Responder: process request and send ALGORITHMS response
    resp_buf.reset();
    responder
        .context_mut()
        .responder_process_message(&mut resp_buf)
        .expect("responder process NEGOTIATE_ALGORITHMS");

    // Transfer: responder → requester
    transfer(&buf_resp, &mut server_req.borrow_mut());
    buf_resp.borrow_mut().clear();

    // Requester: process ALGORITHMS response
    req_buf.reset();
    requester
        .context_mut()
        .requester_process_message(&mut req_buf)
        .expect("requester process ALGORITHMS");

    // VCA flow completed successfully!
}
