// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-side integration tests for the orchestrator state machine.
//!
//! These exercise the full run-loop with an in-memory [`EventPort`] and a
//! scripted [`Actions`] double — no `pw_kernel`, no hardware. This is the
//! OpenPRoT pattern of running services as plain host unit tests, and it is
//! what makes porting the ASPEED state graph low-risk: the entire boot/verify/
//! recovery flow is asserted in software.

use std::collections::VecDeque;
use std::convert::Infallible;

use openprot_orchestrator_sm::{Actions, Event, EventPort, State, StateMachine, UpdateScope};

/// In-memory queue standing in for the `pw_kernel` IPC channel.
///
/// It never fails, so its [`EventPort::Error`] is [`Infallible`]; the run-loop
/// `?`-propagation is therefore a no-op here.
#[derive(Default)]
struct TestQueue {
    events: VecDeque<Event>,
}

impl EventPort for TestQueue {
    type Error = Infallible;

    fn recv(&mut self) -> Result<Event, Infallible> {
        // In tests we never block: a missing event means the scripted flow is
        // complete, so panic loudly rather than hang.
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

impl TestQueue {
    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Scripted actions: each handler returns a pre-programmed follow-up event and
/// records that it ran, so tests can assert both the path taken and the
/// resulting state.
#[derive(Default)]
struct ScriptedActions {
    init: Option<Event>,
    verify: Option<Event>,
    recover: Option<Event>,
    update: Option<Event>,
    trace: Vec<State>,
}

impl Actions for ScriptedActions {
    fn init(&mut self) -> Option<Event> {
        self.trace.push(State::Init);
        self.init
    }
    fn rot_recovery(&mut self) -> Option<Event> {
        self.trace.push(State::RotRecovery);
        None
    }
    fn verify(&mut self) -> Option<Event> {
        self.trace.push(State::FirmwareVerify);
        self.verify
    }
    fn recover(&mut self) -> Option<Event> {
        self.trace.push(State::FirmwareRecovery);
        self.recover
    }
    fn update(&mut self) -> Option<Event> {
        self.trace.push(State::FirmwareUpdate);
        self.update
    }
    fn enter_runtime(&mut self) {
        self.trace.push(State::Runtime);
    }
    fn enter_lockdown(&mut self) {
        self.trace.push(State::Lockdown);
    }
}

/// Drive the machine by hand (no infinite `run`) until the queue empties,
/// returning the final state. This mirrors what `run` does step-by-step but
/// terminates, which is what a test needs.
fn drive(actions: &mut ScriptedActions) -> (State, TestQueue) {
    let mut sm = StateMachine::new();
    let mut q = TestQueue::default();
    q.push(Event::Start).unwrap();
    while !q.is_empty() {
        let ev = q.recv().unwrap();
        sm.step(ev, &mut q, actions).unwrap();
    }
    (sm.state(), q)
}

#[test]
fn happy_path_boots_to_runtime() {
    let mut actions = ScriptedActions {
        init: Some(Event::InitDone),
        verify: Some(Event::VerifyDone),
        ..Default::default()
    };
    let (final_state, _q) = drive(&mut actions);

    assert_eq!(final_state, State::Runtime);
    assert_eq!(
        actions.trace,
        vec![
            State::Init,
            State::FirmwareVerify,
            State::Runtime,
        ]
    );
}

#[test]
fn verify_failure_recovers_then_revalidates() {
    // verify fails → recovery succeeds → re-verify succeeds → runtime.
    // The second verify must pass, so we flip the scripted result via a
    // stateful handler.
    #[derive(Default)]
    struct RecoverThenPass {
        verify_calls: u32,
        trace: Vec<State>,
    }
    impl Actions for RecoverThenPass {
        fn init(&mut self) -> Option<Event> {
            self.trace.push(State::Init);
            Some(Event::InitDone)
        }
        fn rot_recovery(&mut self) -> Option<Event> {
            None
        }
        fn verify(&mut self) -> Option<Event> {
            self.trace.push(State::FirmwareVerify);
            self.verify_calls += 1;
            if self.verify_calls == 1 {
                Some(Event::VerifyFailed)
            } else {
                Some(Event::VerifyDone)
            }
        }
        fn recover(&mut self) -> Option<Event> {
            self.trace.push(State::FirmwareRecovery);
            Some(Event::RecoveryDone)
        }
        fn update(&mut self) -> Option<Event> {
            None
        }
        fn enter_runtime(&mut self) {
            self.trace.push(State::Runtime);
        }
    }

    let mut sm = StateMachine::new();
    let mut q = TestQueue::default();
    let mut actions = RecoverThenPass::default();
    q.push(Event::Start).unwrap();
    while !q.is_empty() {
        let ev = q.recv().unwrap();
        sm.step(ev, &mut q, &mut actions).unwrap();
    }

    assert_eq!(sm.state(), State::Runtime);
    assert_eq!(
        actions.trace,
        vec![
            State::Init,
            State::FirmwareVerify,   // fails
            State::FirmwareRecovery, // succeeds
            State::FirmwareVerify,   // re-verify passes
            State::Runtime,
        ]
    );
}

#[test]
fn unrecoverable_failure_locks_down() {
    let mut actions = ScriptedActions {
        init: Some(Event::InitDone),
        verify: Some(Event::VerifyFailed),
        recover: Some(Event::RecoveryFailed),
        ..Default::default()
    };
    let (final_state, q) = drive(&mut actions);

    assert_eq!(final_state, State::Lockdown);
    assert!(q.is_empty(), "lockdown is terminal: no follow-up events");
    assert_eq!(
        actions.trace,
        vec![
            State::Init,
            State::FirmwareVerify,
            State::FirmwareRecovery,
            State::Lockdown,
        ]
    );
}

#[test]
fn update_on_reset_then_runtime() {
    // Models update-on-reset: the first time we reach FirmwareVerify an update
    // intent is pending, so verify must *not* auto-complete — the external
    // UpdateRequested drives the branch. After the update, the re-verify passes.
    #[derive(Default)]
    struct UpdateOnReset {
        verify_calls: u32,
    }
    impl Actions for UpdateOnReset {
        fn init(&mut self) -> Option<Event> {
            Some(Event::InitDone)
        }
        fn rot_recovery(&mut self) -> Option<Event> {
            None
        }
        fn verify(&mut self) -> Option<Event> {
            self.verify_calls += 1;
            // First entry: wait for the external update intent (no follow-up).
            // After the update: verification passes.
            if self.verify_calls == 1 {
                None
            } else {
                Some(Event::VerifyDone)
            }
        }
        fn recover(&mut self) -> Option<Event> {
            None
        }
        fn update(&mut self) -> Option<Event> {
            Some(Event::UpdateDone(UpdateScope::Platform))
        }
    }

    let mut sm = StateMachine::new();
    let mut q = TestQueue::default();
    let mut actions = UpdateOnReset::default();

    // Boot -> Init -> (InitDone) -> FirmwareVerify. Verify parks (no follow-up),
    // so the queue is empty and the machine awaits an external event.
    sm.step(Event::Start, &mut q, &mut actions).unwrap(); // -> Init, pushes InitDone
    let ev = q.recv().unwrap();
    sm.step(ev, &mut q, &mut actions).unwrap(); // InitDone -> FirmwareVerify
    assert_eq!(sm.state(), State::FirmwareVerify);
    assert!(q.is_empty(), "verify parked awaiting update intent");

    // External producer injects the update intent.
    sm.step(Event::UpdateRequested, &mut q, &mut actions).unwrap(); // -> FirmwareUpdate, pushes UpdateDone
    assert_eq!(sm.state(), State::FirmwareUpdate);

    // Drain: UpdateDone -> FirmwareVerify -> VerifyDone -> Runtime.
    while !q.is_empty() {
        let ev = q.recv().unwrap();
        sm.step(ev, &mut q, &mut actions).unwrap();
    }
    assert_eq!(sm.state(), State::Runtime);
}
