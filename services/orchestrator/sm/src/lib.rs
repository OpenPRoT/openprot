// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_sm` — the eRoT boot-sequence state machine.
//!
//! This is the pure decision core: it describes side effects as [`Effect`]
//! values rather than performing them; the surrounding OpenPRoT platform
//! driver carries them out via a [`Platform`] impl. No concrete hardware
//! appears here — the machine is generic over an opaque [`ComponentId`].
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

mod model;
pub use model::*;

// Internal capacities — these follow from how the machine works, not from the
// deployment. The board owns `N` (chain length), `E` (effect-buffer size) and
// max_retry.

/// Upper bound on events queued at once inside `dispatch_with`. The queue
/// pops as it settles, so this bounds *in-flight* events, not a run's total
/// length: one batch can queue at most one `Emit` follow-up plus one returned
/// event per external effect. Executors that return an event for many effects
/// of one batch can overflow this; overflow is fail-closed (see
/// `dispatch_with`), never silent loss.
const PENDING_CAP: usize = 8;

/// Compile-time floor: room for an `Emit` follow-up, a returned event, and
/// the injected `EffectFailed`. Evaluated at build time (an anonymous
/// `const`), so an under-sized `PENDING_CAP` fails to compile.
const _: () = assert!(
    PENDING_CAP >= 3,
    "PENDING_CAP must hold an Emit follow-up + a returned event + EffectFailed",
);

/// Result of dispatching one event to a state (or its superstate).
enum Outcome {
    /// Event consumed; state unchanged; no entry action runs.
    Handled,
    /// Change to this state and run its entry action.
    Transition(State),
    /// Not handled here; defer to the superstate (or discard if none).
    Super,
}

/// The effect buffer handed to every handler, sized to `E`.
///
/// The only thing a handler can do to the outside world is call `emit`. The
/// orchestrator gives each event a fresh `Sink` and drains it afterward.
///
/// `E` is bounded from below by the chain length: the worst single event is a
/// full cascade (up to `N` `AssertReset`s, each paired with a `ReportIsolated`)
/// plus the destination `PreSupervision` entry's `ReadFirmware`/`VerifyFirmware`
/// (2), all landing in one `Sink`. The re-walk quiesce (an `AssertReset` per
/// live component) never pushes past this bound, because gating and quiescing
/// are mutually exclusive per component: a component the cascade isolates is not
/// also live, so the two counts never sum above the full-cascade worst case.
/// `Rot::new` refuses to compile unless `E >= 2 * N + 2`, so a machine that
/// builds can never overflow this buffer.
pub struct Sink<const E: usize> {
    effects: heapless::Vec<Effect, E>,
}

impl<const E: usize> Sink<E> {
    fn new() -> Self {
        Self {
            effects: heapless::Vec::new(),
        }
    }

    /// Append one effect. `E` is sized so overflow is impossible for a machine
    /// that compiles: `Rot::EFFECT_CAP_OK` proves `E >= 2 * N + 2` and no
    /// handler emits more than `2 * N + 2` effects into one `Sink`, so the push
    /// below can never fail. The `Err` arm is therefore dead — dropped rather
    /// than panicked, since a runtime panic here would be unreachable code
    /// shipped in the binary.
    ///
    /// The driver runs the effects from one handler in the order they were
    /// emitted, and it does not run them as a single all-or-nothing group: if
    /// one effect fails, the ones before it have already happened. When an
    /// effect fails, the driver stops there — it skips the rest and injects
    /// `EffectFailed` so the machine locks down. Stopping partway is still safe
    /// no matter what order the effects were in: a component is only ever
    /// released after it has passed verification, and every other effect either
    /// tightens things (holds a component in reset, or latches lockdown) or just
    /// reports what happened. So a batch that stops early can only leave the
    /// platform more locked down, never less. A skipped report costs information,
    /// not containment — and the batch was cut short by a failure that latches
    /// lockdown anyway, which is the louder signal.
    pub fn emit(&mut self, effect: Effect) {
        // Dead Err arm: overflow is proved impossible by `Rot::EFFECT_CAP_OK`
        // (`E >= 2 * N + 2`) plus the state machine never emitting more than
        // `2 * N + 2` effects into one Sink.
        let _ = self.effects.push(effect);
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
}

/// Outcome of [`Rot::gate_by_policy`]: whether a component was gated out of
/// service. Collapses the three [`FailurePolicy`] values into the two
/// control-flow outcomes the caller actually branches on — so the
/// runtime-corruption path and the recovery-exhaustion path decide on the same
/// result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gating {
    /// The component was gated (`Isolable` → itself; `Cascading` → itself and
    /// its transitive dependents). Its reset is asserted and the walk skips it.
    Gated,
    /// Not a gating policy (`Required`, or an unknown/missing id). Nothing was
    /// gated; the caller handles this in its own context — recover (at runtime)
    /// or lock down (once recovery is exhausted).
    NotGated,
}

/// Per-component service lifecycle. Each chain component carries one of these
/// inside its [`ComponentStatus`], recording whether it is in normal service or
/// gated out. The walk-phase payloads (`AwaitingReady`/`Recovering`) stay on the
/// global [`State`] rather than here — those phases are properties of the whole
/// machine, not of one component. Named for the *component service* axis to keep
/// it distinct from orchestrator-capabilities's trial-boot/commit (update-slot)
/// lifecycle, which
/// is a separate concern.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ComponentLifecycle {
    /// In the normal flow: held, under verification, or released. The global
    /// `State` carries which of those the walk is in.
    Nominal,
    /// Durably gated out of service: has a live `AssertReset` and is skipped on
    /// every chain walk.
    Isolated,
}

/// One per-component record (see [`Rot::statuses`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ComponentStatus {
    lifecycle: ComponentLifecycle,
    /// Consecutive failed-restore count (INV7: consecutive only).
    retry: u8,
    /// Set while this component has been released from reset but has not yet
    /// reported its boot-progress signal ([`Event::ComponentReady`] for an
    /// `Active` component, [`Event::Booted`] for a `Passive` one). The platform driver
    /// arms a per-component watchdog on release; this bit is what a later
    /// [`Event::Timeout`] consults to tell a real boot failure from a stale or
    /// spurious timeout. Orthogonal to `lifecycle`: a gated component owes no
    /// boot-progress signal, so gating clears it.
    awaiting_boot: bool,
    /// Set while this component has been released from reset and not since held
    /// again — i.e. it is *live*, executing code. Set at every `ReleaseReset`,
    /// cleared at every `AssertReset` (gating, recovery, or the pre-walk
    /// quiesce). This is what makes recovery a genuine platform re-boot: on
    /// re-entering [`State::PreSupervision`] the machine asserts reset on every
    /// live component before re-verifying, so `VerifyFirmware` never runs
    /// against code that is still executing (a live check says nothing about
    /// what is running and is open to a post-check flash rewrite). Distinct from
    /// `awaiting_boot`, which is cleared once the component reports in but stays
    /// live: a booted component is `released` yet no longer `awaiting_boot`.
    released: bool,
}

impl Default for ComponentStatus {
    fn default() -> Self {
        Self {
            lifecycle: ComponentLifecycle::Nominal,
            retry: 0,
            awaiting_boot: false,
            released: false,
        }
    }
}

/// Shared storage: data that persists across events. `N` is the chain capacity
/// and `E` the effect-buffer size — both board choices; the core sets no
/// default. `E` must be at least `2 * N + 2` (enforced in [`Rot::new`]).
pub struct Rot<const N: usize, const E: usize> {
    chain: heapless::Vec<(ComponentId, ComponentAttrs), N>,
    cursor: u8,
    /// One record per chain component (parallel to `chain` by index). Each
    /// [`ComponentStatus`] holds the component's service `lifecycle` (`Isolated`
    /// means gated out of the walk) and its consecutive failed-restore `retry`
    /// count, kept per component so interleaved recoveries never share a budget.
    /// Durable across a return to `Ready`; only a fresh `Rot` on `PowerOnReset`
    /// resets it.
    statuses: heapless::Vec<ComponentStatus, N>,
    max_retry: u8,
    /// Set when an update has been activated ([`Effect::ActivateUpdate`]) but
    /// its anti-rollback floor has not yet been committed, i.e. the machine is
    /// in the *activated-but-not-committed* window inside [`State::Ready`]. A
    /// [`Event::BootConfirmed`] commits the floor and clears this; a
    /// [`Event::CommitTimeout`] fired while this is set latches
    /// [`State::Locked`] (commit-or-lock: the floor is never advanced for an
    /// image that has not proven healthy, and the downgrade window is never left
    /// open indefinitely). Cleared on [`Event::BootConfirmed`] (the window
    /// closes normally) and on entry to the two states that end the window by
    /// leaving `Ready` while still running — [`State::Updating`] (a superseding
    /// update) and [`State::Recovering`] (corruption/timeout) — so any path that
    /// leaves and later re-enters `Ready` resets the window without per-branch
    /// bookkeeping. (It is *not* cleared on `Ready` entry, because activation
    /// sets it while transitioning *into* `Ready`.)
    pending_commit: bool,
    /// Ties the effect-buffer size `E` to this type (zero-sized).
    _effect_cap: PhantomData<[u8; E]>,
}

impl<const N: usize, const E: usize> Rot<N, E> {
    /// Compile-time floor: the effect buffer must hold a full cascade (`N`
    /// `AssertReset`s paired with `N` `ReportIsolated`s) plus the destination
    /// `PreSupervision` entry's two effects. Forced by `new` below, so an
    /// under-sized `E` fails to build.
    const EFFECT_CAP_OK: () = assert!(
        E >= 2 * N + 2,
        "effect buffer E must be >= 2 * chain length N + 2",
    );

    pub fn new(chain: Chain<N>, max_retry: u8) -> Self {
        let () = Self::EFFECT_CAP_OK;
        let chain = chain.into_entries();
        // One status record per chain component, parallel by index.
        let mut statuses = heapless::Vec::new();
        for _ in 0..chain.len() {
            let _ = statuses.push(ComponentStatus::default());
        }
        Self {
            chain,
            cursor: 0,
            statuses,
            max_retry,
            pending_commit: false,
            _effect_cap: PhantomData,
        }
    }

    /// Look up a component's attributes by id. `None` if the id is not in the
    /// chain (should never happen for ids the core itself produced).
    fn attrs_of(&self, id: ComponentId) -> Option<ComponentAttrs> {
        self.chain
            .iter()
            .find(|(cid, _)| *cid == id)
            .map(|(_, a)| *a)
    }

    /// Index of `id` in the parallel `chain`/`statuses` arrays. `None` if the
    /// id is not in the chain (should never happen for core-produced ids).
    fn status_index(&self, id: ComponentId) -> Option<usize> {
        self.chain.iter().position(|(cid, _)| *cid == id)
    }

    /// Whether `id` names a component in the configured chain. The core only
    /// supervises chain components, so an event that names an id outside the
    /// chain is dropped rather than acted on — it describes something the core
    /// has no model of and never released.
    fn in_chain(&self, id: ComponentId) -> bool {
        self.status_index(id).is_some()
    }

    fn is_gated(&self, id: ComponentId) -> bool {
        self.status_index(id)
            .is_some_and(|i| self.statuses[i].lifecycle == ComponentLifecycle::Isolated)
    }

    /// Gate one component out of service, returning `true` if this newly gated
    /// it (`false` if already `Isolated`). Emits the paired `AssertReset` +
    /// `ReportIsolated` only on the newly-gated transition, so reporting is
    /// idempotent per component.
    fn gate_one(&mut self, ctx: &mut Sink<E>, id: ComponentId) -> bool {
        if self.is_gated(id) {
            return false;
        }
        ctx.emit(Effect::AssertReset(id));
        ctx.emit(Effect::ReportIsolated(id));
        if let Some(i) = self.status_index(id) {
            self.statuses[i].lifecycle = ComponentLifecycle::Isolated;
            // A gated component is held in reset and no longer owes a
            // boot-progress signal; drop any pending watchdog so a late
            // `Timeout` can't drag an already-isolated component into recovery.
            // It is also no longer live — the `AssertReset` above holds it.
            self.statuses[i].awaiting_boot = false;
            self.statuses[i].released = false;
        }
        true
    }

    /// Increment `id`'s consecutive failed-restore count and return the new
    /// value. Counts are kept per component so that interleaved recovery
    /// episodes for different components never share a budget.
    fn bump_retry(&mut self, id: ComponentId) -> u8 {
        match self.status_index(id) {
            Some(i) => {
                self.statuses[i].retry = self.statuses[i].retry.saturating_add(1);
                self.statuses[i].retry
            }
            None => 0,
        }
    }

    /// Drop `id`'s retry count. Called when the component recovers (passes
    /// verification) or is gated — either way its consecutive-failure streak
    /// ends, so a future failure starts counting fresh (INV7: consecutive only).
    fn clear_retry(&mut self, id: ComponentId) {
        if let Some(i) = self.status_index(id) {
            self.statuses[i].retry = 0;
        }
    }

    /// Record that `id` has been released and now owes a boot-progress signal.
    /// Paired with the `ReleaseReset` emitted at each release site: the platform driver
    /// arms its per-component boot watchdog there, and this arms ours. Also
    /// marks the component *live* (`released`), which a later re-entry to
    /// [`State::PreSupervision`] uses to quiesce it before re-verifying.
    fn mark_released(&mut self, id: ComponentId) {
        if let Some(i) = self.status_index(id) {
            self.statuses[i].awaiting_boot = true;
            self.statuses[i].released = true;
        }
    }

    /// Clear `id`'s boot-progress watchdog because it reported in
    /// ([`Event::ComponentReady`] or [`Event::Booted`]). Idempotent: a report
    /// for a component not awaiting boot simply changes nothing.
    fn clear_awaiting_boot(&mut self, id: ComponentId) {
        if let Some(i) = self.status_index(id) {
            self.statuses[i].awaiting_boot = false;
        }
    }

    /// Whether `id` has been released and still owes a boot-progress signal.
    /// A [`Event::Timeout`] is a real boot failure only for such a component;
    /// otherwise it is stale/spurious.
    fn is_awaiting_boot(&self, id: ComponentId) -> bool {
        self.status_index(id)
            .is_some_and(|i| self.statuses[i].awaiting_boot)
    }

    /// Advance `cursor` from `start_idx` to the first component not in
    /// `gated`, emitting its `ReadFirmware`/`VerifyFirmware`. Returns `true` if
    /// found. If the rest of the chain is exhausted or entirely gated, sets
    /// `cursor` to a past-the-end sentinel (`chain.len()`) and returns
    /// `false` — the caller should treat that as "chain done".
    fn advance_to_next_ungated(&mut self, ctx: &mut Sink<E>, start_idx: usize) -> bool {
        let mut idx = start_idx;
        while let Some(&(id, _)) = self.chain.get(idx) {
            if !self.is_gated(id) {
                // `idx < chain.len() <= N`; chain capacity is board-chosen and
                // assumed to fit `u8`, matching the existing `cursor: u8` contract.
                self.cursor = idx as u8;
                ctx.emit(Effect::ReadFirmware(id));
                ctx.emit(Effect::VerifyFirmware(id));
                return true;
            }
            idx += 1;
        }
        self.cursor = self.chain.len() as u8;
        false
    }

    /// Hold the whole platform for an at-rest re-verification: assert reset on
    /// every component that is currently *live* (`released`) and drop its
    /// boot-progress watchdog, so the walk that follows verifies code that is
    /// quiesced rather than executing. This is what makes recovery a genuine
    /// platform re-boot — no released component is ever re-verified while it is
    /// still running (a live check says nothing about what is executing and is
    /// open to a post-check flash rewrite).
    ///
    /// Lifecycle is untouched: these components are held for re-check, not gated
    /// (`Nominal` stays `Nominal`), and each is released again by its
    /// `ReleaseReset` once it re-passes verification. Isolated components are
    /// already held (`released == false`) and are skipped, as is anything under
    /// active recovery. At the initial power-on walk nothing is live yet, so
    /// this emits nothing and cold boot is unchanged.
    fn quiesce_all(&mut self, ctx: &mut Sink<E>) {
        for i in 0..self.chain.len() {
            if !self.statuses[i].released {
                continue;
            }
            let (id, _) = self.chain[i];
            ctx.emit(Effect::AssertReset(id));
            self.statuses[i].released = false;
            self.statuses[i].awaiting_boot = false;
        }
    }
    /// shared by both paths that take a component out of service — the
    /// runtime-corruption path ([`handle_corruption`](Self::handle_corruption))
    /// and the recovery-exhaustion path — so the two can never disagree about
    /// what a policy means (in particular, `Cascading` cascades in both).
    ///
    /// - `Isolable` → gate this component alone → [`Gating::Gated`].
    /// - `Cascading` → gate this component and every transitive dependent →
    ///   [`Gating::Gated`].
    /// - `Required`, or an unknown/missing id → gate nothing →
    ///   [`Gating::NotGated`], leaving the caller to handle the non-gating
    ///   outcome in its own context (recover, at runtime; lock down, once
    ///   recovery is exhausted).
    ///
    /// Every component this gates also gets an [`Effect::ReportIsolated`], so
    /// management software learns the platform is degraded no matter which path
    /// took the component out of service.
    ///
    /// Idempotent per component (guarded by `is_gated`) — so is the reporting:
    /// a component already gated is neither re-reset nor re-reported.
    fn gate_by_policy(&mut self, ctx: &mut Sink<E>, id: ComponentId) -> Gating {
        match self.attrs_of(id).map(|attrs| attrs.failure_policy) {
            Some(FailurePolicy::Cascading) => {
                self.cascade_hold(ctx, id);
                Gating::Gated
            }
            Some(FailurePolicy::Isolable) => {
                self.gate_one(ctx, id);
                Gating::Gated
            }
            // `Required`, or an unknown/missing id: not a gating policy.
            _ => Gating::NotGated,
        }
    }

    /// Shared `CorruptionDetected` handling, called from both `PreSupervision`
    /// (directly) and `SupervisingPlatform` (via its superstate handler).
    /// Delegates the policy interpretation to [`gate_by_policy`](Self::gate_by_policy)
    /// so this path and the recovery-exhaustion path can never diverge:
    /// `Isolable`/`Cascading` → gate the component (single or cascade) and stay
    /// put, so a later re-walk skips it instead of silently re-releasing one we
    /// already found corrupt; `Required` → recover first (the halt-on-exhaustion
    /// decision happens later in `Recovering`).
    fn handle_corruption(&mut self, id: ComponentId, ctx: &mut Sink<E>) -> Outcome {
        match self.gate_by_policy(ctx, id) {
            Gating::Gated => Outcome::Handled,
            Gating::NotGated => Outcome::Transition(State::Recovering(id)),
        }
    }

    /// Hold `root` and cascade-hold every component whose `depends_on`
    /// (transitively) names it. Emits `AssertReset` and `ReportIsolated` for
    /// each newly gated component, including `root` itself — every isolated
    /// device is reported, not just the one that failed.
    fn cascade_hold(&mut self, ctx: &mut Sink<E>, root: ComponentId) {
        // BFS over the growing isolation front. `frontier` holds the components
        // gated so far whose dependents still need visiting; `statuses` records
        // the durable `Isolated` mark for each.
        let mut frontier: heapless::Vec<ComponentId, N> = heapless::Vec::new();
        if self.gate_one(ctx, root) {
            let _ = frontier.push(root);
        }
        let mut i = 0;
        while let Some(&holder) = frontier.get(i) {
            i += 1;
            let mut newly_gated: heapless::Vec<ComponentId, N> = heapless::Vec::new();
            for &(id, attrs) in self.chain.iter() {
                if attrs.depends_on == Some(holder) && !self.is_gated(id) {
                    let _ = newly_gated.push(id);
                }
            }
            for id in newly_gated {
                if self.gate_one(ctx, id) {
                    let _ = frontier.push(id);
                }
            }
        }
    }
}

/// The state machine proper: the per-state handlers, the superstate handler, and the
/// entry actions. These are pure functions of `(stored data, state, event)` —
/// they mutate [`Rot`]'s storage and push [`Effect`]s into the [`Sink`], and
/// return an [`Outcome`] describing what should happen to the current state.
/// Applying that outcome is the engine's job ([`Orchestrator::step`]).
impl<const N: usize, const E: usize> Rot<N, E> {
    /// Dispatch `event` to the handler for `state` (the leaf state). An
    /// [`Outcome::Super`] here means "not handled at this level" — the engine
    /// forwards to [`handle_supervising`](Self::handle_supervising) if `state`
    /// is supervised, and otherwise discards the event.
    fn handle(&mut self, state: State, event: &Event, ctx: &mut Sink<E>) -> Outcome {
        match state {
            State::PowerOnReset => match event {
                Event::PowerGood(PowerOnResult::Provisioned) => {
                    Outcome::Transition(State::PreSupervision)
                }
                Event::PowerGood(PowerOnResult::Unprovisioned) => {
                    Outcome::Transition(State::Locked)
                }
                Event::PowerGood(PowerOnResult::SelfVerificationFailed) => {
                    Outcome::Transition(State::Locked)
                }
                Event::EffectFailed => Outcome::Transition(State::Locked),
                _ => Outcome::Super,
            },

            // Cursor walk via Outcome::Handled — a self-transition would reset cursor.
            State::PreSupervision => match event {
                Event::VerificationPassed(id) => {
                    // Only the component currently under verification
                    // (`chain[cursor]`, whose `VerifyFirmware` was just emitted)
                    // may be released. A verdict for any other id is stale or
                    // out-of-turn and is dropped, so a misordered or hostile
                    // report cannot release an unverified component (cf. INV9
                    // for `ComponentReady`).
                    if self.chain.get(self.cursor as usize).map(|(c, _)| c) != Some(id) {
                        return Outcome::Handled;
                    }
                    // The component passed its check — it has recovered, so its
                    // consecutive-failure streak ends (INV7: consecutive only).
                    self.clear_retry(*id);
                    ctx.emit(Effect::ReleaseReset(*id));
                    // Released: it is now live, and owes a boot-progress signal
                    // (both tiers).
                    self.mark_released(*id);
                    let current_kind = self.chain.get(self.cursor as usize).map(|(_, a)| a.kind);
                    let next_idx = (self.cursor as usize).saturating_add(1);
                    if self.advance_to_next_ungated(ctx, next_idx) {
                        match current_kind {
                            Some(ComponentKind::Active) => {
                                Outcome::Transition(State::AwaitingReady(Some(*id)))
                            }
                            _ => Outcome::Handled,
                        }
                    } else {
                        Outcome::Transition(State::Ready)
                    }
                }
                Event::VerificationFailed(id) => {
                    // Recovery is attempted first for every failure, regardless
                    // of the component's recovery-failure policy (CSA: recover
                    // first, classify only once retries are exhausted).
                    Outcome::Transition(State::Recovering(*id))
                }
                // Minimal fix: react to a corruption report if one arrives,
                // even though `PreSupervision` isn't linked to
                // `SupervisingPlatform`. Not acting on data we already have
                // would be strictly worse than acting on it, regardless of
                // whether CSA defines a mechanism that guarantees this event
                // exists in the first place. `AttestationChallenge` is left
                // unhandled here (falls through to `Outcome::Super` and is
                // discarded) — that's a separate question.
                Event::CorruptionDetected(id) => self.handle_corruption(*id, ctx),
                // Boot-progress liveness for a passive component released
                // speculatively earlier in this same walk. Clear its watchdog
                // even though `PreSupervision` is unsupervised — acting on a
                // report we already have is never worse than dropping it (same
                // rationale as `CorruptionDetected` above). An `Active`
                // component's `ComponentReady` cannot arrive here: releasing an
                // active moves the machine straight to `AwaitingReady`.
                Event::Booted(id) => {
                    self.clear_awaiting_boot(*id);
                    Outcome::Handled
                }
                // Device-agnostic boot-progress watchdog. A passive component
                // released speculatively can miss its window while the walk is
                // still in `PreSupervision`; treat that as a boot failure and
                // recover it, exactly as the supervised states do. A timeout for
                // a component not awaiting boot (e.g. still under verification)
                // is spurious and dropped.
                Event::Timeout(id) => {
                    if self.is_awaiting_boot(*id) {
                        Outcome::Transition(State::Recovering(*id))
                    } else {
                        Outcome::Handled
                    }
                }
                Event::EffectFailed => Outcome::Transition(State::Locked),
                _ => Outcome::Super,
            },

            // `awaiting` is the state's own payload, so changing it means
            // re-entering the variant: `Outcome::Handled` leaves the payload
            // untouched. That is correct only because `AwaitingReady` has no
            // entry action — do not add one.
            State::AwaitingReady(awaiting) => match event {
                Event::ComponentReady(id) => {
                    // Clear this component's boot-progress watchdog whether or
                    // not it is the one this state's slot is tracking: a later
                    // `Active` released speculatively reports its readiness here
                    // too, and its watchdog must be cleared just the same.
                    self.clear_awaiting_boot(*id);
                    if awaiting != Some(*id) {
                        return Outcome::Handled; // not the tracked slot (INV9)
                    }
                    // If cursor is past the end, the eRoT side of the walk has
                    // already finished (chain done, or the remainder is held) —
                    // nothing left to verify, we're done.
                    if (self.cursor as usize) >= self.chain.len() {
                        Outcome::Transition(State::Ready)
                    } else {
                        // Readiness satisfied, chain not done: stay supervised,
                        // now awaiting nothing.
                        Outcome::Transition(State::AwaitingReady(None))
                    }
                }
                Event::VerificationPassed(id) => {
                    // Only the component currently under verification
                    // (`chain[cursor]`) may be released; a verdict for any other
                    // id is stale or out-of-turn and is dropped (cf. INV9 for
                    // `ComponentReady`).
                    if self.chain.get(self.cursor as usize).map(|(c, _)| c) != Some(id) {
                        return Outcome::Handled;
                    }
                    // The component passed its check — it has recovered, so its
                    // consecutive-failure streak ends (INV7: consecutive only).
                    self.clear_retry(*id);
                    ctx.emit(Effect::ReleaseReset(*id));
                    // Released: it is now live, and owes a boot-progress signal
                    // (both tiers).
                    self.mark_released(*id);
                    let next_idx = (self.cursor as usize).saturating_add(1);
                    if self.advance_to_next_ungated(ctx, next_idx) {
                        // `Handled` preserves the current payload: `awaiting` is
                        // cleared only by `ComponentReady` or `VerificationFailed`.
                        Outcome::Handled
                    } else {
                        Outcome::Transition(State::Ready)
                    }
                }
                Event::VerificationFailed(id) => {
                    // Recovery is attempted first for every failure, regardless
                    // of the component's recovery-failure policy.
                    Outcome::Transition(State::Recovering(*id))
                }
                // `Timeout` is intentionally not handled here: it falls through
                // to `handle_supervising`, which runs the device-agnostic
                // boot-progress watchdog uniformly across every supervised state
                // (an `AwaitingReady` timeout is no longer special-cased).
                _ => Outcome::Super,
            },

            State::Ready => match event {
                Event::UpdateRequest => Outcome::Transition(State::Updating),
                // Proven-boot checkpoint: the image authenticated at
                // `ActivateUpdate`, but the SVN floor only advances now, once
                // the driver reports it healthy. Handled in place — confirming a
                // running image is not a state change. Closing the window clears
                // `pending_commit` so a later `CommitTimeout` cannot lock a
                // device that has already committed.
                Event::BootConfirmed(id) => {
                    ctx.emit(Effect::CommitSvnFloor(*id));
                    self.pending_commit = false;
                    Outcome::Handled
                }
                // Commit watchdog. If the activated-but-not-committed window is
                // still open, fail closed: never commit an unproven image, and
                // never leave the downgrade window open indefinitely. Outside
                // the window this is a stale watchdog fire and is dropped.
                Event::CommitTimeout => {
                    if self.pending_commit {
                        Outcome::Transition(State::Locked)
                    } else {
                        Outcome::Handled
                    }
                }
                _ => Outcome::Super,
            },

            State::Updating => match event {
                Event::UpdateVerified => {
                    ctx.emit(Effect::ActivateUpdate);
                    // Open the activated-but-not-committed window: the floor is
                    // NOT advanced here; it waits for `BootConfirmed`. The
                    // driver arms its commit watchdog on `ActivateUpdate`, and
                    // `CommitTimeout` bounds this window (commit-or-lock).
                    self.pending_commit = true;
                    Outcome::Transition(State::Ready)
                }
                Event::UpdateRejected => {
                    ctx.emit(Effect::DiscardStaged);
                    Outcome::Transition(State::Ready)
                }
                Event::CorruptionDetected(id) => {
                    let outcome = self.handle_corruption(*id, ctx);
                    // Only a *preemption* (transition out to recovery) orphans
                    // the staged image. An `Isolable`/`Cascading` corruption
                    // returns `Handled` and the update continues untouched, so
                    // its staged image must survive — do not discard then.
                    if matches!(outcome, Outcome::Transition(State::Recovering(_))) {
                        // Dispose of the orphaned image *and* answer the update
                        // requester: the update it was awaiting a verdict on has
                        // been superseded by recovery, not merely delayed.
                        ctx.emit(Effect::DiscardStaged);
                        ctx.emit(Effect::ReportUpdateAborted);
                    }
                    outcome
                }
                _ => Outcome::Super,
            },

            State::Recovering(failed) => match event {
                Event::Restored(id) => {
                    // A `Restored` for a component other than the current
                    // recovery target (e.g. an episode displaced by a later
                    // corruption) must not be credited to this recovery. Drop
                    // it; the eventual re-walk re-verifies every component.
                    if *id != failed {
                        return Outcome::Handled;
                    }
                    // Count this attempt against the specific component in
                    // recovery, not a global budget (CSA: exhaustion is
                    // per-device). The recovery target is the state's payload,
                    // so it is always present — no defensive fallback needed.
                    let attempts = self.bump_retry(failed);
                    if attempts < self.max_retry {
                        Outcome::Transition(State::PreSupervision)
                    } else {
                        // Retries exhausted: gate via the same `gate_by_policy`
                        // the runtime-corruption path uses, so the two can never
                        // disagree. Gated → continue the walk; NotGated
                        // (Required/unknown) → lock down.
                        match self.gate_by_policy(ctx, failed) {
                            Gating::Gated => {
                                self.clear_retry(failed);
                                Outcome::Transition(State::PreSupervision)
                            }
                            // `Required`, or an unknown/missing id: report the
                            // component that forced the halt, then lock down.
                            // The report precedes the internal `Emit`, so it is
                            // actuated before the machine moves toward `Locked`.
                            Gating::NotGated => {
                                ctx.emit(Effect::ReportRecoveryFailed(failed));
                                ctx.emit(Effect::Emit(Event::RecoveryFailed));
                                Outcome::Handled
                            }
                        }
                    }
                }
                Event::RecoveryFailed => Outcome::Transition(State::Locked),
                _ => Outcome::Super,
            },

            State::Locked => Outcome::Super,
        }
    }

    /// The supervising handler, reached only when [`handle`](Self::handle)
    /// returns [`Outcome::Super`] from one of the four supervised states
    /// ([`State::AwaitingReady`], [`State::Ready`], [`State::Updating`],
    /// [`State::Recovering`] — see [`Orchestrator::is_supervised`]). It provides
    /// two platform-wide guarantees that must hold across all four:
    ///
    /// - Attestation challenges are always answered.
    /// - Corruption of a required component always triggers recovery.
    ///
    /// Note: [`State::PreSupervision`] is deliberately *not* supervised, so the
    /// attestation guarantee does not hold while a component is still being
    /// walked there — CSA defines no requirement to answer challenges before the
    /// chain walk completes, so `AttestationChallenge` is left unhandled in
    /// `PreSupervision` (falls through to [`Outcome::Super`] and, with no
    /// supervisor, is discarded).
    ///
    /// The corruption guarantee, however, *does* hold in `PreSupervision`: that
    /// state handles [`Event::CorruptionDetected`] directly (via
    /// [`handle_corruption`](Self::handle_corruption)) rather than through this
    /// handler, since routing it here would also pull in the attestation
    /// behavior above. CSA defines no mechanism guaranteeing a corruption report
    /// arrives for an already-released component's *live, executing* state (its
    /// only at-rest mechanism — background NVM integrity polling — is explicitly
    /// scoped to "at rest"/"between boots", not an in-progress boot's chain
    /// walk), but that only means such a report isn't guaranteed to exist — it's
    /// not a reason to discard one if it does arrive. See
    /// `corruption_during_presupervision_selfloop_triggers_recovery` in
    /// `tests.rs`.
    fn handle_supervising(&mut self, event: &Event, ctx: &mut Sink<E>) -> Outcome {
        match event {
            Event::AttestationChallenge => {
                ctx.emit(Effect::SignAttestation);
                Outcome::Handled
            }
            Event::CorruptionDetected(id) => self.handle_corruption(*id, ctx),
            // Boot-progress signals arriving after the walk left `PreSupervision`
            // / `AwaitingReady` (e.g. once the machine is already `Ready`): clear
            // the component's watchdog. `ComponentReady` is the active tier,
            // `Booted` the passive tier; both just retire the outstanding wait.
            Event::ComponentReady(id) | Event::Booted(id) => {
                self.clear_awaiting_boot(*id);
                Outcome::Handled
            }
            // Device-agnostic boot-progress watchdog across every supervised
            // state: a released component that never reported in before its
            // window closed is recovered like any other boot failure. A timeout
            // for a component not awaiting boot (already reported, gated, or
            // never released) is stale/spurious and dropped.
            Event::Timeout(id) => {
                if self.is_awaiting_boot(*id) {
                    Outcome::Transition(State::Recovering(*id))
                } else {
                    Outcome::Handled
                }
            }
            // A new update cannot start from any supervised state except
            // `Ready` (the machine is mid-walk, mid-update, or mid-recovery).
            // `Ready` accepts `UpdateRequest` upstream in `handle` (→ `Updating`)
            // and so never reaches here; this arm therefore fires only for
            // `AwaitingReady`, `Updating`, and `Recovering`. Report the refusal
            // rather than dropping it silently, and leave the in-flight
            // walk/update/recovery untouched (`Handled`, no transition).
            Event::UpdateRequest => {
                ctx.emit(Effect::ReportUpdateDeferred);
                Outcome::Handled
            }
            Event::EffectFailed => Outcome::Transition(State::Locked),
            _ => Outcome::Super,
        }
    }

    /// Run `state`'s entry action. Called by the engine only when a handler
    /// returned [`Outcome::Transition`] — never on [`Outcome::Handled`]. That
    /// distinction is load-bearing: `PreSupervision`'s cursor walk returns
    /// `Handled` precisely so this does not re-run and reset the cursor.
    fn entry_action(&mut self, state: State, ctx: &mut Sink<E>) {
        match state {
            State::PreSupervision => {
                // Recovery is a full platform re-boot: quiesce every live
                // component before the walk, so verification always covers code
                // at rest, never a component whose earlier pass says nothing
                // about what it is currently running. Empty at the initial
                // power-on walk (nothing live yet), so cold boot is unchanged.
                self.quiesce_all(ctx);
                let _ = self.advance_to_next_ungated(ctx, 0);
            }
            State::Updating => {
                // A new update supersedes any activated-but-not-committed image;
                // the prior commit window is void.
                self.pending_commit = false;
                ctx.emit(Effect::AuthenticateUpdate);
                ctx.emit(Effect::StageUpdate);
            }
            State::Recovering(failed) => {
                // Recovery voids any activated-but-not-committed image: the
                // running image is now under suspicion, so its commit window
                // ends here.
                self.pending_commit = false;
                // The component under recovery is being restored, not booting;
                // drop any pending boot-progress watchdog so a late `Timeout`
                // can't re-enter recovery for it. It is also held (not live)
                // while the platform restores it, so it is not quiesced again on
                // the re-walk.
                self.clear_awaiting_boot(failed);
                // `retry` here is the consecutive-recovery count for `failed`
                // (0 on the first attempt); hand it to the driver so it need not
                // track its own. It is bumped later, on `Restored`.
                let attempt = match self.status_index(failed) {
                    Some(i) => {
                        self.statuses[i].released = false;
                        self.statuses[i].retry
                    }
                    None => 0,
                };
                ctx.emit(Effect::RecoverComponent {
                    id: failed,
                    attempt,
                });
            }
            State::Locked => {
                ctx.emit(Effect::LatchLockdown);
            }
            State::Ready => {
                // NB: `gated` is intentionally NOT cleared here. It is a durable
                // trust-boundary gate (each id has a live `AssertReset`);
                // clearing it would let a later chain walk re-release a
                // component that policy deliberately isolated. Only a fresh
                // `Rot` on `PowerOnReset` clears it.
                //
                // Retry counts, by contrast, ARE cleared: a clean boot means no
                // recovery episode is in flight, so every per-component streak
                // resets to zero. (The `Isolated` lifecycle is preserved.)
                for s in self.statuses.iter_mut() {
                    s.retry = 0;
                }
            }
            _ => {}
        }
    }
}

/// Signals that the platform driver could not carry out an [`Effect`]. The machine does
/// not need the driver's error detail — **every** actuation failure is treated
/// the same, fail-closed: the orchestrator injects [`Event::EffectFailed`] and the
/// machine latches to [`State::Locked`]. This blanket policy is deliberate and
/// is what lets the failure signal stay a payload-less marker; a future design
/// that needs per-effect recovery must add a *new*, descriptive event rather
/// than widen this type. The driver logs the specifics on its side.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EffectError;

/// Outward connection to the platform. Carry out one effect, reporting
/// [`EffectError`] if it could not be performed. Never called with
/// [`Effect::Emit`] — the orchestrator consumes those internally.
///
/// `Ok(Some(event))` feeds back what the effect produced synchronously (e.g.
/// a verification verdict); the orchestrator queues it and settles it in the
/// same dispatch run. At most one event per effect. Synchronous results belong
/// here, not in a driver-side queue — one feedback path keeps ordering honest.
/// `execute` may block until the effect completes; results that only arrive
/// later (boot progress, timer expiry) are delivered as their own outside
/// events via `dispatch`.
///
/// Failure stays on the error channel, never in a returned event: `Err` is
/// checked between effects, so a failed actuation aborts the rest of the
/// batch — a feedback event cannot do that.
///
/// Contract the state machine relies on:
/// - **Honest, complete feedback.** The core's correctness rests entirely on
///   the event stream the driver feeds back; dropping, reordering, or
///   synthesizing events silently breaks the state machine's invariants.
/// - **Returned events quiesce.** Every returned event reports a result the
///   reducer consumes (its retry budgets bound re-verification cycles). An
///   executor that manufactures an event for every effect keeps one dispatch
///   run alive indefinitely.
/// - **`AssertReset` holds, it does not pulse.** A reset must keep the component
///   quiesced and non-executing until its matching `ReleaseReset`. The core's
///   at-rest verification guarantee depends on this: it re-asserts reset on
///   every live component before a recovery re-walk (`quiesce_all`) so that
///   `VerifyFirmware` covers code that cannot run or rewrite its own flash
///   between the check and the release. A reset that merely pulses would let a
///   component resume before verification and void that guarantee.
/// - **A failed [`Effect::LatchLockdown`] is a hard fault.** Lockdown is the top
///   of the escalation ladder — the core has nothing stronger to emit and
///   will *believe* it is `Locked`. The driver must treat that failure as
///   terminal (halt/reset), not a recoverable error.
pub trait Platform {
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError>;
}

/// A handle for a caller's own event loop. Owns the machine's storage
/// ([`Rot`]) and its current [`State`], and drives them through [`step`].
///
/// [`step`]: Self::step
pub struct Orchestrator<const N: usize, const E: usize> {
    rot: Rot<N, E>,
    state: State,
}

impl<const N: usize, const E: usize> Orchestrator<N, E> {
    pub fn new(chain: Chain<N>, max_retry: u8) -> Self {
        Self {
            rot: Rot::new(chain, max_retry),
            // The initial state. `PowerOnReset` has no entry action, so there is
            // nothing to run here before the first event.
            state: State::PowerOnReset,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// Whether `state` is nested under the supervising handler
    /// ([`Rot::handle_supervising`]) — i.e. whether an [`Outcome::Super`] from
    /// its leaf handler has anywhere to go. The four supervised states are the
    /// ones the eRoT can be in after it has exited [`State::PreSupervision`],
    /// and before it locks down.
    const fn is_supervised(state: State) -> bool {
        matches!(
            state,
            State::AwaitingReady(_) | State::Ready | State::Updating | State::Recovering(_)
        )
    }

    /// Reduce one event: dispatch, fall through to the supervisor if needed, and
    /// apply the resulting outcome.
    ///
    /// The machine defines no exit actions and no supervisor entry/exit actions,
    /// so a transition's only side effect is the target state's entry action —
    /// and it lands in the *same* `ctx` as the handler that caused it, which the
    /// `E >= 2 * N + 2` bound relies on (a full gate cascade, with its paired
    /// isolation reports, plus the destination `PreSupervision` entry's two
    /// effects share one `Sink`).
    fn step(&mut self, event: &Event, ctx: &mut Sink<E>) {
        // 0. Single point of id-membership enforcement. The core supervises
        //    only the components in the configured chain, so an event that names
        //    an id the chain does not contain is dropped here, before any
        //    handler runs — no handler needs its own membership check, and none
        //    can act on a component the core never modeled. Events that name no
        //    component (`component_id() == None`) always pass through.
        if let Some(id) = event.component_id()
            && !self.rot.in_chain(id)
        {
            return;
        }

        // 1. Dispatch to the current (leaf) state.
        let mut outcome = self.rot.handle(self.state, event, ctx);

        // 2. On `Super`, defer to the supervising handler if this state has one;
        //    otherwise the event is discarded. Discarding is what keeps `Locked`
        //    terminal: it is unsupervised, so its blanket `Super` drops every
        //    event — including the `EffectFailed` from a failed `LatchLockdown`,
        //    which would otherwise loop.
        if let Outcome::Super = outcome
            && Self::is_supervised(self.state)
        {
            outcome = self.rot.handle_supervising(event, ctx);
        }

        // 3. Apply a transition. `Handled` and an unhandled `Super` both leave
        //    the state alone and run no entry action.
        if let Outcome::Transition(target) = outcome {
            self.state = target;
            self.rot.entry_action(target, ctx);
        }
    }

    /// Handle one event all the way through — every [`Effect::Emit`]
    /// follow-up and every event the executors return — calling `on_effect`
    /// for each external effect in order. One call runs to quiescence.
    ///
    /// If `on_effect` reports an [`EffectError`], the orchestrator injects an
    /// [`Event::EffectFailed`] at the *front* of the queue, so a failed
    /// actuation is handled fail-closed: the latch settles next, and feedback
    /// still queued behind it drains into [`State::Locked`] (discarded)
    /// instead of actuating hardware after a failure. A pending-queue
    /// overflow is handled the same way: losing a returned event would break
    /// the honest-feedback contract, so the run latches instead.
    pub fn dispatch_with(
        &mut self,
        event: Event,
        mut on_effect: impl FnMut(Effect) -> Result<Option<Event>, EffectError>,
    ) {
        // Fail-closed latch: `EffectFailed` goes to the *front*, so it settles
        // next and everything still queued drains into `Locked` (discarded)
        // instead of actuating hardware after a failure. Prefer evicting the
        // newest queued event over losing the latch itself.
        fn latch(pending: &mut heapless::Deque<Event, PENDING_CAP>) {
            if pending.is_full() {
                pending.pop_back();
            }
            // Dead Err arm: the eviction above guarantees room.
            let _ = pending.push_front(Event::EffectFailed);
        }

        let mut pending: heapless::Deque<Event, PENDING_CAP> = heapless::Deque::new();
        // Dead Err arm: `pending` is empty and `PENDING_CAP >= 3` (asserted at
        // build time), so the first push always fits.
        let _ = pending.push_back(event);
        // `EffectFailed` is injected at most once: it is idempotent and
        // terminal (drives to `Locked`, which discards everything after). The
        // only external effect executed after it settles is `Locked`'s own
        // entry, whose failure must not inject again.
        let mut failed = false;

        while let Some(ev) = pending.pop_front() {
            let mut buf = Sink::<E>::new();
            self.step(&ev, &mut buf);

            for &effect in buf.effects() {
                let follow_up = match effect {
                    // Internal: handle next, never forwarded to the platform.
                    Effect::Emit(internal) => Some(internal),
                    external => match on_effect(external) {
                        Ok(follow_up) => follow_up,
                        Err(_) => {
                            // Fail-closed AND fail-fast: abandon the rest of
                            // this batch. `step` has already advanced the
                            // state as if the whole batch applied, and the
                            // latch overrides that transition, so nothing
                            // ordered after the failure may hit hardware.
                            if !failed {
                                failed = true;
                                latch(&mut pending);
                            }
                            break;
                        }
                    },
                };
                if let Some(next) = follow_up
                    && pending.push_back(next).is_err()
                    && !failed
                {
                    // Queue full: `next` would be lost, breaking the
                    // honest-feedback contract. Fail closed instead.
                    failed = true;
                    latch(&mut pending);
                    break;
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
mod tests;
