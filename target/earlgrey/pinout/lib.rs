// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use earlgrey_gpio::EarlGreyGpio;
use util_error::ErrorCode;

pub mod config;
pub mod dualsbs;
pub mod swstraps;
#[cfg(target_os = "none")]
mod target;

pub trait Pinout {
    const PINOUT: &[config::PinoutConfig];

    #[cfg(target_os = "none")]
    fn configure(gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        config::PinoutConfig::configure(Self::PINOUT, gpio)
    }

    #[cfg(not(target_os = "none"))]
    fn configure(_gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        unimplemented!()
    }
}
