// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`Recovery`] restore capability contract.

/// Restore capability: rewrite one managed device's active image from its
/// configured recovery source.
///
/// What the recovery source is (a golden image in protected flash, a mux
/// flip to a known-good part, a fetch over a sideband) is board wiring and
/// never leaks through this seam. The orchestrator only asks for the
/// mechanism to run; it holds the device in reset before asking and
/// re-verifies the image on the walk that follows.
///
/// # Contract
///
/// - **`Ok` means the mechanism completed, not that the image is good.**
///   Judging the restored image belongs to the verifier on the re-walk; a
///   restore must not forge a verdict by checking it here.
/// - **Errors are actuation faults only** (source unreachable, write
///   failed). The orchestrator treats them fail-closed.
/// - **Repeatable.** Each recovery attempt calls `restore` again with the
///   next `attempt`; a partial earlier restore must not stop a later call
///   from producing a complete image.
/// - **The caller counts the attempts.** `attempt` comes from the core's own
///   retry counter, the same value its retry cap is measured against, so an
///   implementor that keeps a count of its own would drift: it never sees
///   which attempt succeeded. Running out of sources is an actuation error
///   like any other.
pub trait Recovery {
    /// The error type of this device's restore mechanism.
    ///
    /// Bounded by [`core::error::Error`] so the orchestrator gets `Display`
    /// and a `source()` cause chain, not just a `Debug` dump. Error
    /// categories are implementation-defined.
    type Error: core::error::Error;

    /// Rewrites the device's active image from the recovery source.
    ///
    /// `attempt` is this device's consecutive-recovery count, `0` on the
    /// first try of a recovery cycle. Implementors that hold more
    /// than one source pick per attempt (slot A on `0`, slot B on `1`,
    /// golden on `2`); implementors with a single source ignore it.
    fn restore(&mut self, attempt: u8) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Implements Recovery with no HAL dependency — the contract must be
    // satisfiable from any stack (mock, IPC proxy, simulator). A HAL-bound
    // `Error` type would stop this compiling.
    struct MockRecovery {
        attempts: [u8; 4],
        restores: usize,
        /// Attempt number the source faults on, so a test can make one
        /// restore fail and the next one succeed.
        fail_on: Option<u8>,
    }

    impl MockRecovery {
        fn healthy() -> Self {
            MockRecovery {
                attempts: [0; 4],
                restores: 0,
                fail_on: None,
            }
        }

        fn faulting_on(attempt: u8) -> Self {
            MockRecovery {
                attempts: [0; 4],
                restores: 0,
                fail_on: Some(attempt),
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock restore fault")
        }
    }

    impl core::error::Error for MockFault {}

    impl Recovery for MockRecovery {
        type Error = MockFault;

        fn restore(&mut self, attempt: u8) -> Result<(), MockFault> {
            if self.fail_on == Some(attempt) {
                return Err(MockFault);
            }
            self.attempts[self.restores] = attempt;
            self.restores += 1;
            Ok(())
        }
    }

    /// The orchestrator's shape: run the mechanism, judge nothing here.
    /// `attempt` rides in from `Effect::RecoverComponent`, never from a
    /// count the device keeps.
    fn recover<R: Recovery>(dev: &mut R, attempt: u8) -> Result<(), R::Error> {
        dev.restore(attempt)
    }

    #[test]
    fn contract_is_implementable_without_the_hal() {
        let mut dev = MockRecovery::healthy();

        recover(&mut dev, 0).expect("restore failed");
        recover(&mut dev, 1).expect("repeated restore failed");

        assert_eq!(dev.restores, 2);
    }

    #[test]
    fn each_attempt_reaches_the_implementor() {
        let mut dev = MockRecovery::healthy();

        // Out of order and with a gap, so a device that recorded its own
        // call count instead of the argument fails here.
        for attempt in [2, 0, 7] {
            recover(&mut dev, attempt).expect("restore failed");
        }

        assert_eq!(&dev.attempts[..3], &[2, 0, 7]);
    }

    #[test]
    fn a_failed_restore_does_not_block_the_next_attempt() {
        let mut dev = MockRecovery::faulting_on(0);

        recover(&mut dev, 0).expect_err("expected the first attempt to fault");
        recover(&mut dev, 1).expect("the next attempt must still restore");

        assert_eq!(dev.restores, 1);
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mut dev = MockRecovery::faulting_on(0);

        let err = recover(&mut dev, 0).expect_err("expected the restore fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "mock restore fault");
    }
}
