// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

/// Identifies a firmware domain whose boot line the orchestrator controls.
///
/// This enum is `#[non_exhaustive]`: additional domains may be added as the
/// platform's domain set grows. Platform implementations must handle unknown
/// variants (e.g. via a wildcard arm) to remain forward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootTarget {
    /// The RoT firmware domain.
    RoT,
    /// The host-side boot-managed firmware domain (e.g. PCH or equivalent).
    HostTarget,
}

/// Observable platform state reported to the mailbox or status LEDs.
///
/// The SM names each state at the domain level. The platform impl maps each
/// variant to the concrete register value or LED code for the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformState {
    /// Pre-boot verification phase is active.
    PreBootVerify,
    /// Active firmware recovery is in progress.
    FirmwareRecovery,
    /// Firmware update is being applied.
    FirmwareUpdate,
    /// Platform is in runtime; boot holds released.
    Runtime,
    /// Platform is locked down due to authentication failure.
    Lockdown,
    /// Platform reboot has been initiated.
    Reboot,
}

/// A side-effect request emitted by the state machine for the platform to execute.
///
/// The SM never calls platform code directly. It pushes `Effect` values onto
/// [`Orchestrator::pending`]; the runner drains and dispatches them after each
/// [`sm.handle()`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    HoldBoot(BootTarget),
    ReleaseBoot(BootTarget),
    ArmWatchdog,
    DisarmWatchdog,
    ArmMonitors,
    DisarmMonitors,
    LogPanic,
    Reboot,
    HaltBoot,
    /// Notify the platform of the current observable state.
    ///
    /// The platform impl writes the corresponding code to the mailbox or LEDs.
    SetPlatformState(PlatformState),
}