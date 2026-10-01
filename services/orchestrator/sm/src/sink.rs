// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::model::{Effect, State};

// Internal capacities — these follow from how the machine works, not from the
// deployment. The board owns `N` (chain length), `E` (effect-buffer size) and
// max_retry.

/// Upper bound on events queued at once inside `dispatch_with`. The queue
/// pops as it settles, so this bounds *in-flight* events, not a run's total
/// length: one batch can queue at most one `Emit` follow-up plus one returned
/// event per external effect. Executors that return an event for many effects
/// of one batch can overflow this; overflow is fail-closed (see
/// `dispatch_with`), never silent loss.
pub(crate) const PENDING_CAP: usize = 8;

/// Compile-time floor: room for an `Emit` follow-up, a returned event, and
/// the injected `EffectFailed`. Evaluated at build time (an anonymous
/// `const`), so an under-sized `PENDING_CAP` fails to compile.
const _: () = assert!(
    PENDING_CAP >= 3,
    "PENDING_CAP must hold an Emit follow-up + a returned event + EffectFailed",
);

/// Result of dispatching one event to a state (or its superstate).
pub(crate) enum Outcome {
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
    pub(crate) fn new() -> Self {
        Self {
            effects: heapless::Vec::new(),
        }
    }

    /// Append one effect. `E` is sized so overflow is impossible for a machine
    /// that compiles: `Rot::EFFECT_CAP_OK` proves `E >= 2 * N + 2` and no
    /// handler emits more than `2 * N + 2` effects into one `Sink`, so the push
    /// below can never fail. A `cfg(test)` assert catches a stale derivation
    /// in the test suite; in the release binary the push is unchecked
    /// (fail-closed: fewer effects means more lockdown, never less).
    ///
    /// The driver runs the effects from one handler in the order they were
    /// emitted, and it does not run them as a single all-or-nothing group: if
    /// one effect fails, the ones before it have already happened. When an
    /// effect fails, the driver stops there, it skips the rest and injects
    /// `EffectFailed` so the machine locks down. Stopping partway is still safe
    /// no matter what order the effects were in: a component is only ever
    /// released after it has passed verification, and every other effect either
    /// tightens things (holds a component in reset, or latches lockdown) or just
    /// reports what happened. So a batch that stops early can only leave the
    /// platform more locked down, never less. A skipped report costs information,
    /// not containment, and the batch was cut short by a failure that latches
    /// lockdown anyway, which is the louder signal.
    pub fn emit(&mut self, effect: Effect) {
        let ok = self.effects.push(effect).is_ok();
        // Compile-time proof (EFFECT_CAP_OK) makes this unreachable; the
        // cfg(test) assert catches a stale derivation during testing.
        // debug_assert covers non-Bazel builds where debug_assertions is on.
        debug_assert!(
            ok,
            "Sink overflowed E={E}, EFFECT_CAP_OK derivation is stale"
        );
        #[cfg(test)]
        assert!(
            ok,
            "Sink overflowed E={E}, EFFECT_CAP_OK derivation is stale"
        );
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
}
