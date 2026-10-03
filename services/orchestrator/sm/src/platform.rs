// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::model::{Effect, Event};

/// Signals that the platform driver could not carry out an [`Effect`]. The machine does
/// not need the driver's error detail — **every** actuation failure is treated
/// the same, fail-secure: the orchestrator injects [`Event::EffectFailed`] and the
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
/// `Ok(Some(event))` feeds back what the effect produced without blocking
/// (e.g. a verification verdict from an in-process check); the orchestrator
/// queues it and settles it in the same dispatch run. At most one event per
/// effect. Immediate results belong here, not in a driver-side queue,
/// so the orchestrator sees them in the order they were produced.
/// `execute` must return without blocking:
/// results that require I/O or arrive later (boot progress, timer expiry,
/// a verdict from a remote crypto service) are delivered as their own
/// outside events via `dispatch`.
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
/// - **[`Effect::RecoverComponent`] reports its verdict as an event, not an
///   `execute` error.** On success the driver feeds back
///   [`Event::Restored`]; when its configured recovery sources for that
///   component are exhausted, it feeds back [`Event::RecoveryUnavailable`]
///   instead — never [`EffectError`]. `EffectError` from a `RecoverComponent`
///   call is reserved for a genuine actuation fault (e.g. a bus error during
///   the image swap), which fails secure to [`State::Locked`] unconditionally.
///   Reporting "out of images" that way would lock the whole platform down
///   even for an `Isolable`/`Cascading` component, instead of letting it be
///   gated per [`FailurePolicy`] like the count-driven exhaustion path.
pub trait Platform {
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError>;
}
