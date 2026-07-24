// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.

/// What the orchestrator requires before it commits a staged image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPolicy {
    /// The device reports it came up.
    Liveness,
    /// Liveness plus SPDM re-attestation of the running image.
    LivenessAndAttestation,
}

/// One managed downstream device, as declared by the board config.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig {
    pub name: &'static str,
    /// Reset line id, passed to HalBootControl::new.
    pub reset_line: u8,
    /// How long the orchestrator waits for this device to report Booted
    /// before it declares a timeout.
    pub boot_timeout: core::time::Duration,
    pub commit_policy: CommitPolicy,
}

/// Checks a device table. Board configs call this in a const context so a
/// bad table fails the build.
pub const fn validate(devices: &[DeviceConfig]) {
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
