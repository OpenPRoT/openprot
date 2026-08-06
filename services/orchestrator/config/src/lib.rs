// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.
//!
//! Invariants are enforced in the `const fn` constructors, so an invalid
//! table is a build error and there is no validate step to forget. Checks
//! on board-defined types belong next to the table that gives them
//! meaning (`target/mock/devices.rs` shows the pattern).

#![cfg_attr(not(test), no_std)]

/// One boot checkpoint: a signal the orchestrator waits for, and how long
/// it waits. Retry policy is deliberately not table data: a retry
/// re-resets the device and re-runs the whole walk, so budgets are
/// per boot attempt and owned by the orchestrator state machine.
///
/// The signal is a board-defined id — the schema attaches no meaning to
/// it and names no signal kinds. Each board defines its own vocabulary (a
/// small enum: a GPIO line, a progress-register threshold, a message-path
/// readiness) and gives it meaning in its `EvidenceReader`. The id is a
/// defunctionalized evidence check: data in the table instead of a
/// function, so the table stays printable, comparable, const-checkable —
/// and could one day be generated instead of written.
///
/// Fields are private so a checkpoint that violates the schema is
/// unrepresentable: [`new`](Self::new) is the only way in, and it checks.
#[derive(Debug, Clone, Copy)]
pub struct BootCheckpoint<G> {
    name: &'static str,
    signal: G,
    timeout: core::time::Duration,
}

impl<G> BootCheckpoint<G> {
    /// Declares a checkpoint. `const`, so board tables run the checks at
    /// build time.
    ///
    /// # Panics
    ///
    /// Panics — a build error in const context — if `name` is empty or
    /// `timeout` is zero.
    #[must_use]
    pub const fn new(name: &'static str, signal: G, timeout: core::time::Duration) -> Self {
        assert!(!name.is_empty(), "checkpoint name must not be empty");
        assert!(!timeout.is_zero(), "checkpoint timeout must not be zero");
        Self {
            name,
            signal,
            timeout,
        }
    }

    /// Names the checkpoint in failure reports ("bl1", "kernel", …).
    /// Unique within a device's checkpoint list.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Board-defined signal id, resolved by the board's `EvidenceReader`
    /// (in `orchestrator-capabilities`). An id rather than a function, so
    /// the table stays pure data — the type-level docs say why.
    #[must_use]
    pub const fn signal(&self) -> &G {
        &self.signal
    }

    /// Window for one attempt at this checkpoint. Expiry is the boot
    /// walk's own judgment; hung devices report nothing.
    ///
    /// The orchestrator state machine never sees this value — it is
    /// clockless. The walk consumes the windows and reports expiry as a
    /// failed attempt; a component's whole boot timeout is nothing more
    /// than its walk over these windows, in order.
    #[must_use]
    pub const fn timeout(&self) -> core::time::Duration {
        self.timeout
    }
}

/// Identifies one slot within one device's layout. An opaque per-device
/// token, not an index: ids need only be unique within one device's table
/// ([`DeviceConfig::new`] enforces exactly that) — they are not required
/// to be contiguous, ordered, or to start at zero, and slot 0 on the BMC
/// and slot 0 on the NIC are unrelated. Ladder order comes from table
/// declaration order, never from id values.
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
///
/// Immutable, and only constructible through [`Slot::new`], which
/// enforces the per-slot invariant — an invalid slot is unrepresentable,
/// not merely rejected later. Rules that span a whole layout (unique ids,
/// one recovery slot, ladder rungs) stay in [`DeviceConfig::new`], which
/// sees the list.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    id: SlotId,
    writable: bool,
    bootable: bool,
    role: Option<SlotRole>,
}

impl Slot {
    /// Declares one slot. Const, so board tables still build at compile
    /// time — where a rejected slot is a build error, the same teeth as
    /// [`DeviceConfig::new`]. (A `Result`-returning constructor cannot
    /// build a `&'static` table; panicking in const context is how schema
    /// rules fail the build.)
    ///
    /// # Panics
    ///
    /// Panics if `role` is [`SlotRole::Recovery`] and `bootable` is
    /// `false` — recovery boots the device from that slot, so an
    /// unbootable recovery slot is a contradiction.
    pub const fn new(id: SlotId, writable: bool, bootable: bool, role: Option<SlotRole>) -> Self {
        assert!(
            !matches!(role, Some(SlotRole::Recovery)) || bootable,
            "a recovery-role slot must be bootable"
        );
        Self {
            id,
            writable,
            bootable,
            role,
        }
    }

    /// This slot's id, unique within the device (checked by
    /// [`DeviceConfig::new`]).
    pub const fn id(&self) -> SlotId {
        self.id
    }

    /// May the update path write this slot? `false` on a recovery-role
    /// slot is what makes it "golden".
    pub const fn writable(&self) -> bool {
        self.writable
    }

    /// May the device boot from this slot? Every bootable slot is a rung
    /// of the recovery ladder.
    pub const fn bootable(&self) -> bool {
        self.bootable
    }

    /// This slot's special role, if any.
    pub const fn role(&self) -> Option<SlotRole> {
        self.role
    }
}

/// One managed downstream device, as declared by the board config.
///
/// Generic over the board's reset signal type `R` (which must match the
/// `ResetId` of the reset controller behind the board's `BootControl`
/// implementation) and its boot-signal vocabulary `G`, for the same
/// reason: signal ids are board-specific.
///
/// Deliberately says nothing about attestation or commit requirements:
/// those follow from what kind of device this is (iRoT-backed or
/// symbiont, the orchestrator's `ComponentKind`), not from a table
/// setting — a second knob would only let the two disagree.
///
/// Fields are private so a device entry that violates the schema is
/// unrepresentable: [`new`](Self::new) is the only way in, and it checks.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig<R, G: 'static> {
    name: &'static str,
    reset_signal: R,
    checkpoints: &'static [BootCheckpoint<G>],
    slots: &'static [Slot],
}

impl<R, G> DeviceConfig<R, G> {
    /// Declares a managed device. `const`, so board tables run the checks
    /// at build time.
    ///
    /// # Panics
    ///
    /// Panics — a build error in const context — if `name` is empty, if
    /// `checkpoints` is empty, if two checkpoints share a name (failure
    /// reports identify a checkpoint by name; a duplicate would make them
    /// ambiguous), or if `slots` breaks a layout rule: unique ids, at
    /// most one recovery-role slot, and a bootable slot in any non-empty
    /// layout.
    #[must_use]
    pub const fn new(
        name: &'static str,
        reset_signal: R,
        checkpoints: &'static [BootCheckpoint<G>],
        slots: &'static [Slot],
    ) -> Self {
        assert!(!name.is_empty(), "device name must not be empty");
        assert!(
            !checkpoints.is_empty(),
            "device must declare at least one boot checkpoint"
        );
        let mut c = 0;
        while c < checkpoints.len() {
            let mut d = c + 1;
            while d < checkpoints.len() {
                assert!(
                    !str_eq(checkpoints[c].name, checkpoints[d].name),
                    "checkpoint names must be unique per device"
                );
                d += 1;
            }
            c += 1;
        }
        let mut bootable = 0;
        let mut recovery_slots = 0;
        let mut s = 0;
        while s < slots.len() {
            if slots[s].bootable() {
                bootable += 1;
            }
            // Recovery ⇒ bootable is enforced by Slot::new — an
            // unbootable recovery slot is unrepresentable here.
            if matches!(slots[s].role(), Some(SlotRole::Recovery)) {
                recovery_slots += 1;
            }
            let mut t = s + 1;
            while t < slots.len() {
                assert!(
                    slots[s].id().0 != slots[t].id().0,
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
        Self {
            name,
            reset_signal,
            checkpoints,
            slots,
        }
    }

    /// The device's name in reports and logs.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Reset signal id, passed to HalBootControl::new.
    #[must_use]
    pub const fn reset_signal(&self) -> &R {
        &self.reset_signal
    }

    /// Boot checkpoints, in the order the device passes them. The device
    /// counts as booted when the last one is reached; a checkpoint whose
    /// window expires fails the attempt — whether to retry or recover is
    /// the orchestrator's decision, not table data.
    #[must_use]
    pub const fn checkpoints(&self) -> &'static [BootCheckpoint<G>] {
        self.checkpoints
    }

    /// This device's slot layout. The recovery ladder is derived from it,
    /// never declared: bootable slots in declaration order, recovery-role
    /// slot last, escalation to out-of-band recovery once no rung is left.
    /// A layout without rungs — e.g. empty, for a device that owns its
    /// boot selection internally (the PLDM archetype) — leaves escalation
    /// as the only step.
    #[must_use]
    pub const fn slots(&self) -> &'static [Slot] {
        self.slots
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

    // Board tables run the constructors at compile time, where a
    // rejection is a build error nobody can assert on. These tests call
    // them at runtime to prove the reject paths actually fire.

    const CHECKPOINT: BootCheckpoint<u8> =
        BootCheckpoint::new("boot-complete", 0, Duration::from_secs(1));

    // Same name, different signal: each checkpoint is individually valid,
    // so the pair only trips the device-level duplicate check.
    const CHECKPOINT_DUP: BootCheckpoint<u8> =
        BootCheckpoint::new("boot-complete", 1, Duration::from_secs(1));

    /// An ordinary slot: writable, bootable, no role.
    const fn slot(id: u8) -> Slot {
        Slot::new(SlotId(id), true, true, None)
    }

    /// A recovery-role slot; non-writable, but no test depends on that.
    const fn recovery_slot(id: u8) -> Slot {
        Slot::new(SlotId(id), false, true, Some(SlotRole::Recovery))
    }

    #[test]
    fn accepts_a_valid_table() {
        let device = DeviceConfig::new("dev", 0u8, &[CHECKPOINT], const { &[slot(0), slot(1)] });
        assert_eq!(device.name(), "dev");
        assert_eq!(*device.reset_signal(), 0);
        assert_eq!(device.checkpoints().len(), 1);
        assert_eq!(device.checkpoints()[0].name(), "boot-complete");
        assert_eq!(*device.checkpoints()[0].signal(), 0);
        assert_eq!(device.checkpoints()[0].timeout(), Duration::from_secs(1));
        assert_eq!(device.slots().len(), 2);
    }

    #[test]
    fn accepts_a_layout_with_a_recovery_slot() {
        let _ = DeviceConfig::new(
            "dev",
            0u8,
            &[CHECKPOINT],
            const { &[slot(0), slot(1), recovery_slot(2)] },
        );
    }

    #[test]
    fn accepts_an_empty_layout() {
        let _ = DeviceConfig::new("dev", 0u8, &[CHECKPOINT], &[]);
    }

    #[test]
    #[should_panic(expected = "checkpoint names must be unique")]
    fn rejects_duplicate_checkpoint_names() {
        let _ = DeviceConfig::new("dev", 0u8, &[CHECKPOINT, CHECKPOINT_DUP], &[]);
    }

    #[test]
    #[should_panic(expected = "device name must not be empty")]
    fn rejects_an_empty_device_name() {
        let _ = DeviceConfig::new("", 0u8, &[CHECKPOINT], &[]);
    }

    #[test]
    #[should_panic(expected = "at least one boot checkpoint")]
    fn rejects_an_empty_checkpoint_list() {
        let _ = DeviceConfig::new("dev", 0u8, &[] as &[BootCheckpoint<u8>], &[]);
    }

    #[test]
    #[should_panic(expected = "slot ids must be unique")]
    fn rejects_duplicate_slot_ids() {
        let _ = DeviceConfig::new("dev", 0u8, &[CHECKPOINT], const { &[slot(0), slot(0)] });
    }

    #[test]
    #[should_panic(expected = "at most one recovery-role slot")]
    fn rejects_two_recovery_role_slots() {
        let _ = DeviceConfig::new(
            "dev",
            0u8,
            &[CHECKPOINT],
            const { &[slot(0), recovery_slot(1), recovery_slot(2)] },
        );
    }

    // The per-slot invariant fails at construction, before any list-level
    // check could run — an invalid slot is unrepresentable.
    #[test]
    #[should_panic(expected = "recovery-role slot must be bootable")]
    fn rejects_an_unbootable_recovery_slot_at_construction() {
        Slot::new(SlotId(2), false, false, Some(SlotRole::Recovery));
    }

    #[test]
    #[should_panic(expected = "needs a bootable slot")]
    fn rejects_a_layout_with_no_bootable_slot() {
        let _ = DeviceConfig::new(
            "dev",
            0u8,
            &[CHECKPOINT],
            const { &[Slot::new(SlotId(0), true, false, None)] },
        );
    }

    #[test]
    #[should_panic(expected = "checkpoint name must not be empty")]
    fn rejects_an_empty_checkpoint_name() {
        let _ = BootCheckpoint::new("", 0u8, Duration::from_secs(1));
    }

    #[test]
    #[should_panic(expected = "checkpoint timeout must not be zero")]
    fn rejects_a_zero_checkpoint_timeout() {
        let _ = BootCheckpoint::new("boot-complete", 0u8, Duration::ZERO);
    }
}
