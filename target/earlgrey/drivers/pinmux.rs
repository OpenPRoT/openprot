// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use earlgrey_util_error::pinmux::{
    EG_PINMUX_INVALID_INPUT, EG_PINMUX_INVALID_OUTPUT, EG_PINMUX_INVALID_PAD,
};
use registers::pinmux;
pub use top_earlgrey::{PinmuxOutsel as Outsel, PinmuxPeripheralIn as PeriphIn};
use util_error::ErrorCode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
#[rustfmt::skip]
pub enum Pad {
    // Constant Values
    ConstantZero = -2,
    ConstantOne = -1,
    // MIO Pads (0-46)
    IOA0 = 0,
    IOA1 = 1,
    IOA2 = 2,
    IOA3 = 3,
    IOA4 = 4,
    IOA5 = 5,
    IOA6 = 6,
    IOA7 = 7,
    IOA8 = 8,
    IOB0 = 9,
    IOB1 = 10,
    IOB2 = 11,
    IOB3 = 12,
    IOB4 = 13,
    IOB5 = 14,
    IOB6 = 15,
    IOB7 = 16,
    IOB8 = 17,
    IOB9 = 18,
    IOB10 = 19,
    IOB11 = 20,
    IOB12 = 21,
    IOC0 = 22,
    IOC1 = 23,
    IOC2 = 24,
    IOC3 = 25,
    IOC4 = 26,
    IOC5 = 27,
    IOC6 = 28,
    IOC7 = 29,
    IOC8 = 30,
    IOC9 = 31,
    IOC10 = 32,
    IOC11 = 33,
    IOC12 = 34,
    IOR0 = 35,
    IOR1 = 36,
    IOR2 = 37,
    IOR3 = 38,
    IOR4 = 39,
    IOR5 = 40,
    IOR6 = 41,
    IOR7 = 42,
    IOR10 = 43,
    IOR11 = 44,
    IOR12 = 45,
    IOR13 = 46,
    // DIO Pads (47-62)
    DIO0 = 47,
    DIO1 = 48,
    DIO2 = 49,
    DIO3 = 50,
    DIO4 = 51,
    DIO5 = 52,
    DIO6 = 53,
    DIO7 = 54,
    DIO8 = 55,
    DIO9 = 56,
    DIO10 = 57,
    DIO11 = 58,
    DIO12 = 59,
    DIO13 = 60,
    DIO14 = 61,
    DIO15 = 62,
}

impl Pad {
    const NUM_MIO_PADS: i32 = top_earlgrey::NUM_MIO_PADS as i32;

    /// Is this a direct IO pad?
    pub fn is_dio(&self) -> bool {
        (*self as i32) >= Self::NUM_MIO_PADS
    }

    /// Get the direct IO index of this pad.
    pub fn dio_index(self) -> Option<usize> {
        let index = self as i32;
        if index >= Self::NUM_MIO_PADS {
            Some((index - Self::NUM_MIO_PADS) as usize)
        } else {
            None
        }
    }

    /// Get the muxed IO index of this pad.
    pub fn mio_index(self) -> Option<usize> {
        let index = self as i32;
        if (0..Self::NUM_MIO_PADS).contains(&index) {
            Some(index as usize)
        } else {
            None
        }
    }

    /// Get the input selector index of this pad.
    pub fn as_insel(self) -> Option<u32> {
        let idx = self as i32;
        if idx < Self::NUM_MIO_PADS {
            // The InSel selector is the index + 2.
            Some((idx + 2) as u32)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pull {
    None,
    Up,
    Down,
}

pub struct PadConfig {
    pub pull: Pull,
    pub open_drain: bool,
    pub invert: bool,
}

impl Default for PadConfig {
    fn default() -> Self {
        Self {
            pull: Pull::None,
            open_drain: false,
            invert: false,
        }
    }
}

pub struct EarlGreyPinmux {
    registers: pinmux::RegisterBlock<ureg::RealMmioMut<'static>>,
}

impl EarlGreyPinmux {
    /// Create a new instance of the EarlGrey Pinmux driver.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they have exclusive access to the Pinmux peripheral.
    pub unsafe fn new() -> Self {
        Self {
            registers: unsafe { pinmux::RegisterBlock::new(pinmux::PinmuxAon::PTR) },
        }
    }

    /// Connects a peripheral input to an MIO pad.
    pub fn connect_input(&mut self, input: PeriphIn, pad: Pad) -> Result<(), ErrorCode> {
        if let Some(sel) = pad.as_insel() {
            self.registers
                .mio_periph_insel()
                .at(input as usize)
                .write(|w| w.in_(sel));
            Ok(())
        } else {
            Err(EG_PINMUX_INVALID_INPUT)
        }
    }

    /// Connects an MIO pad to a peripheral output.
    pub fn connect_output(&mut self, pad: Pad, output: Outsel) -> Result<(), ErrorCode> {
        if let Some(idx) = pad.mio_index() {
            self.registers
                .mio_outsel()
                .at(idx)
                .write(|w| w.out(output as u32));
            Ok(())
        } else {
            Err(EG_PINMUX_INVALID_OUTPUT)
        }
    }

    pub fn configure_pad(&mut self, pad: Pad, config: &PadConfig) -> Result<(), ErrorCode> {
        if let Some(dio_idx) = pad.dio_index() {
            self.registers.dio_pad_attr().at(dio_idx).modify(|w| {
                w.pull_en(config.pull != Pull::None)
                    .pull_select(|w| {
                        if config.pull == Pull::Up {
                            w.pull_up()
                        } else {
                            w.pull_down()
                        }
                    })
                    .od_en(config.open_drain)
                    .invert(config.invert)
            });
            Ok(())
        } else if let Some(mio_idx) = pad.mio_index() {
            self.registers.mio_pad_attr().at(mio_idx).modify(|w| {
                w.pull_en(config.pull != Pull::None)
                    .pull_select(|w| {
                        if config.pull == Pull::Up {
                            w.pull_up()
                        } else {
                            w.pull_down()
                        }
                    })
                    .od_en(config.open_drain)
                    .invert(config.invert)
            });
            Ok(())
        } else if pad.as_insel().is_some() {
            // Constant pads (ConstantZero, ConstantOne) have valid input selectors
            // but no physical pad attributes to configure.
            Ok(())
        } else {
            Err(EG_PINMUX_INVALID_PAD)
        }
    }
}
