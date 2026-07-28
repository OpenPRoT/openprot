// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.

/// What the orchestrator requires before it commits a staged image.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): adding a variant is
/// a breaking change, so the compiler forces every match on the policy —
/// in particular the orchestrator's commit decision — to handle the new
/// variant explicitly instead of falling into a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPolicy {
    /// The device reports it came up.
    Liveness,
    /// Liveness plus SPDM re-attestation of the running image.
    LivenessAndAttestation,
}

/// One managed downstream device, as declared by the board config.
///
/// Generic over the board's reset signal type `R`, which must match the
/// `ResetId` of the reset controller behind the board's `BootControl`
/// implementation — the compiler rejects a table whose ids the controller
/// cannot accept.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): board tables
/// construct this struct by literal, which the attribute would forbid.
/// Adding a field is a breaking change that updates every board table.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig<R> {
    pub name: &'static str,
    /// Reset signal id, passed to HalBootControl::new.
    pub reset_signal: R,
    /// How long the orchestrator waits for this device to report Booted
    /// before it declares a timeout.
    pub boot_timeout: core::time::Duration,
    pub commit_policy: CommitPolicy,
}

/// Checks a device table. Board configs call this in a const context so a
/// bad table fails the build.
pub const fn validate<R>(devices: &[DeviceConfig<R>]) {
    let mut i = 0;
    while i < devices.len() {
        assert!(!devices[i].name.is_empty(), "device name must not be empty");
        assert!(
            !devices[i].boot_timeout.is_zero(),
            "boot timeout must not be zero"
        );
        i += 1;
    }
}
