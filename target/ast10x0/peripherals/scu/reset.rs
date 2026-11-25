// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU reset helpers.

use super::registers::ScuRegisters;
use super::types::ScuRegisterHalf;

impl ScuRegisters {
    /// Assert reset bits in the selected reset-control register half.
    ///
    /// Unlocks the SCU for the duration of the write; see
    /// [`ScuRegisters::unlock_and_lock_after`].
    pub fn assert_reset_mask(&self, half: ScuRegisterHalf, mask: u32) {
        self.unlock_and_lock_after(|scu| match half {
            ScuRegisterHalf::Lower => {
                scu.regs().scu040().write(|w| unsafe { w.bits(mask) });
            }
            ScuRegisterHalf::Upper => {
                scu.regs().scu050().write(|w| unsafe { w.bits(mask) });
            }
        });
    }

    /// Deassert reset bits in the selected reset-control register half.
    ///
    /// Unlocks the SCU for the duration of the write; see
    /// [`ScuRegisters::unlock_and_lock_after`].
    pub fn deassert_reset_mask(&self, half: ScuRegisterHalf, mask: u32) {
        self.unlock_and_lock_after(|scu| match half {
            ScuRegisterHalf::Lower => {
                scu.regs().scu044().write(|w| unsafe { w.bits(mask) });
            }
            ScuRegisterHalf::Upper => {
                scu.regs().scu054().write(|w| unsafe { w.bits(mask) });
            }
        });
    }

    /// Read the selected reset-control register half.
    #[must_use]
    pub fn reset_mask(&self, half: ScuRegisterHalf) -> u32 {
        match half {
            ScuRegisterHalf::Lower => self.regs().scu040().read().bits(),
            ScuRegisterHalf::Upper => self.regs().scu050().read().bits(),
        }
    }
}
