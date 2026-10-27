// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`SvnFloor`] anti-rollback capability contract.

/// A security version number, as carried in image manifests and compared
/// against a device's anti-rollback floor.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Svn(pub u32);

/// Anti-rollback capability: one managed device's durable SVN floor.
///
/// The floor is the lowest SVN the device may still boot. Storage — OTP
/// fuse counters, a monotonic counter in protected flash, a mock in tests —
/// is the implementor's concern; its encoding (e.g. unary fuse bits) never
/// leaks through this seam. Devices that keep their own floor (a PLDM
/// firmware device commits internally) have no `SvnFloor` on the eRoT side.
///
/// The orchestrator advances the floor only after an activated image proved
/// itself at runtime (`BootConfirmed`), never on activation alone, so a bad
/// image can still be rolled back.
///
/// # Contract
///
/// - **Monotonic.** `advance(to)` with `to` at or below the current floor
///   succeeds as a no-op (a replayed commit is harmless); no call ever
///   lowers the floor. A lower target is not distinguishable from a replay
///   at this seam; callers that need to detect one compare against
///   [`floor`](Self::floor) first. The no-op lives in the implementor: an
///   encoding that is naturally one-way (unary fuse counters) gets it for
///   free, any other storage must check `to` against its floor itself.
/// - **Durable on return.** When `advance` returns `Ok`, the new floor
///   survives power loss. A torn write may lose the advance (the caller
///   re-commits) but must never leave the floor below its previous value.
pub trait SvnFloor {
    /// The error type of this device's floor storage.
    ///
    /// Bounded by [`core::error::Error`] so the orchestrator gets `Display`
    /// and a `source()` cause chain, not just a `Debug` dump. Error
    /// categories are implementation-defined.
    type Error: core::error::Error;

    /// The current floor: the lowest SVN this device may still boot.
    fn floor(&self) -> Result<Svn, Self::Error>;

    /// Raises the floor to `to`. At or below the current floor: `Ok`, no-op.
    fn advance(&mut self, to: Svn) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Implements SvnFloor with no HAL dependency — the contract must be
    // satisfiable from any stack (mock, IPC proxy, simulator). A HAL-bound
    // `Error` type would stop this compiling.
    struct MockFloor {
        floor: Svn,
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock floor fault")
        }
    }

    impl core::error::Error for MockFault {}

    impl SvnFloor for MockFloor {
        type Error = MockFault;

        fn floor(&self) -> Result<Svn, MockFault> {
            if self.fail {
                Err(MockFault)
            } else {
                Ok(self.floor)
            }
        }

        fn advance(&mut self, to: Svn) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            self.floor = self.floor.max(to);
            Ok(())
        }
    }

    /// The orchestrator's commit shape: advance, then read the floor back.
    fn commit<F: SvnFloor>(floor: &mut F, confirmed: Svn) -> Result<Svn, F::Error> {
        floor.advance(confirmed)?;
        floor.floor()
    }

    #[test]
    fn contract_is_implementable_without_the_hal() {
        let mut floor = MockFloor {
            floor: Svn(3),
            fail: false,
        };

        assert_eq!(commit(&mut floor, Svn(5)), Ok(Svn(5)));
    }

    #[test]
    fn replayed_commit_is_a_noop() {
        let mut floor = MockFloor {
            floor: Svn(5),
            fail: false,
        };

        assert_eq!(commit(&mut floor, Svn(5)), Ok(Svn(5)));
        assert_eq!(commit(&mut floor, Svn(4)), Ok(Svn(5)), "never lowers");
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mut floor = MockFloor {
            floor: Svn(0),
            fail: true,
        };

        let err = commit(&mut floor, Svn(1)).expect_err("expected the floor fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "mock floor fault");
    }
}
