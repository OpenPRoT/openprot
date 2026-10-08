// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use crate::model::*;
use crate::sink::{Outcome, Sink};
use crate::status::{ComponentLifecycle, ComponentStatus, Gating};

/// Shared storage: data that persists across events. `N` is the chain capacity
/// and `E` the effect-buffer size — both board choices; the core sets no
/// default. `E` must be at least `2 * N + 2` (enforced in [`Rot::new`]).
pub struct Rot<const N: usize, const E: usize> {
    chain: heapless::Vec<(ComponentId, ComponentAttrs), N>,
    /// Index into `chain` of the component currently under verification, or the
    /// past-the-end sentinel `chain.len()` once the walk is done. Only
    /// `chain[cursor]` can be released: a `VerificationPassed` for any other id
    /// is stale or out of turn and is dropped.
    ///
    /// While the walk runs (`PreSupervision` and `AwaitingReady`) the cursor
    /// never points at a gated component. Gating the component under
    /// verification therefore has to move the cursor past it, which
    /// [`handle_corruption_advancing`](Self::handle_corruption_advancing) does:
    /// a verdict already in flight then fails the `chain[cursor]` check and is
    /// dropped instead of releasing a component the cascade just isolated.
    /// `property_isolation_is_sticky_under_random_sequences` guards this.
    ///
    /// `Recovering` is the exception: `VerificationFailed` leaves the cursor on
    /// the failed component and a corruption report can gate it there. Entry to
    /// `PreSupervision` re-walks from 0, which restores the invariant.
    cursor: u8,
    /// One record per chain component (parallel to `chain` by index). Each
    /// [`ComponentStatus`] holds the component's service `lifecycle` (`Isolated`
    /// means gated out of the walk) and its consecutive failed-restore `retry`
    /// count, kept per component so interleaved recoveries never share a budget.
    /// Durable across a return to `Ready`; only a fresh `Rot` on `PowerOnReset`
    /// resets it.
    statuses: heapless::Vec<ComponentStatus, N>,
    max_retry: u8,
    /// The component whose update was activated and whose anti-rollback floor
    /// has not been committed yet. `None` when nothing is waiting to commit.
    /// Its own [`Event::BootConfirmed`] commits the floor and clears this; a
    /// [`Event::CommitTimeout`] while it is still set goes to
    /// [`State::Locked`].
    ///
    /// The window spans the walk that follows an activation: the device only
    /// boots the candidate once it is reset, so the window cannot close before
    /// that walk finishes. Every state it passes through answers
    /// [`Event::CommitTimeout`], including the unsupervised
    /// [`State::PreSupervision`], or a fire during the walk would be dropped
    /// and never come again. In a multi-component chain, the updated device
    /// can boot (and confirm) while the walk is still verifying later
    /// neighbours; `BootConfirmed` is only handled in `Ready`, so the
    /// emitter must re-raise or hold it until the walk finishes.
    ///
    /// Cleared on the matching [`Event::BootConfirmed`] (the window closes
    /// normally) and on entry to the two states that end the window by leaving
    /// `Ready` while still running, [`State::Updating`] (a superseding update)
    /// and [`State::Recovering`] (the running image is now suspect). Not cleared
    /// on `Ready` entry, because activation sets it on the way in.
    pending_commit: Option<ComponentId>,
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
            pending_commit: None,
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
    pub(crate) fn in_chain(&self, id: ComponentId) -> bool {
        self.status_index(id).is_some()
    }

    pub(crate) fn pending_commit(&self) -> Option<ComponentId> {
        self.pending_commit
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

    /// Recovery is over for `failed` without success: gate per policy
    /// (`Isolable`/`Cascading` skip; `Required` reports + latches `Locked`).
    /// Shared by the retry-cap path (`Restored`, count exhausted) and the
    /// platform's `RecoveryUnavailable` path, so the two can never diverge.
    fn exhaust_recovery(&mut self, ctx: &mut Sink<E>, failed: ComponentId) -> Outcome {
        match self.gate_by_policy(ctx, failed) {
            Gating::Gated => {
                self.clear_retry(failed);
                Outcome::Transition(State::PreSupervision)
            }
            // `Required`, or an unknown/missing id: report the component that
            // forced the halt, then lock down. The report precedes the
            // internal `Emit`, so it is actuated before the machine moves
            // toward `Locked`.
            Gating::NotGated => {
                ctx.emit(Effect::ReportRecoveryFailed(failed));
                ctx.emit(Effect::Emit(Event::RecoveryFailed));
                Outcome::Handled
            }
        }
    }

    /// Shared `CorruptionDetected` handling. Delegates the policy interpretation
    /// to [`gate_by_policy`](Self::gate_by_policy) so this path and the
    /// recovery-exhaustion path can never diverge:
    /// `Isolable`/`Cascading` → gate the component (single or cascade) and stay
    /// put, so a later re-walk skips it instead of silently re-releasing one we
    /// already found corrupt; `Required` → recover first (the halt-on-exhaustion
    /// decision happens later in `Recovering`).
    fn handle_corruption(&mut self, id: ComponentId, ctx: &mut Sink<E>) -> Outcome {
        // Already isolated: it is held in reset and was reported. Recovering
        // it restores a component the re-walk skips, and on exhaustion a
        // `Required` one locks the platform down over a cascade that was
        // already contained.
        if self.is_gated(id) {
            return Outcome::Handled;
        }
        match self.gate_by_policy(ctx, id) {
            Gating::Gated => Outcome::Handled,
            Gating::NotGated => Outcome::Transition(State::Recovering(id)),
        }
    }

    /// `CorruptionDetected` for `PreSupervision` and `AwaitingReady`, the two
    /// states that release off `chain[cursor]`. Gates by policy, then keeps the
    /// `cursor` invariant by moving it past the component under verification
    /// when the cascade gated it. A `Required` corruption gates nothing and
    /// returns `Transition(Recovering)` unchanged.
    ///
    /// Not called from `handle_supervising`: `Recovering`'s cursor is stale
    /// (`VerificationFailed` left it on the failed component), so advancing
    /// there would verify mid-recovery or reach `Ready` instead of re-walking.
    fn handle_corruption_advancing(&mut self, id: ComponentId, ctx: &mut Sink<E>) -> Outcome {
        let outcome = self.handle_corruption(id, ctx);
        let cursor_gated = self
            .chain
            .get(self.cursor as usize)
            .is_some_and(|(c, _)| self.is_gated(*c));
        if !matches!(outcome, Outcome::Handled) || !cursor_gated {
            return outcome;
        }
        let next_idx = (self.cursor as usize).saturating_add(1);
        if self.advance_to_next_ungated(ctx, next_idx) {
            Outcome::Handled
        } else {
            Outcome::Transition(State::Ready)
        }
    }

    /// Hold `root` and cascade-hold every component whose `depends_on`
    /// (transitively) names it. Emits `AssertReset` and `ReportIsolated` for
    /// each newly gated component, including `root` itself — every isolated
    /// device is reported, not just the one that failed.
    fn cascade_hold(&mut self, ctx: &mut Sink<E>, root: ComponentId) {
        // BFS over the components gated so far. `frontier` is also the visited
        // set: a component already on it is never queued again, so the walk
        // visits each component once and terminates even on a dependency
        // cycle.
        //
        // Traversal must not skip a component that is already gated. Its own
        // dependents may still be running, and they are only reachable through
        // it, so stopping there would leave a component whose dependency is
        // isolated out of reset. `gate_one` is idempotent, so re-visiting a
        // gated component emits nothing and only continues the walk.
        let mut frontier: heapless::Vec<ComponentId, N> = heapless::Vec::new();
        self.gate_one(ctx, root);
        let _ = frontier.push(root);
        let mut i = 0;
        while let Some(&holder) = frontier.get(i) {
            i += 1;
            let mut dependents: heapless::Vec<ComponentId, N> = heapless::Vec::new();
            for &(id, attrs) in self.chain.iter() {
                if attrs.depends_on == Some(holder) && !frontier.contains(&id) {
                    let _ = dependents.push(id);
                }
            }
            for id in dependents {
                self.gate_one(ctx, id);
                let _ = frontier.push(id);
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
    pub(crate) fn handle(&mut self, state: State, event: &Event, ctx: &mut Sink<E>) -> Outcome {
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
                // The commit window can span this walk: an activation enters
                // `PreSupervision` with the window open. `PreSupervision` is
                // unsupervised, so without this match arm the watchdog fire would be
                // dropped and never come again, leaving commit-or-lock
                // unenforced for the length of a boot.
                Event::CommitTimeout => {
                    if self.pending_commit.is_some() {
                        Outcome::Transition(State::Locked)
                    } else {
                        Outcome::Handled
                    }
                }
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
                    // A verdict from before the gating, for a component the
                    // cascade has since isolated: recovering it re-walks the
                    // chain for a device that stays held.
                    if self.is_gated(*id) {
                        return Outcome::Handled;
                    }
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
                Event::CorruptionDetected(id) => self.handle_corruption_advancing(*id, ctx),
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
                // Timeout always recovers: silence says nothing about the
                // image, so a fresh attempt is never futile. Stale ids
                // (component not awaiting boot) are dropped.
                Event::Timeout(id) => {
                    if self.is_awaiting_boot(*id) {
                        Outcome::Transition(State::Recovering(*id))
                    } else {
                        Outcome::Handled
                    }
                }
                // A device-reported boot failure. DeviceFatal means the
                // device itself declared the image unrecoverable by retry,
                // so Isolable/Cascading components are gated immediately
                // (no recovery budget burned); Required still recovers
                // because a different recovery source could help.
                // Non-fatal kinds recover unconditionally, like Timeout.
                Event::BootFailed {
                    id,
                    checkpoint,
                    kind,
                } => {
                    if !self.is_awaiting_boot(*id) {
                        return Outcome::Handled;
                    }
                    ctx.emit(Effect::ReportBootFailed {
                        id: *id,
                        checkpoint,
                        kind: *kind,
                    });
                    if *kind == BootFailureKind::DeviceFatal {
                        self.handle_corruption_advancing(*id, ctx)
                    } else {
                        Outcome::Transition(State::Recovering(*id))
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
                    // Same in-flight verdict as above: an isolated component
                    // does not enter recovery.
                    if self.is_gated(*id) {
                        return Outcome::Handled;
                    }
                    // Recovery is attempted first for every failure, regardless
                    // of the component's recovery-failure policy.
                    Outcome::Transition(State::Recovering(*id))
                }
                // Releases off `chain[cursor]` too, so the cursor must move
                // off a component the cascade gated. `Handled` keeps `awaiting`.
                Event::CorruptionDetected(id) => self.handle_corruption_advancing(*id, ctx),
                // DeviceFatal: the device declared the image unrecoverable.
                // Gate Isolable/Cascading immediately (cursor-advancing),
                // recover Required. Non-fatal kinds and Timeout fall through
                // to handle_supervising below.
                Event::BootFailed {
                    id,
                    kind: BootFailureKind::DeviceFatal,
                    checkpoint,
                } if self.is_awaiting_boot(*id) => {
                    ctx.emit(Effect::ReportBootFailed {
                        id: *id,
                        checkpoint,
                        kind: BootFailureKind::DeviceFatal,
                    });
                    self.handle_corruption_advancing(*id, ctx)
                }
                // `Timeout` and non-fatal `BootFailed` fall through to
                // `handle_supervising`, which runs the device-agnostic
                // boot-progress watchdog uniformly across every supervised
                // state.
                _ => Outcome::Super,
            },

            State::Ready => match event {
                Event::UpdateRequest(id) => Outcome::Transition(State::Updating(*id)),
                // Proven-boot checkpoint: the image authenticated at
                // `ActivateUpdate`, but the SVN floor only advances now, once
                // the driver reports it healthy. Handled in place — confirming a
                // running image is not a state change. Closing the window clears
                // `pending_commit` so a later `CommitTimeout` cannot lock a
                // device that has already committed.
                Event::BootConfirmed(id) => {
                    if self.pending_commit == Some(*id) {
                        ctx.emit(Effect::CommitSvnFloor(*id));
                        self.pending_commit = None;
                    }
                    Outcome::Handled
                }
                // Commit watchdog. If the activated-but-not-committed window is
                // still open, fail secure: never commit an unproven image, and
                // never leave the downgrade window open indefinitely. Outside
                // the window this is a stale watchdog fire and is dropped.
                Event::CommitTimeout => {
                    if self.pending_commit.is_some() {
                        Outcome::Transition(State::Locked)
                    } else {
                        Outcome::Handled
                    }
                }
                _ => Outcome::Super,
            },

            State::Updating(target) => match event {
                Event::UpdateVerified => {
                    ctx.emit(Effect::ActivateUpdate);
                    // Open the activated-but-not-committed window: the floor is
                    // NOT advanced here; it waits for `BootConfirmed`. The
                    // driver arms its commit watchdog on `ActivateUpdate`, and
                    // `CommitTimeout` bounds this window (commit-or-lock).
                    self.pending_commit = Some(target);
                    // Re-walk rather than returning to `Ready`. Activation only
                    // proposes the image; the device runs the old one until it
                    // is reset, and `PreSupervision` entry quiesces every live
                    // component before verifying at rest. That reset is what
                    // boots the candidate, and the walk's `VerifyFirmware` is
                    // what records its SVN, which the floor commit then takes.
                    // Without it a `BootConfirmed` would commit the previous
                    // image's SVN and leave the downgrade window open.
                    Outcome::Transition(State::PreSupervision)
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
                        // Retries exhausted: gate via the same shared arm the
                        // platform's `RecoveryUnavailable` path uses, so the
                        // two can never disagree.
                        self.exhaust_recovery(ctx, failed)
                    }
                }
                Event::RecoveryUnavailable(id) => {
                    if *id != failed {
                        return Outcome::Handled; // same guard as Restored
                    }
                    // Authoritative: the platform is out of sources, so the
                    // machine does not wait for the retry count to run out
                    // (does not call `bump_retry`).
                    self.exhaust_recovery(ctx, failed)
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
    /// state handles [`Event::CorruptionDetected`] in its own arm, via
    /// [`handle_corruption_advancing`](Self::handle_corruption_advancing), since
    /// routing it here would also pull in the attestation behavior above.
    /// `AwaitingReady` does the same; the cursor rationale is on the helper.
    /// CSA defines no mechanism guaranteeing a corruption report
    /// arrives for an already-released component's *live, executing* state (its
    /// only at-rest mechanism — background NVM integrity polling — is explicitly
    /// scoped to "at rest"/"between boots", not an in-progress boot's chain
    /// walk), but that only means such a report isn't guaranteed to exist — it's
    /// not a reason to discard one if it does arrive. See
    /// `corruption_during_presupervision_selfloop_triggers_recovery` in
    /// `tests.rs`.
    pub(crate) fn handle_supervising(&mut self, event: &Event, ctx: &mut Sink<E>) -> Outcome {
        match event {
            Event::AttestationChallenge => {
                ctx.emit(Effect::SignAttestation);
                Outcome::Handled
            }
            // Reached from `Ready` and `Recovering` only: `PreSupervision`,
            // `AwaitingReady` and `Updating` handle this in their own arms.
            Event::CorruptionDetected(id) => self.handle_corruption(*id, ctx),
            // Boot-progress signals arriving after the walk left `PreSupervision`
            // / `AwaitingReady` (e.g. once the machine is already `Ready`): clear
            // the component's watchdog. `ComponentReady` is the active tier,
            // `Booted` the passive tier; both just retire the outstanding wait.
            Event::ComponentReady(id) | Event::Booted(id) => {
                self.clear_awaiting_boot(*id);
                Outcome::Handled
            }
            // Timeout always recovers: silence is ambiguous, so retrying
            // is never futile. Stale ids (already booted) are dropped.
            Event::Timeout(id) => {
                if self.is_awaiting_boot(*id) {
                    Outcome::Transition(State::Recovering(*id))
                } else {
                    Outcome::Handled
                }
            }
            // Same DeviceFatal split as PreSupervision/AwaitingReady, but
            // non-advancing: Ready/Recovering/Updating are not walking the
            // chain, so the cursor is not the walk position.
            Event::BootFailed {
                id,
                checkpoint,
                kind,
            } => {
                if !self.is_awaiting_boot(*id) {
                    return Outcome::Handled;
                }
                ctx.emit(Effect::ReportBootFailed {
                    id: *id,
                    checkpoint,
                    kind: *kind,
                });
                if *kind == BootFailureKind::DeviceFatal {
                    self.handle_corruption(*id, ctx)
                } else {
                    Outcome::Transition(State::Recovering(*id))
                }
            }
            // A new update cannot start from any supervised state except
            // `Ready` (the machine is mid-walk, mid-update, or mid-recovery).
            // `Ready` accepts `UpdateRequest` upstream in `handle` (→ `Updating`)
            // and so never reaches here; this arm therefore fires only for
            // `AwaitingReady`, `Updating`, and `Recovering`. Report the refusal
            // rather than dropping it silently, and leave the in-flight
            // walk/update/recovery untouched (`Handled`, no transition).
            Event::UpdateRequest(_) => {
                ctx.emit(Effect::ReportUpdateDeferred);
                Outcome::Handled
            }
            // Same window, same reason as `PreSupervision`'s match arm: the walk
            // that follows an activation passes through the supervised
            // states too, and a dropped watchdog fire never returns.
            Event::CommitTimeout => {
                if self.pending_commit.is_some() {
                    Outcome::Transition(State::Locked)
                } else {
                    Outcome::Handled
                }
            }
            Event::EffectFailed => Outcome::Transition(State::Locked),
            _ => Outcome::Super,
        }
    }

    /// Run `state`'s entry action. Called by the engine only when a handler
    /// returned [`Outcome::Transition`] — never on [`Outcome::Handled`]. That
    /// distinction is load-bearing: `PreSupervision`'s cursor walk returns
    /// `Handled` precisely so this does not re-run and reset the cursor.
    pub(crate) fn entry_action(&mut self, state: State, ctx: &mut Sink<E>) {
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
            State::Updating(_) => {
                // A new update supersedes any activated-but-not-committed image;
                // the prior commit window is void.
                self.pending_commit = None;
                ctx.emit(Effect::AuthenticateStageUpdate);
            }
            State::Recovering(failed) => {
                // Recovery voids any activated-but-not-committed image: the
                // running image is now under suspicion, so its commit window
                // ends here.
                self.pending_commit = None;
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
