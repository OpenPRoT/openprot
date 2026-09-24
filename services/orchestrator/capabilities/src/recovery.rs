// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`Recovery`] restore capability contract.

/// The outcome of a single restore attempt.
///
/// Carried on the `Ok` side of [`Recovery::restore`] so the orchestrator can
/// distinguish "the mechanism ran" from "there is nothing left to try"
/// without pattern-matching an opaque error type it cannot inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreOutcome {
    /// The recovery source was written to the device's active region.
    /// Whether the image is good is not judged here; the re-walk verifies.
    Restored,
    /// The device's configured recovery sources are exhausted (no untried
    /// image/slot remains). The orchestrator gates the component per its
    /// failure policy immediately, without waiting for the retry cap.
    SourceExhausted,
}

/// Restore capability: rewrite one managed device's active image from its
/// configured recovery source.
///
/// What the recovery source is (a golden image in protected flash, a mux
/// flip to a known-good part, a fetch over a sideband) is board wiring and
/// never leaks through this seam. The orchestrator only asks for the
/// mechanism to run; it holds the device in reset before asking and
/// re-verifies the image on the walk that follows.
///
/// `Ok(Restored)` says the mechanism completed, not that the image is good.
/// The verifier judges the restored image on the re-walk, so a restore must
/// not check it here. `Ok(SourceExhausted)` says no untried source remains,
/// and the orchestrator gates the component per its failure policy right
/// away instead of waiting for the retry cap. Errors are actuation faults
/// only, such as an unreachable source or a failed write, and the
/// orchestrator treats them fail-closed. Source exhaustion travels on the
/// `Ok` side because it is a known condition: the orchestrator applies
/// per-component policy to it instead of locking unconditionally.
///
/// Every recovery attempt calls `restore` again with the next `attempt`, so
/// a partial earlier restore must not stop a later call from producing a
/// complete image. The attempt count comes from the core's retry counter,
/// the same value the retry cap is measured against. A count kept by the
/// device would drift, because it never sees which attempt succeeded.
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
    /// first try of a recovery cycle. Implementors that hold more than one
    /// source pick per attempt (slot A on `0`, slot B on `1`, golden on
    /// `2`); implementors with a single source ignore it and return
    /// [`RestoreOutcome::SourceExhausted`] once their only source has been
    /// tried.
    fn restore(&mut self, attempt: u8) -> Result<RestoreOutcome, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Implements Recovery with no HAL dependency, because the contract must
    // be satisfiable from any stack (mock, IPC proxy, simulator). A HAL-bound
    // `Error` type would stop this compiling.
    struct MockRecovery {
        attempts: [u8; 4],
        restores: usize,
        /// Number of distinct sources this device holds.
        sources: u8,
        /// Attempt number the source faults on, so a test can make one
        /// restore fail and the next one succeed.
        fail_on: Option<u8>,
    }

    impl MockRecovery {
        fn healthy() -> Self {
            MockRecovery {
                attempts: [0; 4],
                restores: 0,
                sources: u8::MAX,
                fail_on: None,
            }
        }

        fn with_sources(sources: u8) -> Self {
            MockRecovery {
                attempts: [0; 4],
                restores: 0,
                sources,
                fail_on: None,
            }
        }

        fn faulting_on(attempt: u8) -> Self {
            MockRecovery {
                attempts: [0; 4],
                restores: 0,
                sources: u8::MAX,
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

        fn restore(&mut self, attempt: u8) -> Result<RestoreOutcome, MockFault> {
            if self.fail_on == Some(attempt) {
                return Err(MockFault);
            }
            if attempt >= self.sources {
                return Ok(RestoreOutcome::SourceExhausted);
            }
            self.attempts[self.restores] = attempt;
            self.restores += 1;
            Ok(RestoreOutcome::Restored)
        }
    }

    /// Calls the mechanism the way the orchestrator does: run it, judge
    /// nothing here. `attempt` comes from `Effect::RecoverComponent`, never
    /// from a count the device keeps.
    fn recover<R: Recovery>(dev: &mut R, attempt: u8) -> Result<RestoreOutcome, R::Error> {
        dev.restore(attempt)
    }

    #[test]
    fn contract_is_implementable_without_the_hal() {
        let mut dev = MockRecovery::healthy();

        assert_eq!(recover(&mut dev, 0).unwrap(), RestoreOutcome::Restored);
        assert_eq!(recover(&mut dev, 1).unwrap(), RestoreOutcome::Restored);

        assert_eq!(dev.restores, 2);
    }

    #[test]
    fn each_attempt_reaches_the_implementor() {
        let mut dev = MockRecovery::healthy();

        // Out of order and with a gap, so a device that recorded its own
        // call count instead of the argument fails here.
        for attempt in [2, 0, 7] {
            assert_eq!(
                recover(&mut dev, attempt).unwrap(),
                RestoreOutcome::Restored
            );
        }

        assert_eq!(&dev.attempts[..3], &[2, 0, 7]);
    }

    #[test]
    fn a_failed_restore_does_not_block_the_next_attempt() {
        let mut dev = MockRecovery::faulting_on(0);

        recover(&mut dev, 0).expect_err("expected the first attempt to fault");
        assert_eq!(recover(&mut dev, 1).unwrap(), RestoreOutcome::Restored);

        assert_eq!(dev.restores, 1);
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mut dev = MockRecovery::faulting_on(0);

        let err = recover(&mut dev, 0).expect_err("expected the restore fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "mock restore fault");
    }

    #[test]
    fn source_exhaustion_is_a_verdict_not_an_error() {
        let mut dev = MockRecovery::with_sources(2);

        assert_eq!(recover(&mut dev, 0).unwrap(), RestoreOutcome::Restored);
        assert_eq!(recover(&mut dev, 1).unwrap(), RestoreOutcome::Restored);
        assert_eq!(
            recover(&mut dev, 2).unwrap(),
            RestoreOutcome::SourceExhausted
        );

        assert_eq!(dev.restores, 2, "exhaustion does not count as a restore");
    }
}
