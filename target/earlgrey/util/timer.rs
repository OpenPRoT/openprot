// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Timer driver wrapper for the Earlgrey target.

use rv_timer::RvTimer;

/// A driver for the OpenTitan RISC-V timer (RvTimer).
///
/// It provides access to the 64-bit system tick counter.
pub struct EarlGreyTimer {
    device: RvTimer,
}

impl EarlGreyTimer {
    /// Creates a new `EarlGreyTimer` instance.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other driver instance is concurrently writing
    /// to the timer hardware configuration. Reading the timer value is always safe.
    pub const unsafe fn new() -> Self {
        Self {
            device: unsafe { RvTimer::new() },
        }
    }

    /// Reads the current 64-bit timer ticks.
    ///
    /// This method performs a safe double-read of the high 32-bit register
    /// to handle potential carry-over/overflow while reading the low 32-bit register
    /// on 32-bit CPU architectures.
    pub fn read_ticks(&self) -> u64 {
        let regs = self.device.regs();
        loop {
            let hi1 = regs.timer_v_upper0().read();
            let low = regs.timer_v_lower0().read();
            let hi2 = regs.timer_v_upper0().read();
            if hi1 == hi2 {
                return ((hi1 as u64) << 32) | (low as u64);
            }
        }
    }
}
