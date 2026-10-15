// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Home of the [`SelfReset`] capability contract.

/// Actuation capability: reset the eRoT itself.
///
/// Separate from [`BootControl`](crate::BootControl), which holds a managed
/// device in reset and releases it afterwards. That pair assumes a caller
/// that survives to make the second call. This one does not come back: the
/// code that asked is gone with the reset, so a successful call has no
/// return value to describe. It still takes `&mut self` rather than
/// `self`, because a board stores it alongside the session and the floor;
/// the borrow says where it lives, not that the call returns.
///
/// The eRoT's own update needs it. An activation on the self path records
/// the session, marks the trial pending and then resets, so the boot
/// selector runs the trial image and
/// [`settle_self_update`](crate::SelfUpdate) judges it on the way back up.
///
/// # Errors
///
/// `reset` returns only when the reset did not happen. The success type is
/// [`Infallible`](core::convert::Infallible), so a caller cannot write a
/// path for a return that cannot occur, and an implementor that would have
/// to invent one is told at compile time that it has not reset anything.
/// An implementation that returns `Err` has left the eRoT running the image
/// it was already running, with the session already marked pending, which
/// the next reset from any source still boots.
pub trait SelfReset {
    /// The error type reported by this platform's reset.
    ///
    /// Bounded by [`core::error::Error`] so the orchestrator gets `Display`
    /// and a `source()` cause chain, not just a `Debug` dump. Error
    /// categories are implementation-defined.
    type Error: core::error::Error;

    /// Resets the eRoT. Returns only on failure.
    fn reset(&mut self) -> Result<core::convert::Infallible, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // A SelfReset against no HAL at all: the contract has to be satisfiable
    // from any stack, the same as every other actuation capability.
    struct MockReset {
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ResetFault;

    impl core::fmt::Display for ResetFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("reset fault")
        }
    }

    impl core::error::Error for ResetFault {}

    impl SelfReset for MockReset {
        type Error = ResetFault;

        fn reset(&mut self) -> Result<core::convert::Infallible, ResetFault> {
            // A mock cannot reset the process it runs in, so the only
            // branch it can take is the failing one.
            assert!(self.fail, "a mock cannot actually reset the eRoT");
            Err(ResetFault)
        }
    }

    #[test]
    fn a_failed_reset_reports_why() {
        assert_eq!(MockReset { fail: true }.reset().unwrap_err(), ResetFault);
    }
}
