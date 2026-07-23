// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_sm` — the eRoT boot-sequence state machine.
//!
//! This is the pure-reducer core ported from `rot_reducer`. It describes side
//! effects as [`Effect`] values rather than performing them; the surrounding
//! OpenPRoT shell carries them out via a [`Platform`] impl. No concrete hardware
//! appears here — the machine is generic over an opaque [`ComponentId`].
//!
//! See `docs/verification-model.md` and `docs/state-machine.md` in the
//! `rot_reducer` workspace for the full domain context and design rationale.
//!
//! Three invariants define the boundary:
//!   1. **Effects flow through [`Sink`]** — fresh per event, drained afterward.
//!   2. **Feedback as data ([`Effect::Emit`])** — follow-up events are effects,
//!      visible in the trace; used for the retry cap (INV7).
//!   3. **Reads as events** — outside information arrives in [`Event`] payloads;
//!      the core never reads anything directly.

#![no_std]
#![forbid(unsafe_code)]

use core::marker::PhantomData;

use statig::blocking::{
    IntoStateMachine, IntoStateMachineExt as _, State as StatigState, StateMachine,
    Superstate as StatigSuperstate,
};
use statig::Outcome;

// Internal capacities — these follow from how the machine works, not from the
// deployment. The board owns CAPACITY (chain length) and max_retry.

/// Max effects one event can emit. The busiest handler emits 3; 8 is plenty.
const EFFECT_CAP: usize = 8;

/// Max pending events while settling one outside event (original + Emit follow-ups).
const PENDING_CAP: usize = 8;

/// An opaque identifier for one platform component. The core never inspects it;
/// the board layer decides which real hardware each id refers to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ComponentId(u8);

impl ComponentId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// How a component in the trust chain is classified. The board supplies one
/// [`ComponentKind`] per [`ComponentId`] when building the chain.
///
/// Corresponds directly to the two-tier model in the CSA architecture document:
/// `Active` = eRoT gate + iRoT gate; `Passive` = eRoT gate only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComponentKind {
    /// Has an integrated iRoT (e.g. Caliptra). Both eRoT-side (signature + SVN)
    /// and iRoT-side (local self-verification) checks apply. The machine waits in
    /// [`State::AwaitingReady`] for [`Event::ComponentReady`] before advancing.
    Active,
    /// No integrated iRoT. The eRoT's signature + SVN check is the only gate.
    /// The chain walk advances immediately after `ReleaseReset`.
    Passive,
}

/// Per-component attributes supplied by the board at chain-build time.
///
/// Two orthogonal axes:
/// - [`kind`](ComponentAttrs::kind): controls the iRoT gate (Active vs Passive).
/// - [`required`](ComponentAttrs::required): controls failure policy.
///   * `true` — verification failure triggers recovery and halts the chain walk.
///   * `false` — verification failure holds the component in reset and skips it;
///     the chain walk continues to the next component without recovery.
///
/// A `required: false` component is never released from reset on failure — running
/// untrusted firmware would break the trust invariant regardless of policy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ComponentAttrs {
    pub kind: ComponentKind,
    pub required: bool,
}

impl ComponentAttrs {
    pub const fn active_required() -> Self {
        Self {
            kind: ComponentKind::Active,
            required: true,
        }
    }
    pub const fn passive_required() -> Self {
        Self {
            kind: ComponentKind::Passive,
            required: true,
        }
    }
    pub const fn active_optional() -> Self {
        Self {
            kind: ComponentKind::Active,
            required: false,
        }
    }
    pub const fn passive_optional() -> Self {
        Self {
            kind: ComponentKind::Passive,
            required: false,
        }
    }
}

/// The result of the board's power-on checks, delivered inside [`Event::PowerGood`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerOnResult {
    /// Self-verified and provisioned.
    Provisioned,
    /// Self-verified but not provisioned — cannot act as a RoT.
    Unprovisioned,
    /// Self-verification failed — latches immediately to [`State::Locked`].
    SelfVerificationFailed,
}

/// Everything the outside world can tell the state machine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// Power-on, carrying the shell's self-verification and provisioning result.
    PowerGood(PowerOnResult),
    VerificationPassed(ComponentId),
    VerificationFailed(ComponentId),
    /// An `Active` component's iRoT has finished local verification and is ready
    /// (e.g. MCTP channel established).
    ComponentReady(ComponentId),
    AttestationChallenge,
    UpdateRequest,
    UpdateVerified,
    UpdateRejected,
    CorruptionDetected(ComponentId),
    Restored(ComponentId),
    RecoveryFailed,
}

/// Everything the state machine can ask the outside world to do.
///
/// [`Effect::Emit`] is the sole internal effect: the orchestrator catches it and
/// queues the carried event for immediate handling, making follow-up events
/// visible in the effect trace instead of hidden state changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    ReadFirmware(ComponentId),
    VerifyFirmware(ComponentId),
    ReleaseReset(ComponentId),
    /// Assert reset on a component that is already running — the inverse of
    /// [`ReleaseReset`]. Emitted when an optional component is found corrupt at
    /// runtime: the component is gated without triggering a recovery cycle.
    AssertReset(ComponentId),
    SignAttestation,
    AuthenticateUpdate,
    StageUpdate,
    ActivateUpdate,
    DiscardStaged,
    RestoreGoldenImage(ComponentId),
    LatchLockdown,
    /// Internal only — tells the orchestrator to handle this event next.
    /// Never forwarded to a [`Platform`].
    Emit(Event),
}

/// The states the machine can be in. None carry data; all mutable state lives
/// in [`Rot`] shared storage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    PowerOnReset,
    VerifyingPlatform,
    /// eRoT has released an `Active` component; waiting for its iRoT to finish
    /// local verification and signal [`Event::ComponentReady`].
    AwaitingReady,
    Ready,
    Updating,
    Recovering,
    Locked,
}

/// Group state shared by the operational states.
#[derive(Debug)]
pub enum Superstate<'sub> {
    Operational(PhantomData<&'sub ()>),
}

/// The effect buffer handed to every handler (statig's `Context`).
///
/// The only thing a handler can do to the outside world is call `emit`. The
/// orchestrator gives each event a fresh `Sink` and drains it afterward.
pub struct Sink {
    effects: heapless::Vec<Effect, EFFECT_CAP>,
}

impl Sink {
    fn new() -> Self {
        Self {
            effects: heapless::Vec::new(),
        }
    }

    /// Append one effect. Overflow is silently dropped rather than panicking
    /// (`no_std` safety); overflow means a logic bug.
    pub fn emit(&mut self, effect: Effect) {
        let _ = self.effects.push(effect);
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
}

/// Shared storage: data that persists across events. `N` is the chain capacity
/// — a board choice; the core sets no default.
pub struct Rot<const N: usize> {
    chain: heapless::Vec<(ComponentId, ComponentAttrs), N>,
    cursor: u8,
    failed: Option<ComponentId>,
    retry_count: u8,
    max_retry: u8,
    /// The `Active` component whose iRoT readiness is outstanding. `Some` only
    /// while in `AwaitingReady` (INV9).
    awaiting: Option<ComponentId>,
}

impl<const N: usize> Rot<N> {
    pub fn new(chain: heapless::Vec<(ComponentId, ComponentAttrs), N>, max_retry: u8) -> Self {
        Self {
            chain,
            cursor: 0,
            failed: None,
            retry_count: 0,
            max_retry,
            awaiting: None,
        }
    }
}

impl<const N: usize> IntoStateMachine for Rot<N> {
    type Event<'evt> = Event;
    type Context<'ctx> = Sink;
    type State = State;
    type Superstate<'sub> = Superstate<'sub>;

    fn initial() -> State {
        State::PowerOnReset
    }
}

impl<const N: usize> StatigState<Rot<N>> for State {
    fn call_handler(&mut self, rot: &mut Rot<N>, event: &Event, ctx: &mut Sink) -> Outcome<State> {
        match self {
            State::PowerOnReset => match event {
                Event::PowerGood(PowerOnResult::Provisioned) => {
                    Outcome::Transition(State::VerifyingPlatform)
                }
                Event::PowerGood(PowerOnResult::Unprovisioned) => {
                    Outcome::Transition(State::Locked)
                }
                Event::PowerGood(PowerOnResult::SelfVerificationFailed) => {
                    Outcome::Transition(State::Locked)
                }
                _ => Outcome::Super,
            },

            // Cursor walk via Outcome::Handled — a self-transition would reset cursor.
            State::VerifyingPlatform => match event {
                Event::VerificationPassed(id) => {
                    ctx.emit(Effect::ReleaseReset(*id));
                    let current_attrs = rot.chain[rot.cursor as usize].1;
                    let next_idx = (rot.cursor as usize) + 1;
                    if next_idx < rot.chain.len() {
                        let (next_id, _) = rot.chain[next_idx];
                        rot.cursor += 1;
                        // Speculative: start next eRoT check while current Active iRoT boots.
                        ctx.emit(Effect::ReadFirmware(next_id));
                        ctx.emit(Effect::VerifyFirmware(next_id));
                        match current_attrs.kind {
                            ComponentKind::Active => {
                                rot.awaiting = Some(*id);
                                Outcome::Transition(State::AwaitingReady)
                            }
                            ComponentKind::Passive => Outcome::Handled,
                        }
                    } else {
                        Outcome::Transition(State::Ready)
                    }
                }
                Event::VerificationFailed(id) => {
                    let attrs = rot.chain[rot.cursor as usize].1;
                    if attrs.required {
                        rot.failed = Some(*id);
                        Outcome::Transition(State::Recovering)
                    } else {
                        // Optional: hold in reset, skip, advance walk.
                        let next_idx = (rot.cursor as usize) + 1;
                        rot.cursor += 1;
                        if next_idx < rot.chain.len() {
                            let (next_id, _) = rot.chain[next_idx];
                            ctx.emit(Effect::ReadFirmware(next_id));
                            ctx.emit(Effect::VerifyFirmware(next_id));
                            Outcome::Handled
                        } else {
                            Outcome::Transition(State::Ready)
                        }
                    }
                }
                _ => Outcome::Super,
            },

            State::AwaitingReady => match event {
                Event::ComponentReady(id) => {
                    if rot.awaiting != Some(*id) {
                        return Outcome::Handled; // spurious / stale (INV9)
                    }
                    rot.awaiting = None;
                    // If cursor is past the end, the last component was skipped
                    // (optional failure) — nothing left to verify, we're done.
                    if (rot.cursor as usize) >= rot.chain.len() {
                        Outcome::Transition(State::Ready)
                    } else {
                        Outcome::Handled
                    }
                }
                Event::VerificationPassed(id) => {
                    ctx.emit(Effect::ReleaseReset(*id));
                    let next_idx = (rot.cursor as usize) + 1;
                    if next_idx < rot.chain.len() {
                        let (next_id, _) = rot.chain[next_idx];
                        rot.cursor += 1;
                        ctx.emit(Effect::ReadFirmware(next_id));
                        ctx.emit(Effect::VerifyFirmware(next_id));
                        Outcome::Handled
                    } else {
                        Outcome::Transition(State::Ready)
                    }
                }
                Event::VerificationFailed(id) => {
                    let attrs = rot.chain[rot.cursor as usize].1;
                    if attrs.required {
                        rot.failed = Some(*id);
                        rot.awaiting = None;
                        Outcome::Transition(State::Recovering)
                    } else {
                        // Optional: hold in reset, skip, advance walk.
                        let next_idx = (rot.cursor as usize) + 1;
                        rot.cursor += 1;
                        if next_idx < rot.chain.len() {
                            let (next_id, _) = rot.chain[next_idx];
                            ctx.emit(Effect::ReadFirmware(next_id));
                            ctx.emit(Effect::VerifyFirmware(next_id));
                            Outcome::Handled
                        } else if rot.awaiting.is_none() {
                            // No iRoT gate pending — done.
                            Outcome::Transition(State::Ready)
                        } else {
                            // Still waiting for ComponentReady; it will fire Ready.
                            Outcome::Handled
                        }
                    }
                }
                _ => Outcome::Super,
            },

            State::Ready => match event {
                Event::UpdateRequest => Outcome::Transition(State::Updating),
                _ => Outcome::Super,
            },

            State::Updating => match event {
                Event::UpdateVerified => {
                    ctx.emit(Effect::ActivateUpdate);
                    Outcome::Transition(State::Ready)
                }
                Event::UpdateRejected => {
                    ctx.emit(Effect::DiscardStaged);
                    Outcome::Transition(State::Ready)
                }
                _ => Outcome::Super,
            },

            State::Recovering => match event {
                Event::Restored(_) => {
                    rot.retry_count = rot.retry_count.saturating_add(1);
                    if rot.retry_count >= rot.max_retry {
                        ctx.emit(Effect::Emit(Event::RecoveryFailed));
                        Outcome::Handled
                    } else {
                        Outcome::Transition(State::VerifyingPlatform)
                    }
                }
                Event::RecoveryFailed => Outcome::Transition(State::Locked),
                _ => Outcome::Super,
            },

            State::Locked => Outcome::Super,
        }
    }

    fn call_entry_action(&mut self, rot: &mut Rot<N>, ctx: &mut Sink) {
        match self {
            State::VerifyingPlatform => {
                rot.cursor = 0;
                rot.awaiting = None;
                if let Some(&(first_id, _)) = rot.chain.first() {
                    ctx.emit(Effect::ReadFirmware(first_id));
                    ctx.emit(Effect::VerifyFirmware(first_id));
                }
            }
            State::Updating => {
                ctx.emit(Effect::AuthenticateUpdate);
                ctx.emit(Effect::StageUpdate);
            }
            State::Recovering => {
                if let Some(failed) = rot.failed {
                    ctx.emit(Effect::RestoreGoldenImage(failed));
                }
            }
            State::Locked => {
                ctx.emit(Effect::LatchLockdown);
            }
            State::Ready => {
                rot.retry_count = 0;
            }
            _ => {}
        }
    }

    fn superstate(&mut self) -> Option<Superstate<'_>> {
        match self {
            State::Ready | State::Updating | State::Recovering | State::AwaitingReady => {
                Some(Superstate::Operational(PhantomData))
            }
            _ => None,
        }
    }
}

impl<const N: usize> StatigSuperstate<Rot<N>> for Superstate<'_> {
    fn call_handler(&mut self, rot: &mut Rot<N>, event: &Event, ctx: &mut Sink) -> Outcome<State> {
        match self {
            Superstate::Operational(_) => match event {
                Event::AttestationChallenge => {
                    ctx.emit(Effect::SignAttestation);
                    Outcome::Handled
                }
                Event::CorruptionDetected(id) => {
                    // Respect the per-component policy encoded at chain-build time.
                    // required: true  → recover (halt chain, restore, re-walk)
                    // required: false → ignore corruption; component stays running
                    //                   but is not considered trusted by the core.
                    let required = rot
                        .chain
                        .iter()
                        .find(|(cid, _)| cid == id)
                        .map(|(_, attrs)| attrs.required)
                        .unwrap_or(true); // unknown id: treat as required (safe default)
                    if required {
                        rot.failed = Some(*id);
                        Outcome::Transition(State::Recovering)
                    } else {
                        // Optional: gate the component (put it back in reset) but
                        // do not halt the chain or trigger recovery.
                        ctx.emit(Effect::AssertReset(*id));
                        Outcome::Handled
                    }
                }
                _ => Outcome::Super,
            },
        }
    }
}

/// Outward connection to the platform. Carry out one effect. Never called with
/// [`Effect::Emit`] — the orchestrator consumes those internally.
pub trait Platform {
    fn execute(&mut self, effect: Effect);
}

/// A handle for a caller's own event loop. Wraps the statig machine so callers
/// only depend on this crate, never on statig types directly.
pub struct Orchestrator<const N: usize> {
    machine: StateMachine<Rot<N>>,
}

impl<const N: usize> Orchestrator<N> {
    pub fn new(chain: heapless::Vec<(ComponentId, ComponentAttrs), N>, max_retry: u8) -> Self {
        Self {
            machine: Rot::new(chain, max_retry).state_machine(),
        }
    }

    pub fn state(&self) -> State {
        *self.machine.state()
    }

    /// Handle one event all the way through — including any [`Effect::Emit`]
    /// follow-ups — calling `on_effect` for each external effect in order.
    pub fn dispatch_with(&mut self, event: Event, mut on_effect: impl FnMut(Effect)) {
        let mut pending: heapless::Vec<Event, PENDING_CAP> = heapless::Vec::new();
        let _ = pending.push(event);

        let mut i = 0;
        while i < pending.len() {
            let ev = pending[i];
            i += 1;

            let mut buf = Sink::new();
            self.machine.handle_with_context(&ev, &mut buf);

            for &effect in buf.effects() {
                match effect {
                    Effect::Emit(internal) => {
                        let _ = pending.push(internal);
                    }
                    external => on_effect(external),
                }
            }
        }
    }

    /// Same as [`dispatch_with`] but routes effects to a [`Platform`].
    pub fn dispatch(&mut self, platform: &mut impl Platform, event: Event) {
        self.dispatch_with(event, |effect| platform.execute(effect));
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    const C0: ComponentId = ComponentId::new(0);
    const C1: ComponentId = ComponentId::new(1);
    const C2: ComponentId = ComponentId::new(2);

    const BOOT: Event = Event::PowerGood(PowerOnResult::Provisioned);

    const CAPACITY: usize = 8;
    const MAX_RETRY: u8 = 3;

    fn chain(
        ids: &[(ComponentId, ComponentAttrs)],
    ) -> heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY> {
        let mut c = heapless::Vec::new();
        for &entry in ids {
            c.push(entry).expect("chain within CAPACITY");
        }
        c
    }

    fn passive_required(
        ids: &[ComponentId],
    ) -> heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY> {
        chain(
            &ids.iter()
                .map(|&id| (id, ComponentAttrs::passive_required()))
                .collect::<std::vec::Vec<_>>(),
        )
    }

    struct Recorder {
        recorded: Vec<Effect>,
    }

    impl Recorder {
        fn new() -> Self {
            Self {
                recorded: Vec::new(),
            }
        }
    }

    impl Platform for Recorder {
        fn execute(&mut self, effect: Effect) {
            self.recorded.push(effect);
        }
    }

    fn drive(
        chain: heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY>,
        script: &[Event],
    ) -> (Vec<Effect>, State) {
        let mut orch = Orchestrator::new(chain, MAX_RETRY);
        let mut platform = Recorder::new();
        for &event in script {
            orch.dispatch(&mut platform, event);
        }
        (platform.recorded, orch.state())
    }

    /// INV1/INV2/INV3: provisioned power-on walks the chain in order; no
    /// component is released before its eRoT-side verification passes.
    #[test]
    fn cold_boot_walks_chain_in_order() {
        let (effects, state) = drive(
            passive_required(&[C0, C1]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::VerificationPassed(C1),
            ],
        );
        assert_eq!(
            effects,
            std::vec![
                Effect::ReadFirmware(C0),
                Effect::VerifyFirmware(C0),
                Effect::ReleaseReset(C0),
                Effect::ReadFirmware(C1),
                Effect::VerifyFirmware(C1),
                Effect::ReleaseReset(C1),
            ],
        );
        assert_eq!(state, State::Ready);
    }

    /// Unprovisioned power-on latches immediately.
    #[test]
    fn unprovisioned_boot_locks_down() {
        let (effects, state) = drive(
            passive_required(&[C0]),
            &[Event::PowerGood(PowerOnResult::Unprovisioned)],
        );
        assert_eq!(effects, std::vec![Effect::LatchLockdown]);
        assert_eq!(state, State::Locked);
    }

    /// INV11: SelfVerificationFailed latches immediately without entering
    /// VerifyingPlatform.
    #[test]
    fn self_verification_failure_latches_immediately() {
        let (effects, state) = drive(
            passive_required(&[C0]),
            &[Event::PowerGood(PowerOnResult::SelfVerificationFailed)],
        );
        assert_eq!(effects, std::vec![Effect::LatchLockdown]);
        assert_eq!(state, State::Locked);
    }

    /// INV6: AttestationChallenge is answerable from every Operational state.
    #[test]
    fn attestation_shared_across_operational_states() {
        let (effects, state) = drive(
            passive_required(&[C0]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::AttestationChallenge,
            ],
        );
        assert_eq!(effects.last(), Some(&Effect::SignAttestation));
        assert_eq!(state, State::Ready);

        let (effects, state) = drive(
            passive_required(&[C0]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::UpdateRequest,
                Event::AttestationChallenge,
            ],
        );
        assert_eq!(effects.last(), Some(&Effect::SignAttestation));
        assert_eq!(state, State::Updating);
    }

    /// INV4: a rejected update rolls back via DiscardStaged and never enters
    /// Recovering.
    #[test]
    fn update_rollback_is_not_recovery() {
        let (effects, state) = drive(
            passive_required(&[C0]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::UpdateRequest,
                Event::UpdateRejected,
            ],
        );
        let tail = &effects[effects.len() - 3..];
        assert_eq!(
            tail,
            &[
                Effect::AuthenticateUpdate,
                Effect::StageUpdate,
                Effect::DiscardStaged
            ],
        );
        assert_eq!(state, State::Ready);
        assert!(!effects.contains(&Effect::LatchLockdown));
    }

    /// INV5: runtime corruption targets the named component and re-walks from
    /// the top after restore.
    #[test]
    fn runtime_corruption_targets_component_and_rewalks() {
        let (effects, state) = drive(
            passive_required(&[C0, C1]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::VerificationPassed(C1),
                Event::CorruptionDetected(C1),
                Event::Restored(C1),
            ],
        );
        let tail = &effects[effects.len() - 2..];
        assert_eq!(
            tail,
            &[Effect::ReadFirmware(C0), Effect::VerifyFirmware(C0)]
        );
        assert_eq!(state, State::VerifyingPlatform);
    }

    /// INV7 (feedback-as-data): after MAX_RETRY restores the core self-emits
    /// RecoveryFailed and latches to Locked without any external RecoveryFailed
    /// in the script.
    #[test]
    fn retry_cap_self_latches_via_emit() {
        let mut script = std::vec![BOOT, Event::VerificationPassed(C0)];
        script.push(Event::CorruptionDetected(C0));
        for _ in 0..(MAX_RETRY - 1) {
            script.push(Event::Restored(C0));
            script.push(Event::VerificationFailed(C0));
        }
        script.push(Event::Restored(C0));

        let (effects, state) = drive(passive_required(&[C0]), &script);

        assert!(!script.contains(&Event::RecoveryFailed));
        assert_eq!(state, State::Locked);
        assert_eq!(effects.last(), Some(&Effect::LatchLockdown));
    }

    /// INV7: retry count resets after a successful recovery so a later episode
    /// starts from zero.
    #[test]
    fn retry_count_resets_after_successful_recovery() {
        let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
        c.push((C0, ComponentAttrs::passive_required()))
            .expect("fits");
        let mut orch = Orchestrator::new(c, 2);
        let mut effects = Vec::new();

        for ev in [
            BOOT,
            Event::VerificationPassed(C0),
            Event::CorruptionDetected(C0),
            Event::Restored(C0),
            Event::VerificationPassed(C0),
        ] {
            orch.dispatch_with(ev, |e| effects.push(e));
        }
        assert_eq!(orch.state(), State::Ready);

        let start = effects.len();
        for ev in [
            Event::CorruptionDetected(C0),
            Event::Restored(C0),
            Event::VerificationPassed(C0),
        ] {
            orch.dispatch_with(ev, |e| effects.push(e));
        }
        assert_eq!(orch.state(), State::Ready);
        assert!(!effects[start..].contains(&Effect::LatchLockdown));
    }

    /// Board-supplied retry cap: max_retry = 1 latches on the first failed
    /// restore.
    #[test]
    fn custom_retry_cap_latches_sooner() {
        let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
        c.push((C0, ComponentAttrs::passive_required()))
            .expect("fits");
        let mut orch = Orchestrator::new(c, 1);
        let mut effects = Vec::new();
        for ev in [
            BOOT,
            Event::VerificationPassed(C0),
            Event::CorruptionDetected(C0),
            Event::Restored(C0),
        ] {
            orch.dispatch_with(ev, |e| effects.push(e));
        }
        assert_eq!(orch.state(), State::Locked);
        assert_eq!(effects.last(), Some(&Effect::LatchLockdown));
    }

    /// Three-component chain uses N=3; walks all three to Ready.
    #[test]
    fn custom_capacity_walks_full_chain() {
        let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), 3>::new();
        for &id in &[C0, C1, C2] {
            c.push((id, ComponentAttrs::passive_required()))
                .expect("3 fits");
        }
        let mut orch = Orchestrator::new(c, MAX_RETRY);
        let mut effects = Vec::new();
        for ev in [
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::VerificationPassed(C2),
        ] {
            orch.dispatch_with(ev, |e| effects.push(e));
        }
        assert_eq!(orch.state(), State::Ready);
        assert_eq!(effects.last(), Some(&Effect::ReleaseReset(C2)));
    }

    /// INV10: Active component gates the chain walk — cursor does not advance
    /// until ComponentReady arrives.
    #[test]
    fn active_component_gates_on_component_ready() {
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::active_required()),
                (C1, ComponentAttrs::passive_required()),
            ]),
            &[BOOT, Event::VerificationPassed(C0)],
        );
        assert_eq!(state, State::AwaitingReady);
        assert!(effects.contains(&Effect::ReleaseReset(C0)));
        assert!(effects.contains(&Effect::ReadFirmware(C1)));

        let (effects2, state2) = drive(
            chain(&[
                (C0, ComponentAttrs::active_required()),
                (C1, ComponentAttrs::passive_required()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::ComponentReady(C0),
                Event::VerificationPassed(C1),
            ],
        );
        assert_eq!(state2, State::Ready);
        assert!(effects2.contains(&Effect::ReleaseReset(C1)));
    }

    /// INV9: a ComponentReady for the wrong id is silently ignored.
    #[test]
    fn spurious_component_ready_is_ignored() {
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::active_required()),
                (C1, ComponentAttrs::passive_required()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::ComponentReady(C1), // wrong id
            ],
        );
        assert_eq!(state, State::AwaitingReady);
        assert!(!effects.contains(&Effect::ReleaseReset(C1)));
    }

    /// INV12: AttestationChallenge is handled in AwaitingReady.
    #[test]
    fn attestation_in_awaiting_ready() {
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::active_required()),
                (C1, ComponentAttrs::passive_required()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::AttestationChallenge,
            ],
        );
        assert_eq!(state, State::AwaitingReady);
        assert_eq!(effects.last(), Some(&Effect::SignAttestation));
    }

    /// Optional component: VerificationFailed skips it (held in reset), chain
    /// continues to Ready.
    #[test]
    fn optional_component_failure_skips_and_continues() {
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::passive_required()),
                (C1, ComponentAttrs::passive_optional()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::VerificationFailed(C1),
            ],
        );
        assert_eq!(state, State::Ready);
        // C1 must never be released.
        assert!(!effects.contains(&Effect::ReleaseReset(C1)));
        // No recovery triggered.
        assert!(!effects.contains(&Effect::RestoreGoldenImage(C1)));
        assert!(!effects.contains(&Effect::LatchLockdown));
    }

    /// Optional Active component failure in AwaitingReady: skipped, held in
    /// reset, chain reaches Ready once ComponentReady clears awaiting.
    #[test]
    fn optional_active_failure_in_awaiting_ready_skips() {
        // C0 Active required, C1 Active optional.
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::active_required()),
                (C1, ComponentAttrs::active_optional()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0), // → AwaitingReady; spec ReadFirmware(C1)
                Event::VerificationFailed(C1), // optional → skip C1
                Event::ComponentReady(C0),     // iRoT gate clears; cursor past end → Ready
            ],
        );
        assert_eq!(state, State::Ready);
        assert!(!effects.contains(&Effect::ReleaseReset(C1)));
        assert!(!effects.contains(&Effect::RestoreGoldenImage(C1)));
    }

    /// Runtime corruption of a `required: false` component gates the component
    /// (AssertReset) but does not trigger recovery — the machine stays in Ready.
    #[test]
    fn optional_runtime_corruption_is_ignored() {
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::passive_required()),
                (C1, ComponentAttrs::passive_optional()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::VerificationPassed(C1),
                Event::CorruptionDetected(C1), // optional → gate, no recovery
            ],
        );
        assert_eq!(state, State::Ready);
        assert!(effects.contains(&Effect::AssertReset(C1)));
        assert!(!effects.contains(&Effect::RestoreGoldenImage(C1)));
        assert!(!effects.contains(&Effect::LatchLockdown));
    }

    /// Runtime corruption of a `required: true` component still triggers
    /// recovery as before.
    #[test]
    fn required_runtime_corruption_triggers_recovery() {
        let (effects, state) = drive(
            chain(&[
                (C0, ComponentAttrs::passive_required()),
                (C1, ComponentAttrs::passive_optional()),
            ]),
            &[
                BOOT,
                Event::VerificationPassed(C0),
                Event::VerificationPassed(C1),
                Event::CorruptionDetected(C0), // required → Recovering
            ],
        );
        assert_eq!(state, State::Recovering);
        assert!(effects.contains(&Effect::RestoreGoldenImage(C0)));
    }
}
