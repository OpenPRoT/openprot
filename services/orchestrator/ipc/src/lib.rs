// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Orchestrator IPC Server Binding
//!
//! The on-target binding that drives [`OrchestratorServer`] over a `pw_kernel` IPC
//! channel. This is the server (handler) end of the orchestrator channel: it reads
//! one request, runs it through the platform-independent service, and writes the
//! response back — the `channel_read` → handle → `channel_respond` shape of
//! `StreamServer::handle_ipc`.
//!
//! ## Layering
//!
//! - The wire codec lives in `openprot_orchestrator_api::wire` (host-tested there).
//! - The decode → drive → encode logic lives in
//!   `openprot_orchestrator_server::OrchestratorServer` (platform-independent and
//!   host-tested).
//! - This crate adds only the `pw_kernel` `userspace` syscalls
//!   (`channel_read`, `channel_respond`), which build on-target only. It is
//!   therefore target-only and carries no host tests; its logic is exercised by
//!   the host tests of the two crates above.
//!
//! ## Follow-up events
//!
//! While [`OrchestratorServer::handle_request`] runs the machine to quiescence, the
//! handlers' follow-up events ([`Event::VerifyDone`] etc.) are produced
//! in-process. They never cross the kernel: they are buffered in a small
//! in-process [`LocalQueue`] and drained within the same request. Only the
//! externally-meaningful request/response crosses the channel.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use openprot_ipc_event_queue::PendingQueue;
use openprot_orchestrator_server::{
    Actions, Event, EventPort, OrchestratorServer, State, REQUEST_WIRE_SIZE, RESPONSE_WIRE_SIZE,
};
use pw_status::Error as PwError;
use userspace::syscall;

/// Maximum number of un-drained follow-up events buffered in process.
///
/// A lifecycle handler emits at most one follow-up event per step, so a single
/// slot would suffice; the small headroom is defensive.
const PENDING_CAPACITY: usize = 4;

/// Bytes read for one inbound request. A [`Request`](openprot_lifecycle_server::Request)
/// encodes to [`REQUEST_WIRE_SIZE`]; the headroom tolerates a future, larger
/// encoding without re-tuning callers.
const REQUEST_BUFFER_LEN: usize = 16;

/// In-process queue for the state machine's follow-up events.
///
/// [`OrchestratorServer`] drives the machine to quiescence within a single
/// request, draining follow-ups via [`EventPort::try_recv`]. Those follow-ups
/// are produced locally and never cross the channel, so this queue is a plain
/// in-memory buffer. [`recv`](EventPort::recv) — the *blocking* source — is
/// never called in the request/response model and returns
/// [`PwError::Unavailable`] if invoked, rather than blocking with no producer.
#[derive(Default)]
pub struct LocalQueue {
    pending: PendingQueue<Event, PENDING_CAPACITY>,
}

impl LocalQueue {
    /// Create an empty queue.
    pub const fn new() -> Self {
        Self {
            pending: PendingQueue::new(),
        }
    }
}

impl EventPort for LocalQueue {
    type Error = PwError;

    fn recv(&mut self) -> Result<Event, PwError> {
        // The request/response server never blocks on an event source; it drives
        // the machine synchronously per request. A blocking recv here would have
        // no producer to wake it.
        Err(PwError::Unavailable)
    }

    fn push(&mut self, event: Event) -> Result<(), PwError> {
        self.pending
            .push(event)
            .map_err(|_| PwError::ResourceExhausted)
    }

    fn try_recv(&mut self) -> Result<Option<Event>, PwError> {
        Ok(self.pending.pop())
    }
}

/// Drives [`OrchestratorServer`] over a `pw_kernel` IPC channel.
///
/// Construct it with the orchestrator task's inbound channel handle and an
/// [`Actions`] implementation, then call [`serve_once`](Self::serve_once) in the
/// task loop: each call reads one request, runs the machine, and replies.
pub struct OrchestratorChannelServer<A: Actions> {
    /// Channel handle to read requests from and respond on.
    ipc_handle: u32,
    /// The platform-independent orchestrator service.
    server: OrchestratorServer<A>,
    /// In-process follow-up buffer for the machine.
    queue: LocalQueue,
}

impl<A: Actions> OrchestratorChannelServer<A> {
    /// Create a server bound to `ipc_handle`, driving the given [`Actions`].
    ///
    /// The handle is the codegen constant for the orchestrator task's inbound
    /// channel (e.g. `orchestrator_codegen::handle::ORCHESTRATOR_IPC_HANDLER`).
    pub fn new(ipc_handle: u32, actions: A) -> Self {
        Self {
            ipc_handle,
            server: OrchestratorServer::new(actions),
            queue: LocalQueue::new(),
        }
    }

    /// The machine's current state. For observability and testing.
    pub fn state(&self) -> State {
        self.server.state()
    }

    /// Service exactly one inbound request.
    ///
    /// Reads a request off the channel, runs it through [`OrchestratorServer`]
    /// (decoding, driving the machine to quiescence, encoding the settled
    /// state), and writes the response back — the `channel_read` →
    /// `handle_request` → `channel_respond` cycle.
    ///
    /// Returns `Err` on a syscall failure or a too-small response buffer; a
    /// merely *undecodable* request is not an error — the server replies with a
    /// rejection, mirroring the transition table's "discard the event" stance.
    pub fn serve_once(&mut self) -> Result<(), PwError> {
        let mut request_buf = [0u8; REQUEST_BUFFER_LEN];
        let request_len = syscall::channel_read(self.ipc_handle, 0, &mut request_buf[..])?;

        let mut response_buf = [0u8; RESPONSE_WIRE_SIZE];
        let response_len = self
            .server
            .handle_request(&request_buf[..request_len], &mut response_buf, &mut self.queue)
            .map_err(handle_error_to_status)?;

        syscall::channel_respond(self.ipc_handle, &response_buf[..response_len])
    }
}

/// Map a [`OrchestratorServer`] handle error onto a `pw_status` error for the
/// channel reply path.
fn handle_error_to_status(error: openprot_orchestrator_server::HandleError<PwError>) -> PwError {
    use openprot_orchestrator_server::HandleError;
    match error {
        // The queue is the in-process LocalQueue; propagate its error verbatim.
        HandleError::Queue(err) => err,
        HandleError::ResponseBufferTooSmall => PwError::OutOfRange,
        // `HandleError` is `#[non_exhaustive]`; report an unrecognized future
        // variant as an internal fault rather than guessing a mapping.
        _ => PwError::Internal,
    }
}

// Fail to compile if the encoded request ever outgrows the read buffer.
const _: () = assert!(REQUEST_WIRE_SIZE <= REQUEST_BUFFER_LEN);
