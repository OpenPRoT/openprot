// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::model::{Chain, Effect, Event, State};
use crate::platform::{EffectError, Platform};
use crate::rot::Rot;
use crate::sink::{Outcome, Sink, PENDING_CAP};

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
            && self.state.is_supervised()
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
    /// actuation is handled fail-secure: the latch settles next, and feedback
    /// still queued behind it drains into [`State::Locked`] (discarded)
    /// instead of actuating hardware after a failure. A pending-queue
    /// overflow is handled the same way: losing a returned event would break
    /// the honest-feedback contract, so the run latches instead.
    pub fn dispatch_with(
        &mut self,
        event: Event,
        mut on_effect: impl FnMut(Effect) -> Result<Option<Event>, EffectError>,
    ) {
        // Fail-secure latch: `EffectFailed` goes to the *front*, so it settles
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
                            // Fail-secure AND fail-fast: abandon the rest of
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
                    // honest-feedback contract. Fail secure instead.
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
