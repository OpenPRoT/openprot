// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The model itself: reads the reset line, drives the ready line.

use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::{InputPin, OutputPin};

use crate::boot::{BootBehaviour, Phase};
use crate::error::Error;

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
