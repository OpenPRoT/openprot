// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OpenPRoT Resiliency state machine.
//!
//! The state machine is hierarchical with top-level parent states (`Boot`, `Init`, etc.)
//! and child states (`FirmwareVerify`, `Runtime`, etc.). Transitions are event-driven
//! and may depend on verification results, recovery status, or external commands.
//!
//! This crate is `#![no_std]` (std is enabled only for the test build).
//!
//! It is built on the [`statig`] hierarchical-state-machine crate. The [`Orchestrator`]
//! struct is both the *implementation target* (its methods become states/superstates)
//! and the *shared context* (its fields persist across transitions).
//!
//! # State hierarchy
//!
//! ```text
//! Boot
//! Init
//! RotRecovery
//! BootGate               (superstate)
//! ├── FirmwareVerify
//! ├── FirmwareRecovery
//! ├── FirmwareUpdate
//! └── SystemLockdown
//! OperationalPhase      (superstate)
//! ├── Unprovisioned
//! ├── Runtime
//! ├── SeamlessUpdate
//! └── SeamlessVerify
//! SystemReboot
//! ```
#![cfg_attr(not(test), no_std)]

pub mod effect;
pub mod platform;

use effect::{BootTarget, Effect};
use heapless::Vec;
use statig::prelude::*;

/// Maximum number of firmware recovery attempts before the platform locks down.
pub const MAX_RECOVERY_ATTEMPTS: u8 = 3;

/// Outcome of authenticating a set of firmware images.
///
/// Produced by an external verifier and delivered to the state machine via
/// [`Event::VerifyComplete`]. The variant drives which child of `BootGate` runs next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyResult {
    /// All images authenticated successfully.
    Valid,
    /// An active image failed but a known-good recovery image is available.
    Recoverable,
    /// A pending firmware update is staged and should be applied.
    UpdatePending,
    /// Unrecoverable authentication or security failure.
    Fatal,
}

/// Events that drive the Resiliency state machine.
///
/// Events originate from hardware signals (`PowerOn`, `WatchdogTimeout`), from the
/// results of long-running operations (`VerifyComplete`, `RecoveryComplete`,
/// `UpdateComplete`), or from external commands (`SeamlessUpdateRequested`,
/// `RebootRequested`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// External startup signal; begins the boot sequence.
    PowerOn,
    /// RoT initialization finished. `hrot_ok` reports RoT integrity.
    InitComplete { hrot_ok: bool },
    /// A firmware verification pass completed with the given result.
    VerifyComplete(VerifyResult),
    /// A recovery flow (HROT or firmware) completed.
    RecoveryComplete { success: bool },
    /// A firmware update flow completed. `rot_updated` is true when the RoT
    /// active region was overwritten and a reboot is required to execute
    /// from the new image.
    UpdateComplete { success: bool, rot_updated: bool },
    /// Provisioning finished; secure keys are now installed.
    Provisioned,
    /// The runtime watchdog expired.
    WatchdogTimeout,
    /// External command requesting a non-blocking seamless host-target update.
    SeamlessUpdateRequested,
    /// The seamless update payload has been written and is ready to verify.
    SeamlessApplied,
    /// External command requesting a forced platform reboot.
    RebootRequested,
}

/// Shared context and state-machine implementation target for the resiliency orchestrator.
///
/// Fields here survive across transitions and are mutated by entry actions and
/// state handlers via `&mut self`.
#[derive(Debug, Default)]
pub struct Orchestrator {
    /// Consecutive firmware recovery attempts within the current `BootGate` pass.
    pub recovery_attempts: u8,
    /// Total number of completed platform boots.
    pub boot_count: u32,
    /// Whether the platform currently holds valid provisioning keys.
    pub provisioned: bool,
    /// Pending side-effect requests for the runner to drain and dispatch.
    pub(crate) pending: Vec<Effect, 16>,
    /// Ordered log of every `(source, target)` transition; populated by the
    /// `after_transition` hook. Only present in test builds.
    #[cfg(test)]
    pub trace: std::vec::Vec<(State, State)>,
}

impl Orchestrator {
    /// Drain all pending [`Effect`] values produced since the last call.
    ///
    /// The runner should call this after every [`StateMachine::handle`] and
    /// dispatch each effect to the [`ResiliencyPlatform`] implementation.
    pub fn drain_effects(&mut self) -> impl Iterator<Item = Effect> + '_ {
        self.pending.drain(..)
    }
}

#[state_machine(
    initial = "State::boot()",
    state(derive(Debug, Clone, PartialEq, Eq)),
    superstate(derive(Debug, Clone, PartialEq, Eq)),
    after_transition = "Self::log_transition"
)]
impl Orchestrator {
    // ---- Top-level states -------------------------------------------------

    /// Initial idle state; system awaiting startup signal.
    #[state(entry_action = "on_enter_boot")]
    fn boot(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::PowerOn => Transition(State::init()),
            _ => Handled,
        }
    }

    /// System initialization and HROT setup.
    #[state]
    fn init(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::InitComplete { hrot_ok: true } => Transition(State::firmware_verify()),
            Event::InitComplete { hrot_ok: false } => Transition(State::rot_recovery()),
            _ => Handled,
        }
    }

    /// HROT firmware recovery flow.
    #[state]
    fn rot_recovery(&mut self, event: &Event) -> Outcome<State> {
        match event {
            // RoT region restored: must reboot to execute from the new image.
            Event::RecoveryComplete { success: true } => Transition(State::system_reboot()),
            // HROT cannot be restored: the root of trust is unusable.
            Event::RecoveryComplete { success: false } => Transition(State::system_lockdown()),
            _ => Handled,
        }
    }

    /// Force platform reboot. This is terminal from the state machine's
    /// perspective; control does not return to the current boot session.
    #[state(entry_action = "on_enter_system_reboot")]
    fn system_reboot(&mut self, event: &Event) -> Outcome<State> {
        let _ = event;
        Handled
    }

    // ---- BootGate: pre-boot verification, recovery, and update -------------

    /// Pre-boot parent state. Provides shared handling for its children.
    #[superstate(entry_action = "on_enter_boot_gate")]
    fn boot_gate(&mut self, event: &Event) -> Outcome<State> {
        match event {
            // A reboot command is honored from anywhere in the pre-boot phase.
            Event::RebootRequested => Transition(State::system_reboot()),
            _ => Handled,
        }
    }

    /// Authenticate RoT and host-target firmware images.
    #[state(superstate = "boot_gate")]
    fn firmware_verify(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::VerifyComplete(VerifyResult::Valid) => {
                // Images are good. Enter runtime, but route through provisioning
                // first if the platform has no secure keys.
                if self.provisioned {
                    Transition(State::runtime())
                } else {
                    Transition(State::unprovisioned())
                }
            }
            Event::VerifyComplete(VerifyResult::Recoverable) => {
                Transition(State::firmware_recovery())
            }
            Event::VerifyComplete(VerifyResult::UpdatePending) => {
                Transition(State::firmware_update())
            }
            Event::VerifyComplete(VerifyResult::Fatal) => Transition(State::system_lockdown()),
            _ => Super,
        }
    }

    /// Restore corrupted active or recovery images.
    #[state(superstate = "boot_gate", entry_action = "on_enter_firmware_recovery")]
    fn firmware_recovery(&mut self, event: &Event) -> Outcome<State> {
        match event {
            // Recovered: re-authenticate the restored image.
            Event::RecoveryComplete { success: true } => Transition(State::firmware_verify()),
            // Recovery failed: retry until the attempt budget is exhausted.
            Event::RecoveryComplete { success: false } => {
                if self.recovery_attempts >= MAX_RECOVERY_ATTEMPTS {
                    Transition(State::system_lockdown())
                } else {
                    Transition(State::firmware_recovery())
                }
            }
            _ => Super,
        }
    }

    /// Apply pending firmware updates.
    #[state(superstate = "boot_gate")]
    fn firmware_update(&mut self, event: &Event) -> Outcome<State> {
        match event {
            // RoT active region updated: must reboot to execute from new image.
            Event::UpdateComplete { success: true, rot_updated: true } => {
                Transition(State::system_reboot())
            }
            // Host-target or peripheral update: re-verify the written image.
            Event::UpdateComplete { success: true, rot_updated: false } => {
                Transition(State::firmware_verify())
            }
            // Update failed: fall back to recovery.
            Event::UpdateComplete { success: false, .. } => Transition(State::firmware_recovery()),
            _ => Super,
        }
    }

    /// Fatal security failure; halt boot. Only a power cycle restarts the platform.
    #[state(superstate = "boot_gate", entry_action = "on_enter_system_lockdown")]
    fn system_lockdown(&mut self, event: &Event) -> Outcome<State> {
        match event {
            // Inherit `boot_gate`'s reboot handling; ignore everything else (halt).
            Event::RebootRequested => Super,
            _ => Handled,
        }
    }

    // ---- OperationalPhase: release boot holds and enter runtime ----------

    /// Operational parent state. Provides shared handling for its children.
    #[superstate(entry_action = "on_enter_operational_phase", exit_action = "on_exit_operational_phase")]
    fn operational_phase(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::RebootRequested => Transition(State::system_reboot()),
            _ => Handled,
        }
    }

    /// Platform lacks secure keys; provisioning needed.
    #[state(superstate = "operational_phase")]
    fn unprovisioned(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::Provisioned => {
                self.provisioned = true;
                Transition(State::runtime())
            }
            _ => Super,
        }
    }

    /// Normal operation with watchdog monitoring.
    #[state(superstate = "operational_phase", entry_action = "on_enter_runtime", exit_action = "on_exit_runtime")]
    fn runtime(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::SeamlessUpdateRequested => Transition(State::seamless_update()),
            // A specific image failed its boot watchdog; recover it.
            Event::WatchdogTimeout => Transition(State::firmware_recovery()),
            _ => Super,
        }
    }

    /// Non-blocking host-target firmware update during runtime.
    #[state(superstate = "operational_phase")]
    fn seamless_update(&mut self, event: &Event) -> Outcome<State> {
        match event {
            Event::SeamlessApplied => Transition(State::seamless_verify()),
            _ => Super,
        }
    }

    /// Verify seamless update integrity.
    #[state(superstate = "operational_phase")]
    fn seamless_verify(&mut self, event: &Event) -> Outcome<State> {
        match event {
            // Integrity confirmed: resume normal operation.
            Event::VerifyComplete(VerifyResult::Valid) => Transition(State::runtime()),
            // Anything else means the seamless image is bad: drop to pre-boot recovery.
            Event::VerifyComplete(_) => Transition(State::firmware_recovery()),
            _ => Super,
        }
    }

    // ---- Entry actions ---------------------------------------------------

    #[action]
    fn on_enter_boot(&mut self) {
        // Fresh boot pass: clear the per-pass recovery budget.
        self.recovery_attempts = 0;
    }

    #[action]
    fn on_enter_boot_gate(&mut self) {
        self.pending.push(Effect::HoldBoot(BootTarget::RoT)).ok();
        self.pending.push(Effect::HoldBoot(BootTarget::HostTarget)).ok();
    }

    #[action]
    fn on_enter_operational_phase(&mut self) {
        self.pending.push(Effect::ArmMonitors).ok();
        self.pending.push(Effect::ReleaseBoot(BootTarget::HostTarget)).ok();
    }

    #[action]
    fn on_exit_operational_phase(&mut self) {
        self.pending.push(Effect::DisarmMonitors).ok();
    }

    #[action]
    fn on_enter_firmware_recovery(&mut self) {
        self.recovery_attempts = self.recovery_attempts.saturating_add(1);
    }

    #[action]
    fn on_enter_runtime(&mut self) {
        // Reaching runtime ends the pre-boot recovery pass.
        self.recovery_attempts = 0;
        self.pending.push(Effect::ArmWatchdog).ok();
    }

    #[action]
    fn on_exit_runtime(&mut self) {
        self.pending.push(Effect::DisarmWatchdog).ok();
    }

    #[action]
    fn on_enter_system_lockdown(&mut self) {
        self.pending.push(Effect::HaltBoot).ok();
    }

    #[action]
    fn on_enter_system_reboot(&mut self) {
        self.boot_count = self.boot_count.saturating_add(1);
        self.pending.push(Effect::LogPanic).ok();
        self.pending.push(Effect::Reboot).ok();
    }
}

#[cfg(test)]
impl Orchestrator {
    /// Called by statig after every transition; appends the `(source, target)`
    /// pair to [`Orchestrator::trace`] for test assertions.
    fn log_transition(&mut self, source: &State, target: &State, _context: &mut ()) {
        std::eprintln!("  transition: {source:?} → {target:?}");
        self.trace.push((source.clone(), target.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Boot → Init → BootGate::FirmwareVerify → OperationalPhase::Runtime, fully provisioned.
    #[test]
    fn happy_path_to_runtime() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::Valid));

        assert_eq!(sm.state(), &State::runtime());
        assert_eq!(sm.boot_count, 0);
    }

    /// An unprovisioned platform is routed through provisioning before runtime.
    #[test]
    fn unprovisioned_routes_through_provisioning() {
        let mut sm = Orchestrator::default()
            .uninitialized_state_machine()
            .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::Valid));
        assert_eq!(sm.state(), &State::unprovisioned());

        sm.handle(&Event::Provisioned);
        assert_eq!(sm.state(), &State::runtime());
        assert!(sm.provisioned);
    }

    /// Recovery retries up to the budget, then locks the platform down.
    #[test]
    fn recovery_exhaustion_locks_down() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::Recoverable));
        assert_eq!(sm.state(), &State::firmware_recovery());

        // Each failed attempt re-enters recovery until the budget is spent.
        for _ in 0..MAX_RECOVERY_ATTEMPTS {
            sm.handle(&Event::RecoveryComplete { success: false });
        }
        assert_eq!(sm.state(), &State::system_lockdown());
        assert_eq!(sm.recovery_attempts, MAX_RECOVERY_ATTEMPTS);
    }

    /// A failed seamless update drops back into the pre-boot recovery flow.
    #[test]
    fn seamless_failure_drops_to_recovery() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::Valid));
        sm.handle(&Event::SeamlessUpdateRequested);
        sm.handle(&Event::SeamlessApplied);
        assert_eq!(sm.state(), &State::seamless_verify());

        sm.handle(&Event::VerifyComplete(VerifyResult::Fatal));
        assert_eq!(sm.state(), &State::firmware_recovery());
    }

    /// The full transition sequence for the happy path matches the expected trace.
    #[test]
    fn happy_path_trace() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::Valid));

        assert_eq!(
            sm.trace,
            vec![
                (State::boot(), State::init()),
                (State::init(), State::firmware_verify()),
                (State::firmware_verify(), State::runtime()),
            ]
        );
    }

    /// `RebootRequested` is honored from within both BootGate and OperationalPhase (superstate handling).
    #[test]
    fn reboot_requested_from_superstates() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        // In BootGate::FirmwareVerify, bubble up to boot_gate's shared handler.
        sm.handle(&Event::RebootRequested);
        assert_eq!(sm.state(), &State::system_reboot());
        assert_eq!(sm.boot_count, 1);
    }

    /// RoT recovery success requires a reboot (not re-init) to execute from the new image.
    #[test]
    fn rot_recovery_success_reboots() {
        let mut sm = Orchestrator::default()
            .uninitialized_state_machine()
            .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: false });
        assert_eq!(sm.state(), &State::rot_recovery());

        sm.handle(&Event::RecoveryComplete { success: true });
        assert_eq!(sm.state(), &State::system_reboot());
        assert_eq!(sm.boot_count, 1);
    }

    /// Watchdog timeout in runtime triggers firmware recovery, not a blind reboot.
    #[test]
    fn watchdog_timeout_triggers_recovery() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::Valid));
        assert_eq!(sm.state(), &State::runtime());

        sm.handle(&Event::WatchdogTimeout);
        assert_eq!(sm.state(), &State::firmware_recovery());
    }

    /// Updating a non-RoT image re-verifies; updating the RoT active region reboots.
    #[test]
    fn update_complete_rot_updated_reboots() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::UpdatePending));
        assert_eq!(sm.state(), &State::firmware_update());

        sm.handle(&Event::UpdateComplete { success: true, rot_updated: true });
        assert_eq!(sm.state(), &State::system_reboot());
    }

    /// A host-target update success returns to verify, not reboot.
    #[test]
    fn update_complete_host_verifies() {
        let mut sm = Orchestrator {
            provisioned: true,
            ..Default::default()
        }
        .uninitialized_state_machine()
        .init();

        sm.handle(&Event::PowerOn);
        sm.handle(&Event::InitComplete { hrot_ok: true });
        sm.handle(&Event::VerifyComplete(VerifyResult::UpdatePending));
        assert_eq!(sm.state(), &State::firmware_update());

        sm.handle(&Event::UpdateComplete { success: true, rot_updated: false });
        // Returns to FirmwareVerify; a subsequent VerifyComplete(Valid) reaches runtime.
        assert_eq!(sm.state(), &State::firmware_verify());

        sm.handle(&Event::VerifyComplete(VerifyResult::Valid));
        assert_eq!(sm.state(), &State::runtime());
    }
}
