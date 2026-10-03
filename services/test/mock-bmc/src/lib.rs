// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A BMC that boots, or does not, on demand.
//!
//! The RoT drives a reset line and watches a ready line. This models what
//! sits on the other end of those two wires: hold the ready line low while
//! reset is asserted, and once reset is released, raise it after a delay,
//! or never. That is the whole vocabulary a boot-sequence test needs from
//! a managed device.
//!
//! Nothing here knows what the lines are made of. On hardware they are
//! GPIO; in a QEMU test both ends live in one image and the pins are
//! whatever the board wiring supplies. The model is generic over
//! `InputPin` and `OutputPin` so neither case is special.
//!
//! Time arrives as an argument. [`MockBmc::poll`] is given the current
//! millisecond count and never waits, so a caller can drive it from a
//! kernel event loop, or from a test that moves the clock by hand.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::{InputPin, OutputPin};

/// What the mock BMC does once reset is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootBehaviour {
    /// Raises the ready line `after_millis` after release. A boot that is
    /// late rather than healthy is the same variant with a delay past the
    /// window the RoT is willing to wait, so there is no third variant for
    /// it: late is a property of the deadline, not of the device.
    Boots {
        /// Milliseconds from release to the ready line going active.
        after_millis: u64,
    },
    /// Never raises the ready line. The RoT finds out by its deadline
    /// expiring, which is the only way it ever finds out.
    Hangs,
}

/// Where the modelled device is in its boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Reset is asserted. The ready line is inactive.
    Held,
    /// Reset is released and the boot delay has not elapsed.
    Booting {
        /// When the delay started, in the caller's millisecond count.
        released_at_millis: u64,
    },
    /// The ready line is active.
    Ready,
    /// Reset is released and this device will never report ready.
    Hung,
}

/// Why a poll could not be completed. Carries the pin error so a caller
/// sees which line failed, not just that one did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<R, Y> {
    /// The reset line could not be read.
    Reset(R),
    /// The ready line could not be driven.
    Ready(Y),
}

impl<R: core::fmt::Display, Y: core::fmt::Display> core::fmt::Display for Error<R, Y> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Reset(e) => write!(f, "reading the reset line: {e}"),
            Self::Ready(e) => write!(f, "driving the ready line: {e}"),
        }
    }
}

impl<R, Y> core::error::Error for Error<R, Y>
where
    R: core::fmt::Debug + core::fmt::Display,
    Y: core::fmt::Debug + core::fmt::Display,
{
}

/// The device on the other end of a reset line and a ready line.
pub struct MockBmc<Reset, Ready> {
    reset: Reset,
    reset_polarity: ActivePolarity,
    ready: Ready,
    ready_polarity: ActivePolarity,
    behaviour: BootBehaviour,
    phase: Phase,
}

impl<Reset: InputPin, Ready: OutputPin> MockBmc<Reset, Ready> {
    /// Wires the model to its two lines.
    ///
    /// Starts in [`Phase::Held`] without touching either line: the first
    /// [`poll`](Self::poll) reads reset and drives ready to match, so a
    /// caller that constructs this before the RoT has settled its outputs
    /// does not latch a stale phase.
    pub fn new(
        reset: Reset,
        reset_polarity: ActivePolarity,
        ready: Ready,
        ready_polarity: ActivePolarity,
        behaviour: BootBehaviour,
    ) -> Self {
        Self {
            reset,
            reset_polarity,
            ready,
            ready_polarity,
            behaviour,
            phase: Phase::Held,
        }
    }

    /// Where the device is now. Only moves on [`poll`](Self::poll).
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Advances the model to `now_millis` and drives the ready line to
    /// match. Never waits.
    ///
    /// Asserting reset at any point returns the device to [`Phase::Held`]
    /// and drops the ready line, so a RoT that resets a device mid-boot
    /// sees the evidence go away, exactly as it would on hardware.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Reset`] if the reset line cannot be read and
    /// [`Error::Ready`] if the ready line cannot be driven.
    pub fn poll(&mut self, now_millis: u64) -> Result<Phase, Error<Reset::Error, Ready::Error>> {
        if self.reset_asserted()? {
            if self.phase != Phase::Held {
                self.drive_ready(false)?;
                self.phase = Phase::Held;
            }
            return Ok(self.phase);
        }

        self.phase = match (self.phase, self.behaviour) {
            // Released. Start the delay, or settle into never reporting.
            (Phase::Held, BootBehaviour::Boots { .. }) => Phase::Booting {
                released_at_millis: now_millis,
            },
            (Phase::Held, BootBehaviour::Hangs) => Phase::Hung,

            // The delay is measured from release, so a caller that polls
            // late still reports ready rather than restarting the count.
            (Phase::Booting { released_at_millis }, BootBehaviour::Boots { after_millis }) => {
                if now_millis.saturating_sub(released_at_millis) >= after_millis {
                    self.drive_ready(true)?;
                    Phase::Ready
                } else {
                    Phase::Booting { released_at_millis }
                }
            }

            // Behaviour does not change under the model's feet, so the
            // remaining pairs are either terminal or unreachable.
            (phase, _) => phase,
        };

        Ok(self.phase)
    }

    fn reset_asserted(&mut self) -> Result<bool, Error<Reset::Error, Ready::Error>> {
        let high = self.reset.is_high().map_err(Error::Reset)?;
        Ok(match self.reset_polarity {
            ActivePolarity::ActiveHigh => high,
            ActivePolarity::ActiveLow => !high,
        })
    }

    fn drive_ready(&mut self, active: bool) -> Result<(), Error<Reset::Error, Ready::Error>> {
        let high = match self.ready_polarity {
            ActivePolarity::ActiveHigh => active,
            ActivePolarity::ActiveLow => !active,
        };
        if high {
            self.ready.set_high().map_err(Error::Ready)
        } else {
            self.ready.set_low().map_err(Error::Ready)
        }
    }
}

#[cfg(test)]
mod tests;
