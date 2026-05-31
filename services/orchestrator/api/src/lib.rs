// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # PRoT Lifecycle State Machine — API
//!
//! Target-agnostic state, event, and transition definitions for the OpenPRoT
//! platform lifecycle (secure-boot / verify / recovery / update / runtime).
//!
//! This crate is the Rust port of the architecture used by ASPEED's
//! `AspeedStateMachine`: a single-threaded, event-driven finite state machine
//! whose states form the PRoT boot-and-runtime lifecycle. Only the *shape* is
//! carried over — the transition graph and the "a handler runs work, then emits
//! the next event" drive model. The Zephyr `smf.h` machinery, `union` event
//! payloads, and manual allocation are replaced with plain Rust enums and a
//! pure transition function.
//!
//! ## What lives here vs. the `sm` crate
//!
//! This crate holds *only* data and pure logic: the [`State`] and [`Event`]
//! enums and the [`transition`] function. It has no run-loop, no queue, no
//! actions, and no OS or transport dependencies — so the transition graph can
//! be reasoned about and tested in isolation. The run-loop and the work
//! handlers live in `openprot_orchestrator_sm`.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod wire;

/// States of the PRoT lifecycle.
///
/// The graph mirrors the ASPEED PFR lifecycle but trimmed to what an OpenPRoT
/// target needs for first bring-up. Intel-specific and seamless-update states
/// are intentionally omitted; add them behind the same [`transition`] function
/// if a target requires them.
///
/// Hierarchical parent states (`Tmin1` / `Tzero` in the original) are not
/// encoded as enum variants here. Shared entry/exit behaviour is instead the
/// responsibility of the `Actions` implementation in the `sm` crate, which can
/// branch on the target state. This keeps the transition table flat and total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum State {
    /// Power-on entry state. The machine sits here until [`Event::Start`].
    Boot,
    /// One-time platform/RoT initialization.
    Init,
    /// The RoT booted from its secondary image and must self-recover.
    RotRecovery,
    /// Authenticate the active platform firmware image.
    FirmwareVerify,
    /// Restore platform firmware from a known-good recovery image.
    FirmwareRecovery,
    /// Apply a staged firmware update.
    FirmwareUpdate,
    /// Provisioning has not been performed; await a provisioning command.
    Unprovisioned,
    /// Normal operation: firmware is authenticated and running.
    Runtime,
    /// Terminal safe state — refuse to release the platform from reset.
    Lockdown,
    /// Request a platform reset.
    Reboot,
}

/// Events that drive the lifecycle.
///
/// Events are produced both externally (commands, watchdog, reset detection)
/// and internally by `Actions` handlers reporting the outcome of work they
/// performed (e.g. [`Event::VerifyDone`] after a successful verify). The latter
/// is what re-drives the loop, exactly as `GenerateStateMachineEvent` did in
/// the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// Kick the machine out of [`State::Boot`].
    Start,
    /// Initialization completed successfully.
    InitDone,
    /// The RoT was found running from its secondary image during init.
    InitRotSecondaryBooted,
    /// Verification determined the device is unprovisioned.
    VerifyUnprovisioned,
    /// Firmware verification failed authentication.
    VerifyFailed,
    /// Firmware verification succeeded.
    VerifyDone,
    /// A recovery attempt failed.
    RecoveryFailed,
    /// A recovery attempt succeeded.
    RecoveryDone,
    /// A firmware update was requested (e.g. update-on-reset intent).
    UpdateRequested,
    /// A firmware update completed successfully, carrying which region was
    /// updated so the machine can decide whether a reboot is required.
    UpdateDone(UpdateScope),
    /// A firmware update failed.
    UpdateFailed,
    /// A provisioning command was received.
    ProvisionCmd,
    /// A handshake (e.g. attestation/SPDM) with a platform component failed.
    HandshakeFailed,
}

/// What an applied update modified, reported by [`Event::UpdateDone`].
///
/// Ports the PRD's RoT-active-update reboot guard (§4.7, §6.1): an update that
/// replaces the RoT's own active region only takes effect after a reboot, so it
/// routes to [`State::Reboot`]; a platform-only update re-verifies in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateScope {
    /// The update touched only platform firmware; re-verify and continue.
    Platform,
    /// The update replaced the RoT active region; a reboot is required to run
    /// the newly written RoT firmware.
    RotActive,
}

/// Compute the next state for a `(state, event)` pair.
///
/// Returns `Some(next)` when the pair is a defined transition, or `None` when
/// the event is not meaningful in the current state. A `None` result means
/// "discard the event and stay put" — the same semantics as the `default:`
/// arms in the original `switch` statements. Keeping this total and pure makes
/// the entire transition graph unit-testable without a queue, clock, or
/// hardware.
pub fn transition(state: State, event: Event) -> Option<State> {
    use Event::*;
    use State::*;

    Some(match (state, event) {
        (Boot, Start) => Init,

        (Init, InitDone) => FirmwareVerify,
        (Init, InitRotSecondaryBooted) => RotRecovery,

        (RotRecovery, RecoveryDone) => Reboot,
        (RotRecovery, RecoveryFailed) => Lockdown,

        (FirmwareVerify, UpdateRequested) => FirmwareUpdate,
        (FirmwareVerify, VerifyUnprovisioned) => Unprovisioned,
        (FirmwareVerify, VerifyFailed) => FirmwareRecovery,
        (FirmwareVerify, VerifyDone) => Runtime,
        (FirmwareVerify, RecoveryFailed | HandshakeFailed) => Lockdown,

        (FirmwareRecovery, RecoveryDone) => FirmwareVerify,
        (FirmwareRecovery, RecoveryFailed) => Lockdown,

        (FirmwareUpdate, UpdateDone(UpdateScope::RotActive)) => Reboot,
        (FirmwareUpdate, UpdateDone(UpdateScope::Platform)) => FirmwareVerify,
        (FirmwareUpdate, UpdateFailed) => FirmwareRecovery,

        (Unprovisioned, ProvisionCmd) => Init,

        (Runtime, UpdateRequested) => FirmwareUpdate,
        (Runtime, VerifyFailed) => FirmwareRecovery,

        // No transition defined for this pair: discard the event.
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_starts_into_init() {
        assert_eq!(transition(State::Boot, Event::Start), Some(State::Init));
    }

    #[test]
    fn unknown_event_is_discarded() {
        // Boot only reacts to Start; anything else is a no-op.
        assert_eq!(transition(State::Boot, Event::VerifyDone), None);
    }

    #[test]
    fn verify_failure_routes_to_recovery() {
        assert_eq!(
            transition(State::FirmwareVerify, Event::VerifyFailed),
            Some(State::FirmwareRecovery)
        );
    }

    #[test]
    fn repeated_recovery_failure_locks_down() {
        assert_eq!(
            transition(State::FirmwareRecovery, Event::RecoveryFailed),
            Some(State::Lockdown)
        );
    }

    #[test]
    fn successful_update_revalidates() {
        // After a platform update we re-verify rather than trusting the new image.
        assert_eq!(
            transition(
                State::FirmwareUpdate,
                Event::UpdateDone(UpdateScope::Platform)
            ),
            Some(State::FirmwareVerify)
        );
    }

    #[test]
    fn rot_active_update_reboots() {
        // Updating the RoT's own active region requires a reboot to take effect.
        assert_eq!(
            transition(
                State::FirmwareUpdate,
                Event::UpdateDone(UpdateScope::RotActive)
            ),
            Some(State::Reboot)
        );
    }

    /// Every defined edge of the transition graph, asserted exhaustively so a
    /// change to the table is a deliberate, reviewable diff. Each row is one
    /// `(from, event) -> to` fact from `transition`.
    #[test]
    fn every_defined_transition_is_correct() {
        use Event::*;
        use State::*;

        let edges = [
            (Boot, Start, Init),
            (Init, InitDone, FirmwareVerify),
            (Init, InitRotSecondaryBooted, RotRecovery),
            (RotRecovery, RecoveryDone, Reboot),
            (RotRecovery, RecoveryFailed, Lockdown),
            (FirmwareVerify, UpdateRequested, FirmwareUpdate),
            (FirmwareVerify, VerifyUnprovisioned, Unprovisioned),
            (FirmwareVerify, VerifyFailed, FirmwareRecovery),
            (FirmwareVerify, VerifyDone, Runtime),
            (FirmwareVerify, RecoveryFailed, Lockdown),
            (FirmwareVerify, HandshakeFailed, Lockdown),
            (FirmwareRecovery, RecoveryDone, FirmwareVerify),
            (FirmwareRecovery, RecoveryFailed, Lockdown),
            (FirmwareUpdate, UpdateDone(UpdateScope::RotActive), Reboot),
            (FirmwareUpdate, UpdateDone(UpdateScope::Platform), FirmwareVerify),
            (FirmwareUpdate, UpdateFailed, FirmwareRecovery),
            (Unprovisioned, ProvisionCmd, Init),
            (Runtime, UpdateRequested, FirmwareUpdate),
            (Runtime, VerifyFailed, FirmwareRecovery),
        ];

        for (from, event, to) in edges {
            assert_eq!(
                transition(from, event),
                Some(to),
                "expected {from:?} --{event:?}--> {to:?}",
            );
        }
    }

    /// Terminal states have no outgoing edges: any event is discarded.
    #[test]
    fn terminal_states_have_no_outgoing_edges() {
        use Event::*;
        use State::*;

        let every_event = [
            Start,
            InitDone,
            InitRotSecondaryBooted,
            VerifyUnprovisioned,
            VerifyFailed,
            VerifyDone,
            RecoveryFailed,
            RecoveryDone,
            UpdateRequested,
            UpdateDone(UpdateScope::Platform),
            UpdateDone(UpdateScope::RotActive),
            UpdateFailed,
            ProvisionCmd,
            HandshakeFailed,
        ];

        for state in [Lockdown, Reboot] {
            for event in every_event {
                assert_eq!(
                    transition(state, event),
                    None,
                    "{state:?} must discard {event:?} (terminal state)",
                );
            }
        }
    }

    /// A handful of representative non-edges: events that are meaningful in
    /// some state but not the one they are paired with here are discarded
    /// rather than silently routed somewhere.
    #[test]
    fn undefined_pairs_are_discarded() {
        use Event::*;
        use State::*;

        let non_edges = [
            (Boot, VerifyDone),
            (Init, VerifyFailed),
            (RotRecovery, VerifyDone),
            (FirmwareVerify, Start),
            (FirmwareRecovery, VerifyDone),
            (FirmwareUpdate, VerifyDone),
            (Unprovisioned, VerifyDone),
            (Runtime, RecoveryDone),
        ];

        for (state, event) in non_edges {
            assert_eq!(
                transition(state, event),
                None,
                "{state:?} should not react to {event:?}",
            );
        }
    }
}
