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

/// Identifies one slot within one device's layout. Opaque: meaning comes
/// from the position in that device's slot table, never from a global
/// vocabulary — slot 0 on the BMC and slot 0 on the NIC are unrelated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotId(pub u8);

/// A distinguished duty a slot carries beyond being writable/bootable.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): adding a role is a
/// breaking change, so every consumer that dispatches on roles — in
/// particular recovery-candidate selection — is forced to handle the new
/// role explicitly instead of falling into a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRole {
    /// The slot recovery falls back to after every ordinary rung failed.
    /// A "golden" image is this role plus `writable: false` — a property
    /// combination, not a separate name.
    Recovery,
}

/// One slot in a device's layout: topology as data, not a type. A layout
/// is plain A/B, A/B + golden, or single + golden purely by what the table
/// declares — no layout shape is named anywhere.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    pub id: SlotId,
    /// May the update path write this slot? `false` on a recovery-role
    /// slot is what makes it "golden".
    pub writable: bool,
    /// May the device boot from this slot? Every bootable slot is a rung
    /// of the recovery ladder.
    pub bootable: bool,
    pub role: Option<SlotRole>,
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
    /// This device's slot layout. The recovery ladder is derived from it,
    /// never declared: bootable slots in declaration order, recovery-role
    /// slot last, escalation to out-of-band recovery once no rung is left.
    /// A layout without rungs — e.g. empty, for a device that owns its
    /// boot selection internally (the PLDM archetype) — leaves escalation
    /// as the only step.
    pub slots: &'static [Slot],
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

        let slots = devices[i].slots;
        let mut bootable = 0;
        let mut recovery_slots = 0;
        let mut s = 0;
        while s < slots.len() {
            if slots[s].bootable {
                bootable += 1;
            }
            if matches!(slots[s].role, Some(SlotRole::Recovery)) {
                recovery_slots += 1;
                assert!(slots[s].bootable, "a recovery-role slot must be bootable");
            }
            let mut t = s + 1;
            while t < slots.len() {
                assert!(
                    slots[s].id.0 != slots[t].id.0,
                    "slot ids must be unique within a device"
                );
                t += 1;
            }
            s += 1;
        }
        assert!(
            recovery_slots <= 1,
            "at most one recovery-role slot per device"
        );
        assert!(
            slots.is_empty() || bootable > 0,
            "a non-empty slot layout needs a bootable slot"
        );
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

    /// An ordinary slot: writable, bootable, no role.
    const fn slot(id: u8) -> Slot {
        Slot {
            id: SlotId(id),
            writable: true,
            bootable: true,
            role: None,
        }
    }

    /// A recovery-role slot; non-writable, but no test depends on that.
    const fn recovery_slot(id: u8) -> Slot {
        Slot {
            writable: false,
            role: Some(SlotRole::Recovery),
            ..slot(id)
        }
    }

    const DEVICE: DeviceConfig<u8, u8> = DeviceConfig {
        name: "dev",
        reset_signal: 0,
        checkpoints: &[CHECKPOINT],
        commit_policy: CommitPolicy::Liveness,
        slots: &[slot(0), slot(1)],
    };

    #[test]
    fn accepts_a_valid_table() {
        validate(&[DEVICE]);
    }

    #[test]
    fn accepts_a_layout_with_a_recovery_slot() {
        validate(&[DeviceConfig {
            slots: const { &[slot(0), slot(1), recovery_slot(2)] },
            ..DEVICE
        }]);
    }

    #[test]
    fn accepts_an_empty_layout() {
        validate(&[DeviceConfig {
            slots: &[],
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "slot ids must be unique")]
    fn rejects_duplicate_slot_ids() {
        validate(&[DeviceConfig {
            slots: const { &[slot(0), slot(0)] },
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "at most one recovery-role slot")]
    fn rejects_two_recovery_role_slots() {
        validate(&[DeviceConfig {
            slots: const { &[slot(0), recovery_slot(1), recovery_slot(2)] },
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "recovery-role slot must be bootable")]
    fn rejects_an_unbootable_recovery_slot() {
        validate(&[DeviceConfig {
            slots: const {
                &[
                    slot(0),
                    slot(1),
                    Slot {
                        bootable: false,
                        ..recovery_slot(2)
                    },
                ]
            },
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "needs a bootable slot")]
    fn rejects_a_layout_with_no_bootable_slot() {
        validate(&[DeviceConfig {
            slots: const {
                &[Slot {
                    bootable: false,
                    ..slot(0)
                }]
            },
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
