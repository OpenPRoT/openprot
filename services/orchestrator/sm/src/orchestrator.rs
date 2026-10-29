// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::model::{Chain, Effect, Event, State};
use crate::platform::{EffectError, Platform};
use crate::rot::Rot;
use crate::sink::{Outcome, Sink, PENDING_CAP};

/// A handle the caller drives from its own event loop. Owns the state
/// machine storage ([`Rot`]) and the current [`State`], and passes both to
/// [`step`].
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

    /// Handles one event: dispatch to the current state, fall through to the
    /// supervisor if needed, and apply the outcome.
    ///
    /// The state machine defines no exit actions and no supervisor
    /// entry/exit actions, so the one side effect of a transition is the
    /// entry action for the target state. That action lands in the *same*
    /// `ctx` as the handler that caused it, which the `E >= 2 * N + 2` bound
    /// relies on: a full gate cascade, with the isolation reports it pairs
    /// each gate with, plus the two effects the destination `PreSupervision`
    /// entry emits, all share one `Sink`.
    fn step(&mut self, event: &Event, ctx: &mut Sink<E>) {
        // 0. The core checks chain membership here, once. It supervises only
        //    the components in the configured chain, so an event naming an id
        //    the chain does not contain is dropped before any handler runs.
        //    Each handler can then skip its own membership check and cannot
        //    act on a component the core never modeled. An event naming no
        //    component (`component_id() == None`) passes through.
        if let Some(id) = event.component_id()
            && !self.rot.in_chain(id)
        {
            return;
        }

        // 1. Dispatch to the current (leaf) state.
        let mut outcome = self.rot.handle(self.state, event, ctx);

        // 2. On `Super`, defer to the supervising handler where the current
        //    state has one, and discard the event where it does not.
        //    Discarding is what keeps `Locked` terminal: `Locked` is
        //    unsupervised, so its blanket `Super` drops every event,
        //    including the `EffectFailed` a failed `LatchLockdown` injects,
        //    which would otherwise loop.
        if let Outcome::Super = outcome
            && self.state.is_supervised()
        {
            outcome = self.rot.handle_supervising(event, ctx);
        }

        // 3. Apply a transition. On `Handled` and on an unhandled `Super`,
        //    nothing writes `self.state` and no entry action runs.
        if let Outcome::Transition(target) = outcome {
            self.state = target;
            self.rot.entry_action(target, ctx);
        }
    }

    /// Handles one event all the way through, calling `on_effect` for each
    /// external effect in order. One call runs to quiescence: it covers every
    /// [`Effect::Emit`] follow-up and every event the executors return.
    ///
    /// If `on_effect` reports an [`EffectError`], the orchestrator injects an
    /// [`Event::EffectFailed`] at the *front* of the queue, so a failed
    /// actuation is handled fail secure: `EffectFailed` settles next, and
    /// [`State::Locked`] discards the feedback still queued behind it rather
    /// than actuating hardware after a failure. A pending-queue overflow
    /// takes the same path: losing a returned event would break the
    /// honest-feedback contract, so the run injects `EffectFailed` instead.
    pub fn dispatch_with(
        &mut self,
        event: Event,
        mut on_effect: impl FnMut(Effect) -> Result<Option<Event>, EffectError>,
    ) {
        // The front of the queue, for the ordering the doc comment above
        // describes. Evicts the newest queued event rather than losing
        // `EffectFailed` itself when the queue is full.
        fn inject_effect_failed(pending: &mut heapless::Deque<Event, PENDING_CAP>) {
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
        // terminal, driving to `Locked`, which discards everything after. The
        // one external effect that runs after it settles is the entry action
        // for `Locked`, and a failure there must not inject again.
        let mut failed = false;

        while let Some(ev) = pending.pop_front() {
            let mut buf = Sink::<E>::new();
            self.step(&ev, &mut buf);

            for &effect in buf.effects() {
                let follow_up = match effect {
                    // Internal: handled next, and the orchestrator never
                    // forwards it to the platform.
                    Effect::Emit(internal) => Some(internal),
                    external => match on_effect(external) {
                        Ok(follow_up) => follow_up,
                        Err(_) => {
                            // Fail secure and fail fast: abandon the rest of
                            // this batch. `step` has already advanced the
                            // state as if the whole batch applied, and
                            // `EffectFailed` overrides that transition, so
                            // nothing ordered after the failure may hit
                            // hardware.
                            if !failed {
                                failed = true;
                                inject_effect_failed(&mut pending);
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
                    // honest-feedback contract. Fail secure instead.
                    failed = true;
                    inject_effect_failed(&mut pending);
                    break;
                }
            }
        }
    }

    /// Same as [`dispatch_with`], with the effects routed to a [`Platform`].
    pub fn dispatch(&mut self, platform: &mut impl Platform, event: Event) {
        self.dispatch_with(event, |effect| platform.execute(effect));
    }
}
