// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`LockdownLatch`] terminal safe-state capability contract.

/// Latch capability: put the platform into its terminal safe state.
///
/// What the safe state is (gating every managed device, tripping a fuse,
/// parking straps) is board wiring and never leaks through this seam.
///
/// The latch is one-way and idempotent: nothing short of a platform reset
/// unlatches it, and latching an already-latched platform succeeds. `Ok`
/// means the safe state is in force, not merely requested. A failed latch
/// is a hard fault: `Err` means the safe state is not in force and the
/// caller must not continue as if it were. How the platform escalates from
/// there is board policy, outside this contract.
pub trait LockdownLatch {
    /// The error type of this platform's latch mechanism.
    ///
    /// Bounded by [`core::error::Error`] so the caller gets `Display` and a
    /// `source()` cause chain. Error categories are implementation-defined.
    type Error: core::error::Error;

    /// Latches the platform into the safe state.
    fn latch(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Implements LockdownLatch with no HAL dependency: the contract must be
    // satisfiable from any stack (mock, IPC proxy, simulator). A HAL-bound
    // `Error` type would stop this compiling.
    struct MockLatch {
        latched: bool,
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock latch fault")
        }
    }

    impl core::error::Error for MockFault {}

    impl LockdownLatch for MockLatch {
        type Error = MockFault;

        fn latch(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            self.latched = true;
            Ok(())
        }
    }

    #[test]
    fn contract_is_implementable_without_the_hal() {
        let mut dev = MockLatch {
            latched: false,
            fail: false,
        };

        dev.latch().expect("latch failed");
        dev.latch().expect("repeated latch failed"); // idempotent

        assert!(dev.latched);
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mut dev = MockLatch {
            latched: false,
            fail: true,
        };

        let err = dev.latch().expect_err("expected the latch fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "mock latch fault");
    }
}
