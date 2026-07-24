// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Observation capability: read a managed device's boot liveness.

/// Liveness of a managed device's boot: Boot Confirmation only.
///
/// Reports only that a device came up, never what booted; confirming the
/// running image is the one the RoT staged is attestation, a separate step.
/// `Failed` is optional device-reported evidence and never the only failure
/// path, since a hung device reports nothing — a stuck boot is caught by the
/// orchestrator's timeout, not by this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStatus {
    /// Held in reset by the orchestrator; not yet released.
    InReset,
    /// Released, but boot completion not yet observed.
    Booting,
    /// Boot completion observed.
    Booted,
    /// Device reported a boot failure.
    Failed,
}

/// Observation capability: read a managed device's boot liveness.
///
/// Pull-shaped: where the underlying signal is an edge or pulse, the interrupt
/// latches a flag beneath this seam and `boot_status` only reads it. No
/// callback registration, which would require allocation and invert control
/// into device implementations.
///
/// The reported status must describe the **current** boot cycle. An
/// implementation backed by a latched signal must guarantee the latch is
/// cleared whenever the device re-enters reset, so evidence left over from a
/// previous boot never reads as [`BootStatus::Booted`]. This trait
/// deliberately has no re-arm operation: clearing is the reset path's job
/// (hardware tying the latch to the device's reset line, or the same platform
/// code that drives `BootControl`), not the observer's — a monitor that could
/// clear its own evidence would let a read race a reset.
pub trait BootMonitor {
    /// The error type reported by this device's boot monitor.
    ///
    /// Requires [`core::error::Error`] (in `core` since Rust 1.81) so the
    /// orchestrator gets `Display` and a `source()` cause chain, not just a
    /// `Debug` dump. Error categories stay implementation-defined — this
    /// crate names no error vocabulary of its own; a consumer that knows the
    /// concrete adapter can recover its details by downcasting the
    /// `&dyn core::error::Error`.
    type Error: core::error::Error;

    /// Returns the current liveness of the device.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying liveness signal cannot be read.
    fn boot_status(&self) -> Result<BootStatus, Self::Error>;
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use super::*;
    use core::cell::Cell;

    // ── Trait contract ──────────────────────────────────────────────────
    // MockMonitor implements the trait without any HAL dependency. If a
    // HAL-specific bound sneaks back onto `Error`, this module stops
    // compiling.

    struct MockMonitor {
        ready_after: usize,
        polls: Cell<usize>,
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock monitor fault")
        }
    }

    impl core::error::Error for MockFault {}

    impl BootMonitor for MockMonitor {
        type Error = MockFault;

        fn boot_status(&self) -> Result<BootStatus, MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            let polls = self.polls.get();
            self.polls.set(polls + 1);
            Ok(if polls >= self.ready_after {
                BootStatus::Booted
            } else {
                BootStatus::Booting
            })
        }
    }

    // A device that is still coming up reads Booting, then Booted once it
    // is up.
    #[test]
    fn status_progresses_from_booting_to_booted() {
        let mon = MockMonitor {
            ready_after: 1,
            polls: Cell::new(0),
            fail: false,
        };

        assert_eq!(
            mon.boot_status().expect("boot_status failed"),
            BootStatus::Booting
        );
        assert_eq!(
            mon.boot_status().expect("boot_status failed"),
            BootStatus::Booted
        );
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mon = MockMonitor {
            ready_after: 0,
            polls: Cell::new(0),
            fail: true,
        };

        let err = comes_up_within(&mon, 1).expect_err("expected the monitor fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "mock monitor fault");
    }

    // ── The orchestrator's future shape ─────────────────────────────────
    // Usage examples for the future orchestrator, not API guarantees; move
    // these to the orchestrator crate once it exists.

    /// Poll a monitor up to `poll_budget` times. `Booting` is not a failure;
    /// `Ok(false)` means the budget ran out before the device came up.
    fn comes_up_within<M: BootMonitor>(mon: &M, poll_budget: usize) -> Result<bool, M::Error> {
        for _ in 0..poll_budget {
            if mon.boot_status()? == BootStatus::Booted {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // A device that comes up within the poll budget reads Booted.
    #[test]
    fn a_device_that_comes_up_within_budget_is_booted() {
        let mon = MockMonitor {
            ready_after: 2,
            polls: Cell::new(0),
            fail: false,
        };

        assert_eq!(comes_up_within(&mon, 5).expect("boot_status failed"), true);
    }

    #[test]
    fn a_device_that_never_comes_up_is_a_timeout_not_an_error() {
        let mon = MockMonitor {
            ready_after: usize::MAX,
            polls: Cell::new(0),
            fail: false,
        };

        assert_eq!(comes_up_within(&mon, 3).expect("boot_status failed"), false);
    }
}
