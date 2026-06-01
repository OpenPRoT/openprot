// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU clock gate helpers.

use super::registers::ScuRegisters;
use super::types::ClockRegisterHalf;

impl ScuRegisters {
    /// Gate clocks by setting bits in the selected clock-stop register half.
    ///
    /// Caller must call `unlock_write_protection()` before this.
    pub fn gate_clock_mask(&self, half: ClockRegisterHalf, mask: u32) {
        match half {
            ClockRegisterHalf::Lower => {
                self.regs().scu080().write(|w| unsafe { w.bits(mask) });
            }
            ClockRegisterHalf::Upper => {
                self.regs().scu090().write(|w| unsafe { w.bits(mask) });
            }
        }
    }

    /// Ungate clocks by clearing bits in the selected clock-stop register half.
    ///
    /// Caller must call `unlock_write_protection()` before this.
    pub fn ungate_clock_mask(&self, half: ClockRegisterHalf, mask: u32) {
        match half {
            ClockRegisterHalf::Lower => {
                self.regs().scu084().write(|w| unsafe { w.bits(mask) });
            }
            ClockRegisterHalf::Upper => {
                self.regs().scu094().write(|w| unsafe { w.bits(mask) });
            }
        }
    }

    /// Read the selected clock-stop register half.
    #[must_use]
    pub fn gated_clock_mask(&self, half: ClockRegisterHalf) -> u32 {
        match half {
            ClockRegisterHalf::Lower => self.regs().scu080().read().bits(),
            ClockRegisterHalf::Upper => self.regs().scu090().read().bits(),
        }
    }
}
