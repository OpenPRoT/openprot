// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.

#![cfg_attr(not(test), no_std)]

use orchestrator_capabilities::BootStatus;

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

/// One boot checkpoint: timing policy plus the evidence check itself.
///
/// The check is handed the board's device context `D`, so the channel
/// underneath it (a GPIO line, a progress register, a message path) stays
/// inside the check and a checkpoint nothing can observe is
/// unrepresentable.
///
/// `passed` is a capture-less `fn` pointer rather than a closure: a table
/// of closures each capturing `&mut D` cannot exist, while the walker
/// holding the one `&mut D` and passing it in can — and capture-less
/// closures coerce to `fn` in const tables. The division of state:
/// per-checkpoint parameters belong in the `fn` body, per-device and
/// per-board state belongs in `D`.
pub struct BootCheckpoint<D: ?Sized, E> {
    /// Names the checkpoint in failure reports ("bl1", "kernel", …).
    pub name: &'static str,
    /// Window for one attempt at this checkpoint. Expiry is the
    /// orchestrator's own judgment; hung devices report nothing.
    pub timeout: core::time::Duration,
    /// Attempts allowed beyond the first before the failure is final.
    pub max_retries: u8,
    /// The evidence check. The status must describe the current boot
    /// cycle — see [`BootStatus`] for the latching contract.
    pub passed: fn(&mut D) -> Result<BootStatus, E>,
}

// Manual impls: deriving would demand `D: Clone`/`D: Debug` bounds the
// fields never need (`D` only appears behind the `fn` pointer).
impl<D: ?Sized, E> Clone for BootCheckpoint<D, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: ?Sized, E> Copy for BootCheckpoint<D, E> {}

impl<D: ?Sized, E> core::fmt::Debug for BootCheckpoint<D, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootCheckpoint")
            .field("name", &self.name)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

/// One managed downstream device, as declared by the board config.
///
/// Generic over the board's reset signal type `R` (which must match the
/// `ResetId` of the reset controller behind the board's `BootControl`
/// implementation), the board's device context `D` every evidence check
/// receives, and the board-wide check error `E` — one context and one
/// error type per table, both board-defined.
///
/// Intentionally exhaustive (not `#[non_exhaustive]`): board tables
/// construct this struct by literal, which the attribute would forbid.
/// Adding a field is a breaking change that updates every board table.
pub struct DeviceConfig<R, D: ?Sized + 'static, E: 'static> {
    pub name: &'static str,
    /// Reset signal id, passed to HalBootControl::new.
    pub reset_signal: R,
    /// Boot checkpoints, in the order the device passes them. The device
    /// counts as booted when the last one is reached; a checkpoint whose
    /// window and retry budget are exhausted fails the boot.
    pub checkpoints: &'static [BootCheckpoint<D, E>],
    pub commit_policy: CommitPolicy,
}

// Manual impls for the same reason as BootCheckpoint's: only `R` is held
// by value, so only `R` gets a bound.
impl<R: Clone, D: ?Sized, E> Clone for DeviceConfig<R, D, E> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            reset_signal: self.reset_signal.clone(),
            checkpoints: self.checkpoints,
            commit_policy: self.commit_policy,
        }
    }
}

impl<R: Copy, D: ?Sized, E> Copy for DeviceConfig<R, D, E> {}

impl<R: core::fmt::Debug, D: ?Sized, E> core::fmt::Debug for DeviceConfig<R, D, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DeviceConfig")
            .field("name", &self.name)
            .field("reset_signal", &self.reset_signal)
            .field("checkpoints", &self.checkpoints)
            .field("commit_policy", &self.commit_policy)
            .finish()
    }
}

/// Checks a device table. Board configs call this in a const context so a
/// bad table fails the build.
pub const fn validate<R, D: ?Sized, E>(devices: &[DeviceConfig<R, D, E>]) {
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
    //
    // The fixture is a staged-boot device: one monotonic progress register
    // serves four checkpoints through one reader, and a poison value fails
    // every one — the pattern a real SoC table is expected to use.

    const POISON: u8 = 0xFF;

    struct SocBoard {
        level: u8,
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct RegFault;

    impl core::fmt::Display for RegFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("progress register unreadable")
        }
    }

    impl core::error::Error for RegFault {}

    impl SocBoard {
        fn progress_at_least(&mut self, level: u8) -> Result<BootStatus, RegFault> {
            if self.fail {
                return Err(RegFault);
            }
            Ok(match self.level {
                POISON => BootStatus::Failed,
                l if l >= level => BootStatus::Booted,
                _ => BootStatus::Booting,
            })
        }
    }

    // Named so the reject-path fixtures below can `..BL1` — an indexed
    // `CHECKPOINTS[0]` would not promote to 'static.
    const BL1: BootCheckpoint<SocBoard, RegFault> = BootCheckpoint {
        name: "bl1",
        timeout: Duration::from_millis(200),
        max_retries: 0,
        passed: |soc| soc.progress_at_least(1),
    };

    const CHECKPOINTS: &[BootCheckpoint<SocBoard, RegFault>] = &[
        BL1,
        BootCheckpoint {
            name: "bl2",
            timeout: Duration::from_secs(1),
            max_retries: 0,
            passed: |soc| soc.progress_at_least(2),
        },
        BootCheckpoint {
            name: "kernel",
            timeout: Duration::from_secs(10),
            max_retries: 2,
            passed: |soc| soc.progress_at_least(3),
        },
        BootCheckpoint {
            name: "service",
            timeout: Duration::from_secs(30),
            max_retries: 2,
            passed: |soc| soc.progress_at_least(4),
        },
    ];

    const DEVICE: DeviceConfig<u8, SocBoard, RegFault> = DeviceConfig {
        name: "soc",
        reset_signal: 0,
        checkpoints: CHECKPOINTS,
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
            checkpoints: &[BootCheckpoint { name: "", ..BL1 }],
            ..DEVICE
        }]);
    }

    #[test]
    #[should_panic(expected = "checkpoint timeout must not be zero")]
    fn rejects_a_zero_checkpoint_timeout() {
        validate(&[DeviceConfig {
            checkpoints: &[BootCheckpoint {
                timeout: Duration::ZERO,
                ..BL1
            }],
            ..DEVICE
        }]);
    }

    // One register, four checkpoints: each check sees exactly its own
    // threshold, so a device mid-boot passes the early ones and not the
    // late ones.
    #[test]
    fn checks_resolve_through_the_board_context() {
        let mut soc = SocBoard {
            level: 2,
            fail: false,
        };
        let read =
            |soc: &mut SocBoard, i: usize| (CHECKPOINTS[i].passed)(soc).expect("check failed");

        assert_eq!(read(&mut soc, 0), BootStatus::Booted); // bl1
        assert_eq!(read(&mut soc, 1), BootStatus::Booted); // bl2
        assert_eq!(read(&mut soc, 2), BootStatus::Booting); // kernel
        assert_eq!(read(&mut soc, 3), BootStatus::Booting); // service
    }

    // A poisoned register must read Failed from every checkpoint, whichever
    // one the walk happens to be awaiting.
    #[test]
    fn a_poisoned_register_fails_every_checkpoint() {
        let mut soc = SocBoard {
            level: POISON,
            fail: false,
        };

        for cp in CHECKPOINTS {
            assert_eq!(
                (cp.passed)(&mut soc).expect("check failed"),
                BootStatus::Failed
            );
        }
    }

    #[test]
    fn errors_surface_through_the_check() {
        let mut soc = SocBoard {
            level: 0,
            fail: true,
        };

        let err = (CHECKPOINTS[0].passed)(&mut soc).expect_err("expected the register fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "progress register unreadable");
    }
}
