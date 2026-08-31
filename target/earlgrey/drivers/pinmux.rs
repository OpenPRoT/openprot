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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum SlewRate {
    #[default]
    Slowest = 0,
    Slow = 1,
    Fast = 2,
    Fastest = 3,
}

impl SlewRate {
    pub const fn from_raw(val: u32) -> Self {
        match val & 3 {
            0 => Self::Slowest,
            1 => Self::Slow,
            2 => Self::Fast,
            _ => Self::Fastest,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum DriveStrength {
    #[default]
    Drive0 = 0,
    Drive1 = 1,
    Drive2 = 2,
    Drive3 = 3,
    Drive4 = 4,
    Drive5 = 5,
    Drive6 = 6,
    Drive7 = 7,
    Drive8 = 8,
    Drive9 = 9,
    Drive10 = 10,
    Drive11 = 11,
    Drive12 = 12,
    Drive13 = 13,
    Drive14 = 14,
    Drive15 = 15,
}

impl DriveStrength {
    pub const fn from_raw(val: u32) -> Self {
        match val & 0xf {
            0 => Self::Drive0,
            1 => Self::Drive1,
            2 => Self::Drive2,
            3 => Self::Drive3,
            4 => Self::Drive4,
            5 => Self::Drive5,
            6 => Self::Drive6,
            7 => Self::Drive7,
            8 => Self::Drive8,
            9 => Self::Drive9,
            10 => Self::Drive10,
            11 => Self::Drive11,
            12 => Self::Drive12,
            13 => Self::Drive13,
            14 => Self::Drive14,
            _ => Self::Drive15,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PadConfig {
    pub pull: Pull,
    pub open_drain: bool,
    pub invert: bool,
    pub slew_rate: SlewRate,
    pub drive_strength: DriveStrength,
}

impl Default for PadConfig {
    fn default() -> Self {
        Self {
            pull: Pull::None,
            open_drain: false,
            invert: false,
            slew_rate: SlewRate::Slowest,
            drive_strength: DriveStrength::Drive0,
        }
    }
}

impl PadConfig {
    pub const fn with_pull(mut self, pull: Pull) -> Self {
        self.pull = pull;
        self
    }

    pub const fn with_open_drain(mut self, open_drain: bool) -> Self {
        self.open_drain = open_drain;
        self
    }

    pub const fn with_invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    pub const fn with_slew_rate(mut self, slew_rate: SlewRate) -> Self {
        self.slew_rate = slew_rate;
        self
    }

    pub const fn with_drive_strength(mut self, drive_strength: DriveStrength) -> Self {
        self.drive_strength = drive_strength;
        self
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
                    .slew_rate(config.slew_rate as u32)
                    .drive_strength(config.drive_strength as u32)
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
                    .slew_rate(config.slew_rate as u32)
                    .drive_strength(config.drive_strength as u32)
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

    pub fn get_pad_config(&self, pad: Pad) -> Result<PadConfig, ErrorCode> {
        let (pull_en, pull_sel, od_en, invert, slew_rate, drive_strength) =
            if let Some(dio_idx) = pad.dio_index() {
                let reg = self.registers.dio_pad_attr().at(dio_idx).read();
                (
                    reg.pull_en(),
                    reg.pull_select(),
                    reg.od_en(),
                    reg.invert(),
                    reg.slew_rate(),
                    reg.drive_strength(),
                )
            } else if let Some(mio_idx) = pad.mio_index() {
                let reg = self.registers.mio_pad_attr().at(mio_idx).read();
                (
                    reg.pull_en(),
                    reg.pull_select(),
                    reg.od_en(),
                    reg.invert(),
                    reg.slew_rate(),
                    reg.drive_strength(),
                )
            } else if pad.as_insel().is_some() {
                // Constant pads (ConstantZero, ConstantOne) have valid input selectors
                // but no physical pad attributes to configure.
                return Ok(PadConfig::default());
            } else {
                return Err(EG_PINMUX_INVALID_PAD);
            };

        let pull = if !pull_en {
            Pull::None
        } else if pull_sel == pinmux::enums::PullSelect::PullUp {
            Pull::Up
        } else {
            Pull::Down
        };

        Ok(PadConfig {
            pull,
            open_drain: od_en,
            invert,
            slew_rate: SlewRate::from_raw(slew_rate),
            drive_strength: DriveStrength::from_raw(drive_strength),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slew_rate_from_raw() {
        assert_eq!(SlewRate::from_raw(0), SlewRate::Slowest);
        assert_eq!(SlewRate::from_raw(1), SlewRate::Slow);
        assert_eq!(SlewRate::from_raw(2), SlewRate::Fast);
        assert_eq!(SlewRate::from_raw(3), SlewRate::Fastest);
        assert_eq!(SlewRate::from_raw(4), SlewRate::Slowest);
    }

    #[test]
    fn test_drive_strength_from_raw() {
        assert_eq!(DriveStrength::from_raw(0), DriveStrength::Drive0);
        assert_eq!(DriveStrength::from_raw(5), DriveStrength::Drive5);
        assert_eq!(DriveStrength::from_raw(15), DriveStrength::Drive15);
        assert_eq!(DriveStrength::from_raw(16), DriveStrength::Drive0);
    }

    #[test]
    fn test_pad_config_builder() {
        let config = PadConfig::default()
            .with_pull(Pull::Up)
            .with_open_drain(true)
            .with_invert(true)
            .with_slew_rate(SlewRate::Fast)
            .with_drive_strength(DriveStrength::Drive7);

        assert_eq!(config.pull, Pull::Up);
        assert!(config.open_drain);
        assert!(config.invert);
        assert_eq!(config.slew_rate, SlewRate::Fast);
        assert_eq!(config.drive_strength, DriveStrength::Drive7);
    }
}
