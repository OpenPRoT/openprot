// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.
//!
//! Invariants are enforced in the `const fn` constructors, so an invalid
//! table is a build error and there is no validate step to forget. Checks
//! on board-defined types belong next to the table that gives them
//! meaning (`services/orchestrator/test/devices.rs` shows the pattern).

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
}

impl<R, G> DeviceConfig<R, G> {
    /// Declares a managed device. `const`, so board tables run the checks
    /// at build time.
    ///
    /// # Panics
    ///
    /// Panics — a build error in const context — if `name` is empty, if
    /// `checkpoints` is empty, or if two checkpoints share a name
    /// (failure reports identify a checkpoint by name; a duplicate would
    /// make them ambiguous).
    #[must_use]
    pub const fn new(
        name: &'static str,
        reset_signal: R,
        checkpoints: &'static [BootCheckpoint<G>],
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
        Self {
            name,
            reset_signal,
            checkpoints,
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

    #[test]
    fn accepts_a_valid_table() {
        let device = DeviceConfig::new("dev", 0u8, &[CHECKPOINT]);
        assert_eq!(device.name(), "dev");
        assert_eq!(*device.reset_signal(), 0);
        assert_eq!(device.checkpoints().len(), 1);
        assert_eq!(device.checkpoints()[0].name(), "boot-complete");
        assert_eq!(*device.checkpoints()[0].signal(), 0);
        assert_eq!(device.checkpoints()[0].timeout(), Duration::from_secs(1));
    }

    #[test]
    #[should_panic(expected = "checkpoint names must be unique")]
    fn rejects_duplicate_checkpoint_names() {
        let _ = DeviceConfig::new("dev", 0u8, &[CHECKPOINT, CHECKPOINT_DUP]);
    }

    #[test]
    #[should_panic(expected = "device name must not be empty")]
    fn rejects_an_empty_device_name() {
        let _ = DeviceConfig::new("", 0u8, &[CHECKPOINT]);
    }

    #[test]
    #[should_panic(expected = "at least one boot checkpoint")]
    fn rejects_an_empty_checkpoint_list() {
        let _ = DeviceConfig::new("dev", 0u8, &[] as &[BootCheckpoint<u8>]);
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
