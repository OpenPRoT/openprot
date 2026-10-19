// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`SelfReset`] actuation capability contract.

/// Actuation capability: reset the eRoT itself.
///
/// Activating the eRoT's own update ends here: record the session, mark
/// the trial pending, reset. The boot selector then runs the trial image
/// and [`settle_self_update`](crate::SelfUpdate) judges it on the way up.
///
/// Not [`BootControl`](crate::BootControl), whose `hold_in_reset` and
/// `release` are two calls: the caller has to survive the first to make
/// the second. This caller does not survive `reset`. The receiver is
/// `&mut self` like the other capabilities, which also lets an
/// implementation record the attempt, as the driver tests' mock does.
///
/// # Errors
///
/// `reset` returns only when the reset did not happen.
/// [`Infallible`](core::convert::Infallible) enforces it: with no success
/// value, an implementation that returns at all returns an error. `Err`
/// means the old image still runs with the trial marked, so the next
/// reset from any source boots the trial.
pub trait SelfReset {
    /// Why the reset did not happen. Each platform defines its own
    /// failures.
    ///
    /// [`core::error::Error`] so it can be displayed and chained.
    type Error: core::error::Error;

    /// Resets the eRoT. Returns only on failure.
    fn reset(&mut self) -> Result<core::convert::Infallible, Self::Error>;
}

/// The reset for a board that cannot reset itself: it fails instead of
/// pretending. A board that does update itself implements `SelfReset`
/// properly. Boards that never update themselves keep `()` rather than
/// naming a type they never call, like the `()` impls of `SelfUpdate`
/// and `Recovery`.
impl SelfReset for () {
    type Error = NoReset;

    fn reset(&mut self) -> Result<core::convert::Infallible, NoReset> {
        Err(NoReset)
    }
}

/// This board cannot reset the eRoT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoReset;

impl core::fmt::Display for NoReset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("this board cannot reset the eRoT")
    }
}

impl core::error::Error for NoReset {}

#[cfg(test)]
mod tests {
    use super::*;

    // `()` is the reset type a board that cannot reset itself names.
    // The QEMU scenarios run this impl.
    #[test]
    fn a_board_with_no_way_to_reset_says_so() {
        assert_eq!(().reset().unwrap_err(), NoReset);
    }
}
