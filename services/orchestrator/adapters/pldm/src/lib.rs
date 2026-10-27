// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM-backed adapter for the Boot Orchestrator's update-request input.
//!
//! [`UpdateRequestLatch`] binds the PLDM firmware-device service's
//! [`FdEventSink`] seam to the orchestrator's
//! [`Event::UpdateRequest`]: the PLDM run loop notifies the latch when the
//! Update Agent's `RequestUpdate` is accepted, and the orchestrator run loop
//! drains it with [`take`](UpdateRequestLatch::take). This crate depends on
//! both stacks by design — the PLDM service stays orchestrator-free and the
//! orchestrator stays transport-free, the same rule that keeps HAL adapters
//! out of `orchestrator-capabilities`.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use openprot_orchestrator_sm::Event;
use openprot_pldm_service::firmware_device::{FdEvent, FdEventSink};

/// Latches an accepted PLDM `RequestUpdate` until the orchestrator run loop
/// drains it as [`Event::UpdateRequest`].
///
/// A `bool` latch, not a counter: the FD rejects a second `RequestUpdate`
/// while an update is in progress (`ALREADY_IN_UPDATE_MODE`), so at most one
/// accepted request can be outstanding per update cycle. Should a completed
/// or cancelled cycle admit a new `RequestUpdate` before the previous latch
/// is drained, the two coalesce into one [`Event::UpdateRequest`] — which is
/// what the state machine would do anyway (an update already being handled
/// defers further requests).
#[derive(Default)]
pub struct UpdateRequestLatch {
    pending: bool,
}

impl UpdateRequestLatch {
    /// A latch with nothing pending.
    pub const fn new() -> Self {
        Self { pending: false }
    }

    /// Drain the latch: [`Event::UpdateRequest`] if a `RequestUpdate` was
    /// accepted since the last call, else `None`.
    pub fn take(&mut self) -> Option<Event> {
        self.pending.then(|| {
            self.pending = false;
            Event::UpdateRequest
        })
    }
}

/// Latches [`FdEvent::UpdateRequested`]; other FD lifecycle events have no
/// orchestrator mapping yet and are dropped here by design.
impl FdEventSink for UpdateRequestLatch {
    fn notify(&mut self, event: FdEvent) {
        if matches!(event, FdEvent::UpdateRequested) {
            self.pending = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_latch_yields_nothing() {
        assert_eq!(UpdateRequestLatch::new().take(), None);
    }

    #[test]
    fn accepted_request_yields_one_event() {
        let mut latch = UpdateRequestLatch::new();
        latch.notify(FdEvent::UpdateRequested);
        assert_eq!(latch.take(), Some(Event::UpdateRequest));
        assert_eq!(latch.take(), None, "a drained latch must not re-fire");
    }

    #[test]
    fn undrained_notifications_coalesce() {
        let mut latch = UpdateRequestLatch::new();
        latch.notify(FdEvent::UpdateRequested);
        latch.notify(FdEvent::UpdateRequested);
        assert_eq!(latch.take(), Some(Event::UpdateRequest));
        assert_eq!(latch.take(), None);
    }
}
