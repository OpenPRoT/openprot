// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use rv_timer::RvTimer;

pub struct EarlGreyTimer {
    device: RvTimer,
}

impl EarlGreyTimer {
    pub const unsafe fn new() -> Self {
        Self {
            device: unsafe { RvTimer::new() },
        }
    }

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
