// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.

#![cfg_attr(not(test), no_std)]

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

/// One boot checkpoint: a signal the orchestrator waits for, how long it
/// waits per attempt, and how many failed attempts it tolerates.
///
/// `signal` is a board-defined id — the schema attaches no meaning to it
/// and names no signal kinds. Each board defines its own vocabulary (a
/// small enum: a GPIO line, a progress-register threshold, a message-path
/// readiness) and gives it meaning in its `EvidenceReader`. The id is a
/// defunctionalized evidence check: data in the table instead of a
/// function, so the table stays printable, comparable, const-checkable —
/// and could one day be generated instead of written.
#[derive(Debug, Clone, Copy)]
pub struct BootCheckpoint<G> {
    /// Names the checkpoint in failure reports ("bl1", "kernel", …).
    pub name: &'static str,
    /// Board-defined signal id, resolved by the board's `EvidenceReader`
    /// (in `orchestrator-capabilities`). An id rather than a function, so
    /// the table stays pure data — the type-level docs say why.
    pub signal: G,
    /// Window for one attempt at this checkpoint. Expiry is the
    /// orchestrator's own judgment; hung devices report nothing.
    pub timeout: core::time::Duration,
    /// Attempts allowed beyond the first before the failure is final.
    /// `0` means the one attempt is all the device gets.
    pub max_retries: u8,
}

/// One managed downstream device, as declared by the board config.
///
/// Generic over the board's reset signal type `R` (which must match the
/// `ResetId` of the reset controller behind the board's `BootControl`
/// implementation) and its boot-signal vocabulary `G`, for the same
/// reason: signal ids are board-specific.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): board tables
/// construct this struct by literal, which the attribute would forbid.
/// Adding a field is a breaking change that updates every board table.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig<R, G: 'static> {
    pub name: &'static str,
    /// Reset signal id, passed to HalBootControl::new.
    pub reset_signal: R,
    /// Boot checkpoints, in the order the device passes them. The device
    /// counts as booted when the last one is reached; a checkpoint whose
    /// window and retry budget are exhausted fails the boot.
    pub checkpoints: &'static [BootCheckpoint<G>],
    pub commit_policy: CommitPolicy,
}

/// Checks a device table. Board configs call this in a const context so a
/// bad table fails the build.
///
/// Only schema-shape checks are possible here; checks on the board's own
/// types (signal ranges, uniqueness of signal ids) belong next to the
/// table that defines their meaning, in a board-local `const fn` run
/// alongside this one — `target/mock/devices.rs` shows the pattern.
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
                !devices[i].checkpoints[c].timeout.is_zero(),
                "checkpoint timeout must not be zero"
            );
            // Failure reports identify a checkpoint by name; a duplicate
            // would make them ambiguous.
            let mut d = c + 1;
            while d < devices[i].checkpoints.len() {
                assert!(
                    !str_eq(
                        devices[i].checkpoints[c].name,
                        devices[i].checkpoints[d].name
                    ),
                    "checkpoint names must be unique per device"
                );
                d += 1;
            }
            c += 1;
        }
        i += 1;
    }
}

// `==` on `&str` is not const; compare bytes by hand.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
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
        signal: 0,
        timeout: Duration::from_secs(1),
        max_retries: 1,
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
    #[should_panic(expected = "checkpoint names must be unique")]
    fn rejects_duplicate_checkpoint_names() {
        validate(&[DeviceConfig {
            checkpoints: &[
                CHECKPOINT,
                BootCheckpoint {
                    signal: 1,
                    ..CHECKPOINT
                },
            ],
            ..DEVICE
        }]);
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
    #[should_panic(expected = "checkpoint timeout must not be zero")]
    fn rejects_a_zero_checkpoint_timeout() {
        validate(&[DeviceConfig {
            checkpoints: &[BootCheckpoint {
                timeout: Duration::ZERO,
                ..CHECKPOINT
            }],
            ..DEVICE
        }]);
    }
}
