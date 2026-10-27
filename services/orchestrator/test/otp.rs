// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The mock board's OTP fuse bank: [`MockOtpFloor`], an OTP-fused
//! [`SvnFloor`] with real one-time semantics — the floor is a unary fuse
//! counter, so it moves one way by construction, capacity is finite
//! (`FUSES`), and a fault can be injected for the next operation to
//! exercise failure paths.
//!
//! Lives with the board, not in a testonly crate, so host integration
//! tests and a QEMU mock-board image link the same fuse bank; `seed` and
//! `fail_next` are the harness knobs in both.
//!
//! [`SvnFloor`]: orchestrator_capabilities::SvnFloor

#![cfg_attr(not(test), no_std)]

use core::cell::Cell;

use orchestrator_capabilities::{Svn, SvnFloor};

/// Error returned by [`MockOtpFloor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFloorError {
    /// The requested floor exceeds the region's fuse capacity.
    Exhausted,
    /// Injected via [`MockOtpFloor::fail_next`].
    Injected,
}

impl core::fmt::Display for MockFloorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            MockFloorError::Exhausted => "svn floor fuses exhausted",
            MockFloorError::Injected => "injected floor fault",
        })
    }
}

impl core::error::Error for MockFloorError {}

/// An OTP-fused SVN floor of `FUSES` one-way bits, all unburnt at reset.
///
/// The floor is the number of burnt fuses (unary encoding), so `FUSES` is
/// the highest representable floor and lowering is impossible by
/// construction — the same shape a real fuse bank gives.
pub struct MockOtpFloor<const FUSES: usize> {
    burnt: usize,
    // `Cell`: `SvnFloor::floor` reads through `&self` but the injected
    // fault is one-shot, so consuming it must work without `&mut`.
    fail_next: Cell<bool>,
}

impl<const FUSES: usize> MockOtpFloor<FUSES> {
    /// A pristine region: no fuses burnt, floor zero.
    pub fn new() -> Self {
        Self {
            burnt: 0,
            fail_next: Cell::new(false),
        }
    }

    /// Burn fuses directly to `floor`, bypassing the capability seam.
    /// Test setup only; panics if `floor` exceeds `FUSES` or would lower
    /// the current floor.
    pub fn seed(&mut self, floor: Svn) {
        let target = floor.0 as usize;
        assert!(target <= FUSES, "seed beyond fuse capacity");
        assert!(target >= self.burnt, "seed would lower the floor");
        self.burnt = target;
    }

    /// Make the next operation (read or advance) fail. One-shot: the
    /// operation after that behaves normally again. Nothing is burnt by
    /// the failed operation.
    pub fn fail_next(&mut self) {
        self.fail_next.set(true);
    }

    fn check(&self) -> Result<(), MockFloorError> {
        if self.fail_next.take() {
            return Err(MockFloorError::Injected);
        }
        Ok(())
    }
}

impl<const FUSES: usize> Default for MockOtpFloor<FUSES> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const FUSES: usize> SvnFloor for MockOtpFloor<FUSES> {
    type Error = MockFloorError;

    fn floor(&self) -> Result<Svn, MockFloorError> {
        self.check()?;
        Ok(Svn(self.burnt as u32))
    }

    fn advance(&mut self, to: Svn) -> Result<(), MockFloorError> {
        self.check()?;
        let target = to.0 as usize;
        if target <= self.burnt {
            // At or below the current floor: a replayed commit, no-op.
            return Ok(());
        }
        if target > FUSES {
            return Err(MockFloorError::Exhausted);
        }
        self.burnt = target;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pristine_floor_is_zero_and_advances() {
        let mut otp = MockOtpFloor::<8>::new();

        assert_eq!(otp.floor(), Ok(Svn(0)));
        assert_eq!(otp.advance(Svn(3)), Ok(()));
        assert_eq!(otp.floor(), Ok(Svn(3)));
    }

    #[test]
    fn replayed_and_lower_commits_are_noops() {
        let mut otp = MockOtpFloor::<8>::new();
        otp.seed(Svn(5));

        assert_eq!(otp.advance(Svn(5)), Ok(()));
        assert_eq!(otp.advance(Svn(2)), Ok(()));
        assert_eq!(otp.floor(), Ok(Svn(5)), "never lowers");
    }

    #[test]
    fn exhausted_fuses_are_an_error_and_burn_nothing() {
        let mut otp = MockOtpFloor::<4>::new();
        otp.seed(Svn(2));

        assert_eq!(otp.advance(Svn(5)), Err(MockFloorError::Exhausted));
        assert_eq!(otp.floor(), Ok(Svn(2)), "failed advance burns nothing");
        assert_eq!(otp.advance(Svn(4)), Ok(()), "capacity itself still fits");
    }

    #[test]
    fn injected_fault_is_one_shot() {
        let mut otp = MockOtpFloor::<8>::new();
        otp.fail_next();

        assert_eq!(otp.advance(Svn(1)), Err(MockFloorError::Injected));
        assert_eq!(otp.floor(), Ok(Svn(0)), "failed advance burnt nothing");
        assert_eq!(otp.advance(Svn(1)), Ok(()));
    }

    /// The commit shape the platform driver will use, generic over the
    /// capability seam — the mock is indistinguishable from real storage.
    #[test]
    fn usable_through_the_generic_seam() {
        fn commit<F: SvnFloor>(floor: &mut F, confirmed: Svn) -> Result<Svn, F::Error> {
            floor.advance(confirmed)?;
            floor.floor()
        }

        let mut otp = MockOtpFloor::<8>::new();
        assert_eq!(commit(&mut otp, Svn(6)), Ok(Svn(6)));
    }
}
