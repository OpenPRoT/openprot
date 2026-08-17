// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The AST1060 pin universe: one owned ZST token per pad, with its capability role impls.
//! A SoC fact (same across all AST1060 boards), so it lives here, not in a board.

use crate::gpio::{ABCD, EFGH, IJKL};
use ast1060_pac::{I2c, I2c1, I2c2, I2c3, I2cbuff, I2cbuff1, I2cbuff2, I2cbuff3};
use openprot_hal::field_mux::Block;
use openprot_hal::gpio::{Gpio, GpioData};
use openprot_hal::i2c::{I2cCtrlRegs, I2cData, I2cScl, I2cSda};

/// Block-identity marker for the AST10x0 SCU singleton register block (see `Mmio<B>` in `field_mux`).
pub struct ScuBlock;

// SAFETY: `Scu::ptr()` is the valid, aligned, `'static` SCU base, usable for `u32` MMIO at every offset its driver touches.
unsafe impl Block for ScuBlock {
    const BASE: *const u8 = ast1060_pac::Scu::ptr().cast();
}

// SAFETY: `Gpio::ptr()` is the valid, aligned, `'static` GPIO base, usable for `u32` MMIO at every offset its driver touches.
pub(crate) const GPIO_BASE: *const () = ast1060_pac::Gpio::ptr().cast();

/// Every I2C controller the AST1060 exposes (0..=13) — the single source of truth for controller
/// register bases. Pins reference a row; the userspace server indexes it by bus number.
pub const I2C_CTRL: &[I2cCtrlRegs] = &[
    I2cCtrlRegs {
        id: 0,
        i2c: I2c::ptr() as *const (),
        buff: I2cbuff::ptr() as *const (),
    },
    I2cCtrlRegs {
        id: 1,
        i2c: I2c1::ptr() as *const (),
        buff: I2cbuff1::ptr() as *const (),
    },
    I2cCtrlRegs {
        id: 2,
        i2c: I2c2::ptr() as *const (),
        buff: I2cbuff2::ptr() as *const (),
    },
    I2cCtrlRegs {
        id: 3,
        i2c: I2c3::ptr() as *const (),
        buff: I2cbuff3::ptr() as *const (),
    },
];

openprot_hal::pins! {
    scu414_28 { Gpio: &[clear(0x414, 28), clear(0x4b4, 28), clear(0x694, 28)] => GpioData { bit: 28, map: &EFGH } },
    scu414_29 { Gpio: &[clear(0x414, 29), clear(0x4b4, 29), clear(0x694, 29)] => GpioData { bit: 29, map: &EFGH } },
    scu414_30 { I2cScl: &[set(0x414, 30)] => I2cData { ctrl: I2C_CTRL[1] }, Gpio: &[clear(0x414, 30)] => GpioData { bit: 30, map: &EFGH } },
    scu414_31 { I2cSda: &[set(0x414, 31)] => I2cData { ctrl: I2C_CTRL[1] }, Gpio: &[clear(0x414, 31)] => GpioData { bit: 31, map: &EFGH } },
    scu418_0 { I2cScl: &[set(0x418, 0)] => I2cData { ctrl: I2C_CTRL[2] }, Gpio: &[clear(0x418, 0)] => GpioData { bit: 0, map: &IJKL } },
    scu418_1 { I2cSda: &[set(0x418, 1)] => I2cData { ctrl: I2C_CTRL[2] }, Gpio: &[clear(0x418, 1)] => GpioData { bit: 1, map: &IJKL } },

    scu410_0 { Gpio: &[clear(0x410, 0), clear(0x4b0, 0), clear(0x690, 0)] => GpioData { bit: 0, map: &ABCD } },
    scu410_1 { Gpio: &[clear(0x410, 1), clear(0x4b0, 1), clear(0x690, 1)] => GpioData { bit: 1, map: &ABCD } },
    scu410_2 { Gpio: &[clear(0x410, 2), clear(0x4b0, 2), clear(0x690, 2)] => GpioData { bit: 2, map: &ABCD } },
    scu410_3 { Gpio: &[clear(0x410, 3), clear(0x4b0, 3), clear(0x690, 3)] => GpioData { bit: 3, map: &ABCD } },
    scu410_4 { Gpio: &[clear(0x410, 4), clear(0x4b0, 4), clear(0x690, 4)] => GpioData { bit: 4, map: &ABCD } },
    scu410_5 { Gpio: &[clear(0x410, 5), clear(0x4b0, 5), clear(0x690, 5)] => GpioData { bit: 5, map: &ABCD } },
    scu410_6 { Gpio: &[clear(0x410, 6), clear(0x4b0, 6), clear(0x690, 6)] => GpioData { bit: 6, map: &ABCD } },
    scu410_7 { Gpio: &[clear(0x410, 7), clear(0x4b0, 7), clear(0x690, 7)] => GpioData { bit: 7, map: &ABCD } },
}
