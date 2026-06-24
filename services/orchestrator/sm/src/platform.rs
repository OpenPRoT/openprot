// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::effect::Effect;

/// Executes platform-level side effects requested by the resiliency state machine.
///
/// The runner calls [`execute`] for each [`Effect`] drained from
/// [`Orchestrator::pending`] after every [`sm.handle()`] call.
/// Config-gated behavior (seamless update, SPDM attestation, checkpoint
/// recovery) is implemented here, not in the state machine.
pub trait ResiliencyPlatform {
    fn execute(&mut self, effect: Effect);
}

/// No-op platform implementation for unit tests.
///
/// Discards all effects so tests can focus on state transitions without
/// requiring a hardware implementation.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) struct NoopPlatform;

#[cfg(test)]
impl ResiliencyPlatform for NoopPlatform {
    fn execute(&mut self, _: Effect) {}
}
