// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # PRoT Orchestrator State Machine — Runtime
//!
//! The event-driven run-loop that drives the [`State`] graph defined in
//! `openprot_lifecycle_api`. This is the Rust port of the body of
//! `AspeedStateMachine()`: a `loop { recv(); decide; run; }` over a blocking
//! event queue.
//!
//! ## Decoupling
//!
//! The loop is generic over two traits so it carries no OS, transport, or
//! hardware dependency — matching how the MCTP server keeps platform
//! primitives out of its core:
//!
//! - [`EventPort`] — the event source/sink the loop is generic over. In the
//!   request/response model (`OrchestratorServer`) it is a local in-process
//!   buffer for handler follow-up events; in free-running tests it is a
//!   `VecDeque`. There is no on-target blocking implementation: external events
//!   arrive as IPC requests, not through this trait.
//! - [`Actions`] — the work performed on entering a state (verify, recover,
//!   update, …). On a real target each method calls into OpenPRoT services and
//!   HAL traits (`Digest`, `Ecdsa`, the fwupdate service); in tests it is a
//!   scripted double. Handlers return the follow-up [`Event`] they want fed
//!   back into the loop — the same contract as `GenerateStateMachineEvent`.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use openprot_orchestrator_api::{transition, Event, State, UpdateScope};

/// Event source and sink the state machine loop is generic over.
///
/// In the request/response model, only `push` and `try_recv` are used: handlers
/// push follow-up events into an in-process buffer, which the server drains via
/// `try_recv` before replying. `recv` is available for free-running test loops
/// but has no on-target blocking implementation — external events arrive as IPC
/// requests, not through this trait.
pub trait EventPort {
    /// Error returned when the queue's source/sink fails (e.g. a dead IPC
    /// channel). An in-memory queue that cannot fail uses
    /// [`core::convert::Infallible`].
    type Error;

    /// Block until the next event is available and return it.
    ///
    /// Returns `Err` only on an unrecoverable source failure; transient
    /// conditions (a spurious wakeup, an undecodable message) are handled
    /// inside the implementation, which keeps blocking.
    fn recv(&mut self) -> Result<Event, Self::Error>;

    /// Enqueue an event to be processed later.
    ///
    /// Handlers use it to emit follow-up events, and external producers use it
    /// to inject events into the running machine. Returns `Err` if the sink
    /// cannot accept the event.
    fn push(&mut self, event: Event) -> Result<(), Self::Error>;

    /// Return the next already-available event without blocking, or `Ok(None)`
    /// if none is ready.
    ///
    /// This is the non-blocking counterpart to [`recv`](Self::recv): it returns
    /// events that have already been enqueued (notably handler follow-ups) but,
    /// unlike `recv`, never blocks waiting on the external source. A driver uses
    /// it to run the machine to quiescence — applying every queued follow-up —
    /// and then go back to blocking in `recv` for the next external event.
    fn try_recv(&mut self) -> Result<Option<Event>, Self::Error>;
}

/// Work performed by the state machine when it enters a state.
///
/// Each method runs the side-effecting work for one state and returns the
/// follow-up [`Event`] reporting its outcome, or `None` if the state has no
/// automatic follow-up (it waits for an externally injected event instead).
///
/// Splitting the work out of the loop this way keeps long-running crypto and
/// flash operations off the decision path and makes every handler independently
/// mockable on the host.
pub trait Actions {
    /// One-time platform/RoT initialization. → [`Event::InitDone`] etc.
    fn init(&mut self) -> Option<Event>;
    /// Recover the RoT itself (booted from secondary).
    fn rot_recovery(&mut self) -> Option<Event>;
    /// Authenticate the active platform firmware image.
    fn verify(&mut self) -> Option<Event>;
    /// Restore platform firmware from a known-good recovery image.
    fn recover(&mut self) -> Option<Event>;
    /// Apply a staged firmware update.
    fn update(&mut self) -> Option<Event>;

    /// Called on entering [`State::Lockdown`]. Default: no follow-up.
    fn enter_lockdown(&mut self) {}
    /// Called on entering [`State::Runtime`]. Default: no follow-up.
    fn enter_runtime(&mut self) {}
    /// Called on entering [`State::Reboot`]; should not return.
    fn reboot(&mut self) -> Option<Event> {
        None
    }
    /// Called on entering [`State::Unprovisioned`]. Default: no follow-up.
    fn enter_unprovisioned(&mut self) {}
}

/// Run the side-effecting work for `state` and return any follow-up event.
///
/// This is the dispatch from state → handler, factored out so it can be tested
/// directly and so the run-loop stays small.
fn run_state<A: Actions>(state: State, actions: &mut A) -> Option<Event> {
    match state {
        State::Init => actions.init(),
        State::RotRecovery => actions.rot_recovery(),
        State::FirmwareVerify => actions.verify(),
        State::FirmwareRecovery => actions.recover(),
        State::FirmwareUpdate => actions.update(),
        State::Reboot => actions.reboot(),
        State::Lockdown => {
            actions.enter_lockdown();
            None
        }
        State::Runtime => {
            actions.enter_runtime();
            None
        }
        State::Unprovisioned => {
            actions.enter_unprovisioned();
            None
        }
        // Boot performs no work; it only waits for Event::Start.
        State::Boot => None,
        // `State` is `#[non_exhaustive]`: a future variant added in the api
        // crate has no handler here yet, so it performs no automatic work and
        // simply waits for an externally injected event.
        _ => None,
    }
}

/// The lifecycle state machine: current state plus the run-loop.
pub struct StateMachine {
    state: State,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    /// Create a machine parked in [`State::Boot`].
    pub fn new() -> Self {
        Self { state: State::Boot }
    }

    /// The current state. Primarily for observability and testing.
    pub fn state(&self) -> State {
        self.state
    }

    /// Process exactly one event: apply the transition, and if the state
    /// changed, run the new state's handler and enqueue any follow-up event.
    ///
    /// Returns the state the machine is in after handling the event, or the
    /// queue's error if enqueuing a follow-up event failed. Exposed separately
    /// from [`run`](Self::run) so a single step can be asserted in tests, and so
    /// a target that wants to interleave other work can drive the machine
    /// cooperatively instead of surrendering its thread to `run`.
    pub fn step<Q: EventPort, A: Actions>(
        &mut self,
        event: Event,
        queue: &mut Q,
        actions: &mut A,
    ) -> Result<State, Q::Error> {
        if let Some(next) = transition(self.state, event) {
            self.state = next;
            if let Some(follow_up) = run_state(next, actions) {
                queue.push(follow_up)?;
            }
        }
        Ok(self.state)
    }

    /// Seed the machine with [`Event::Start`] and process events forever.
    ///
    /// This is the direct analogue of `AspeedStateMachine()`: enqueue `Start`,
    /// then block on the queue and step on each event. It returns only if the
    /// queue ever errors (a healthy queue blocks forever in `recv` on-target).
    pub fn run<Q: EventPort, A: Actions>(
        &mut self,
        queue: &mut Q,
        actions: &mut A,
    ) -> Result<core::convert::Infallible, Q::Error> {
        queue.push(Event::Start)?;
        loop {
            let event = queue.recv()?;
            self.step(event, queue, actions)?;
        }
    }
}
