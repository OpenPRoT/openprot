// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::GpioPin;
use earlgrey_pinmux::Pad;

use crate::config::*;
use crate::Pinout;

type PC = PinoutConfig;

pub struct SwStraps;
impl SwStraps {
    pub const SW_STRAP0: GpioPin = GpioPin::Pin22;
    pub const SW_STRAP1: GpioPin = GpioPin::Pin23;
    pub const SW_STRAP2: GpioPin = GpioPin::Pin24;
}

impl Pinout for SwStraps {
    #[rustfmt::skip]
    const PINOUT: &[PinoutConfig] = &[
        // Note: to correctly read the strap value, the pinout must be ordered
        // from MSB to LSB.
        PC::gpio_in("SW_STRAP2", "IOC2", Self::SW_STRAP2, Pad::IOC2, IN_PULL_NONE),
        PC::gpio_in("SW_STRAP1", "IOC1", Self::SW_STRAP1, Pad::IOC1, IN_PULL_NONE),
        PC::gpio_in("SW_STRAP0", "IOC0", Self::SW_STRAP0, Pad::IOC0, IN_PULL_NONE),
    ];
}
