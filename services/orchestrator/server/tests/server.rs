// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host integration tests for the orchestrator server.
//!
//! Drive the server with decoded requests (and raw bytes via `handle_request`)
//! against an in-memory [`EventPort`] and a scripted [`Actions`] double,
//! asserting the request -> machine -> response flow. No `pw_kernel`, no
//! hardware — the same host-test pattern as the `sm` integration test.

use std::collections::VecDeque;
use std::convert::Infallible;

use openprot_orchestrator_server::{
    wire, Actions, Event, EventPort, OrchestratorServer, Request, Response, State, UpdateScope,
};

/// In-memory queue standing in for the IPC-backed queue. `push`/`try_recv`
/// share one FIFO; `recv` is unused by the server (it drains via `try_recv`).
#[derive(Default)]
struct TestQueue {
    events: VecDeque<Event>,
}

impl EventPort for TestQueue {
    type Error = Infallible;

    fn recv(&mut self) -> Result<Event, Infallible> {
        Ok(self.events.pop_front().expect("queue drained unexpectedly"))
    }

    fn push(&mut self, event: Event) -> Result<(), Infallible> {
        self.events.push_back(event);
        Ok(())
    }

    fn try_recv(&mut self) -> Result<Option<Event>, Infallible> {
        Ok(self.events.pop_front())
    }
}

/// Scripted actions: each handler returns a pre-programmed follow-up event so a
/// single injected request runs the machine to its next quiescent state.
#[derive(Default)]
struct ScriptedActions {
    init: Option<Event>,
    verify: Option<Event>,
    recover: Option<Event>,
    update: Option<Event>,
}

impl Actions for ScriptedActions {
    fn init(&mut self) -> Option<Event> {
        self.init
    }
    fn rot_recovery(&mut self) -> Option<Event> {
        None
    }
    fn verify(&mut self) -> Option<Event> {
        self.verify
    }
    fn recover(&mut self) -> Option<Event> {
        self.recover
    }
    fn update(&mut self) -> Option<Event> {
        self.update
    }
}

#[test]
fn start_request_drives_boot_to_verify_wait() {
    // init auto-completes; verify parks (no follow-up). One Start request
    // should run Boot -> Init -> FirmwareVerify and settle there.
    let actions = ScriptedActions {
        init: Some(Event::InitDone),
        verify: None,
        ..Default::default()
    };
    let mut server = OrchestratorServer::new(actions);
    let mut q = TestQueue::default();

    let resp = server.dispatch(Request::Inject(Event::Start), &mut q).unwrap();
    assert_eq!(resp, Response::State(State::FirmwareVerify));
    assert_eq!(server.state(), State::FirmwareVerify);
}

#[test]
fn start_request_runs_all_the_way_to_runtime() {
    // Both init and verify auto-complete, so a single Start request settles the
    // machine in Runtime via the chain of follow-up events.
    let actions = ScriptedActions {
        init: Some(Event::InitDone),
        verify: Some(Event::VerifyDone),
        ..Default::default()
    };
    let mut server = OrchestratorServer::new(actions);
    let mut q = TestQueue::default();

    let resp = server.dispatch(Request::Inject(Event::Start), &mut q).unwrap();
    assert_eq!(resp, Response::State(State::Runtime));
}

#[test]
fn query_state_does_not_advance_the_machine() {
    let mut server = OrchestratorServer::new(ScriptedActions::default());
    let mut q = TestQueue::default();

    let resp = server.dispatch(Request::QueryState, &mut q).unwrap();
    assert_eq!(resp, Response::State(State::Boot));
    assert_eq!(server.state(), State::Boot);
}

#[test]
fn update_requested_from_runtime_routes_to_update() {
    // Get to Runtime first, then an external UpdateRequested should drive
    // Runtime -> FirmwareUpdate -> (UpdateDone) FirmwareVerify -> (VerifyDone)
    // Runtime.
    let actions = ScriptedActions {
        init: Some(Event::InitDone),
        verify: Some(Event::VerifyDone),
        update: Some(Event::UpdateDone(UpdateScope::Platform)),
        ..Default::default()
    };
    let mut server = OrchestratorServer::new(actions);
    let mut q = TestQueue::default();

    server.dispatch(Request::Inject(Event::Start), &mut q).unwrap();
    assert_eq!(server.state(), State::Runtime);

    let resp = server
        .dispatch(Request::Inject(Event::UpdateRequested), &mut q)
        .unwrap();
    assert_eq!(resp, Response::State(State::Runtime));
}

#[test]
fn handle_request_decodes_dispatches_and_encodes() {
    let actions = ScriptedActions {
        init: Some(Event::InitDone),
        verify: Some(Event::VerifyDone),
        ..Default::default()
    };
    let mut server = OrchestratorServer::new(actions);
    let mut q = TestQueue::default();

    // Encode a Start request the way a producer would.
    let mut req_buf = [0u8; 8];
    let req_len = wire::encode_request(&mut req_buf, Request::Inject(Event::Start)).unwrap();

    let mut resp_buf = [0u8; 8];
    let resp_len = server
        .handle_request(&req_buf[..req_len], &mut resp_buf, &mut q)
        .unwrap();

    assert_eq!(
        wire::decode_response(&resp_buf[..resp_len]),
        Ok(Response::State(State::Runtime))
    );
}

#[test]
fn malformed_request_is_rejected_not_fatal() {
    let mut server = OrchestratorServer::new(ScriptedActions::default());
    let mut q = TestQueue::default();

    let mut resp_buf = [0u8; 8];
    let resp_len = server
        .handle_request(&[0xFF, 0xFF], &mut resp_buf, &mut q)
        .unwrap();

    assert_eq!(
        wire::decode_response(&resp_buf[..resp_len]),
        Ok(Response::Rejected)
    );
    // The machine did not move.
    assert_eq!(server.state(), State::Boot);
}
