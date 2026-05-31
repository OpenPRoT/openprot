// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Orchestrator Server
//!
//! The platform-independent orchestrator *service*: the server end of the
//! orchestrator IPC channel. External producers — command transports, the
//! watchdog, reset-detect IRQs — send requests to inject lifecycle [`Event`]s
//! or query state; this crate decodes those requests, drives the
//! [`StateMachine`], and produces responses.
//!
//! Like `openprot_mctp_server`, the I/O is abstracted out so the logic is host
//! testable: the request/response *codec* and the *dispatch* onto the state
//! machine are pure and syscall-free (see [`Request`], [`Response`], and
//! [`OrchestratorServer::dispatch`]). The target glue reads a request off a
//! `pw_kernel` channel, calls [`OrchestratorServer::handle_request`] to get the
//! response bytes, and writes them back — the same `channel_read` →
//! handle → `channel_respond` shape as `StreamServer::handle_ipc`, with the
//! syscalls kept in the target-only `openprot_orchestrator_ipc` crate.
//!
//! The server is generic over [`Actions`]: a real target supplies handlers that
//! call OpenPRoT services and HAL traits; host tests supply a scripted double,
//! exactly as the `sm` integration test does.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use openprot_orchestrator_sm::{Actions, Event, EventPort, State, StateMachine, UpdateScope};

// The request/response wire protocol is owned by the API crate so both the
// server and any client share one codec; re-export it for ergonomics.
pub use openprot_orchestrator_api::wire::{
    self, Request, Response, WireError, REQUEST_WIRE_SIZE, RESPONSE_WIRE_SIZE,
};

/// The orchestrator service.
///
/// Owns the [`StateMachine`] and the caller-supplied [`Actions`]. It is driven
/// one inbound request at a time via [`handle_request`](Self::handle_request)
/// (decode + dispatch + encode) or, if the caller has already decoded a
/// request, [`dispatch`](Self::dispatch). Follow-up events the machine emits
/// while servicing a request are processed before the response is produced, so
/// a single request runs the machine to its next quiescent (externally-waiting)
/// state.
pub struct OrchestratorServer<A: Actions> {
    machine: StateMachine,
    actions: A,
}

impl<A: Actions> OrchestratorServer<A> {
    /// Create a server with a fresh machine (parked in [`State::Boot`]) and the
    /// given [`Actions`].
    pub fn new(actions: A) -> Self {
        Self {
            machine: StateMachine::new(),
            actions,
        }
    }

    /// The machine's current state. For observability and testing.
    pub fn state(&self) -> State {
        self.machine.state()
    }

    /// Borrow the underlying [`Actions`], e.g. to inspect a test double.
    pub fn actions(&self) -> &A {
        &self.actions
    }

    /// Apply one decoded [`Request`] and run the machine to quiescence,
    /// returning the [`Response`].
    ///
    /// A request that injects an event drives the machine; any follow-up events
    /// the handlers emit are drained from `queue` and applied before returning,
    /// so the response reflects the state the machine settles in (the next
    /// state that waits for an external event). The `queue` is the same
    /// [`EventPort`] the machine pushes follow-ups into; on a real target it is
    /// the IPC-backed queue, in tests an in-memory one.
    ///
    /// Returns the queue's error only if draining a follow-up fails.
    pub fn dispatch<Q: EventPort>(
        &mut self,
        request: Request,
        queue: &mut Q,
    ) -> Result<Response, Q::Error> {
        match request {
            Request::Inject(event) => {
                let state = self.run_to_quiescence(event, queue)?;
                Ok(Response::State(state))
            }
            Request::QueryState => Ok(Response::State(self.machine.state())),
            // `Request` is `#[non_exhaustive]`; a request variant this server
            // does not understand is treated as a read-only no-op rather than
            // advancing the machine on a guessed meaning.
            _ => Ok(Response::State(self.machine.state())),
        }
    }

    /// Decode a request from `request_bytes`, [`dispatch`](Self::dispatch) it,
    /// and encode the response into `response_buf`.
    ///
    /// Returns the number of bytes written to `response_buf`. A request that
    /// fails to decode yields [`Response::Rejected`] rather than an error: an
    /// unknown or malformed request is reported back to the producer, not
    /// treated as a fatal channel fault (mirroring the transition table's
    /// "discard the event" stance). A queue failure while running the machine
    /// is surfaced as `Err`.
    pub fn handle_request<Q: EventPort>(
        &mut self,
        request_bytes: &[u8],
        response_buf: &mut [u8],
        queue: &mut Q,
    ) -> Result<usize, HandleError<Q::Error>> {
        let response = match wire::decode_request(request_bytes) {
            Ok(request) => self.dispatch(request, queue).map_err(HandleError::Queue)?,
            Err(_) => Response::Rejected,
        };
        wire::encode_response(response_buf, response)
            .map_err(|_| HandleError::ResponseBufferTooSmall)
    }

    /// Push `event` and step the machine until the queue drains, returning the
    /// settled state. Mirrors the drain loop in the `sm` integration test, but
    /// owned by the server so a target need only feed it requests.
    fn run_to_quiescence<Q: EventPort>(
        &mut self,
        event: Event,
        queue: &mut Q,
    ) -> Result<State, Q::Error> {
        let mut state = self.machine.step(event, queue, &mut self.actions)?;
        while let Some(follow_up) = queue.try_recv()? {
            state = self.machine.step(follow_up, queue, &mut self.actions)?;
        }
        Ok(state)
    }
}

/// Failure from [`OrchestratorServer::handle_request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HandleError<E> {
    /// The state machine's queue failed while running the request.
    Queue(E),
    /// The supplied response buffer was too small to hold the encoded response.
    ResponseBufferTooSmall,
}
