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

/// How the orchestrator observes a device's boot-progress signal.
///
/// Generic over the id type `G` the board's boot monitor uses to read a
/// boot-complete line, for the same reason `DeviceConfig` is generic over
/// its reset signal: signal ids are board-specific.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): adding a signal
/// kind is a breaking change, so every consumer that dispatches on it is
/// forced to handle the new kind explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootSignal<G> {
    /// The device raises a boot-complete GPIO line.
    GpioBootComplete(G),
    /// The device sends a heartbeat message.
    Heartbeat,
    /// The device's MCTP endpoint answers as ready.
    MctpReady,
    /// The device answers a firmware version query.
    VersionQuery,
}

/// One boot-progress checkpoint: a signal the orchestrator waits for, and
/// how long it waits.
#[derive(Debug, Clone, Copy)]
pub struct BootCheckpoint<G> {
    /// Names the checkpoint in timeout reports.
    pub name: &'static str,
    pub signal: BootSignal<G>,
    /// How long the orchestrator waits for `signal` before it declares the
    /// checkpoint — and the device's boot — failed. Expiry is the
    /// orchestrator's own judgment; hung devices report nothing.
    pub window: core::time::Duration,
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
pub struct DeviceConfig<R, G: 'static> {
    pub name: &'static str,
    /// Reset signal id, passed to HalBootControl::new.
    pub reset_signal: R,
    /// Boot-progress checkpoints, in the order the device passes them.
    /// The device counts as booted when the last one is reached; a
    /// checkpoint whose window expires fails the boot.
    pub checkpoints: &'static [BootCheckpoint<G>],
    pub commit_policy: CommitPolicy,
}

/// Checks a device table. Board configs call this in a const context so a
/// bad table fails the build.
pub const fn validate<R, G>(devices: &[DeviceConfig<R, G>]) {
    let mut i = 0;
    while i < devices.len() {
        assert!(!devices[i].name.is_empty(), "device name must not be empty");
        assert!(
            !devices[i].checkpoints.is_empty(),
            "device must declare at least one boot checkpoint"
        );
        let mut c = 0;
        while c < devices[i].checkpoints.len() {
            assert!(
                !devices[i].checkpoints[c].name.is_empty(),
                "checkpoint name must not be empty"
            );
            assert!(
                !devices[i].checkpoints[c].window.is_zero(),
                "checkpoint window must not be zero"
            );
            c += 1;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    // Board tables run validate() at compile time, where a rejection is a
    // build error nobody can assert on. These tests call it at runtime to
    // prove the reject paths actually fire — a vacuous loop would pass
    // every `const _` check silently.

    const CHECKPOINT: BootCheckpoint<u8> = BootCheckpoint {
        name: "boot-complete",
        signal: BootSignal::GpioBootComplete(0),
        window: Duration::from_secs(1),
    };

    const DEVICE: DeviceConfig<u8, u8> = DeviceConfig {
        name: "dev",
        reset_signal: 0,
        checkpoints: &[CHECKPOINT],
        commit_policy: CommitPolicy::Liveness,
    };

    #[test]
    fn accepts_a_valid_table() {
        validate(&[DEVICE]);
    }

    #[test]
    #[should_panic(expected = "device name must not be empty")]
    fn rejects_an_empty_device_name() {
        validate(&[DEVICE, DeviceConfig { name: "", ..DEVICE }]);
    }

    #[test]
    #[should_panic(expected = "at least one boot checkpoint")]
    fn rejects_an_empty_checkpoint_list() {
        validate(&[DeviceConfig {
            checkpoints: &[],
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "checkpoint name must not be empty")]
    fn rejects_an_empty_checkpoint_name() {
        validate(&[DeviceConfig {
            checkpoints: &[BootCheckpoint {
                name: "",
                ..CHECKPOINT
            }],
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "checkpoint window must not be zero")]
    fn rejects_a_zero_checkpoint_window() {
        validate(&[DeviceConfig {
            checkpoints: &[
                CHECKPOINT,
                BootCheckpoint {
                    window: Duration::ZERO,
                    ..CHECKPOINT
                },
            ],
            ..DEVICE
        }]);
    }
}
