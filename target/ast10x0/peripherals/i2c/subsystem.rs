// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared I2C subsystem bring-up: the one home for the clock/reset/global sequence and the I2C SCU
//! bit layout. It runs once for the whole subsystem — the block reset clobbers every controller.

use crate::scu::{ClockRegisterHalf, ScuRegisterHalf, ScuRegisters};

/// Shared I2C clock enable — Group 0 (lower clock-stop half), bit 2.
const I2C_CLOCK_BIT: u32 = 1 << 2;
/// Shared I2C block reset — upper reset-control half, bit 2.
const I2C_RESET_BIT: u32 = 1 << 2;

/// Bring up the shared I2C subsystem (ungate clock, cycle block reset, configure globals), consuming
/// the caller's already-unlocked SCU handle — the authority — so no second unlock is needed.
pub fn bringup(scu: ScuRegisters, mut delay_us: impl FnMut(u32)) {
    scu.ungate_clock_mask(ClockRegisterHalf::Lower, I2C_CLOCK_BIT);
    scu.assert_reset_mask(ScuRegisterHalf::Upper, I2C_RESET_BIT);
    delay_us(1000);
    scu.deassert_reset_mask(ScuRegisterHalf::Upper, I2C_RESET_BIT);
    delay_us(1000);
    // SAFETY: clock is ungated and reset deasserted above — the precondition for global init.
    unsafe { super::global::init_i2c_global() };
}
