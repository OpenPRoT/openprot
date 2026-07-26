// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::GpioPin;
use earlgrey_pinmux::{Pad, PadConfig, Pull};
use top_earlgrey::{PinmuxOutsel as Outsel, PinmuxPeripheralIn as PeriphIn};

pub enum Config {
    Input {
        periph: PeriphIn,
        pad: Pad,
        pad_config: PadConfig,
    },
    Output {
        periph: Outsel,
        pad: Pad,
        pad_config: PadConfig,
    },
    Io {
        periph: PeriphIn,
        pad: Pad,
        pad_config: PadConfig,
    },
}

impl Config {
    pub fn is_input(&self) -> bool {
        matches!(self, Config::Input { .. } | Config::Io { .. })
    }

    pub fn is_output(&self) -> bool {
        matches!(self, Config::Output { .. } | Config::Io { .. })
    }
}

pub struct PinoutConfig {
    pub name: &'static str,
    pub padname: &'static str,
    pub pin: Option<GpioPin>,
    pub config: Config,
}

impl PinoutConfig {
    pub const fn gpio_in(
        name: &'static str,
        padname: &'static str,
        pin: GpioPin,
        pad: Pad,
        pad_config: PadConfig,
    ) -> PinoutConfig {
        PinoutConfig {
            name,
            padname,
            pin: Some(pin),
            config: Config::Input {
                periph: pin.as_periph(),
                pad,
                pad_config,
            },
        }
    }

    pub const fn gpio_out(
        name: &'static str,
        padname: &'static str,
        pin: GpioPin,
        pad: Pad,
        pad_config: PadConfig,
    ) -> PinoutConfig {
        PinoutConfig {
            name,
            padname,
            pin: Some(pin),
            config: Config::Output {
                periph: pin.as_outsel(),
                pad,
                pad_config,
            },
        }
    }

    pub const fn func_in(
        name: &'static str,
        padname: &'static str,
        periph: PeriphIn,
        pad: Pad,
        pad_config: PadConfig,
    ) -> PinoutConfig {
        PinoutConfig {
            name,
            padname,
            pin: None,
            config: Config::Input {
                periph,
                pad,
                pad_config,
            },
        }
    }

    pub const fn func_out(
        name: &'static str,
        padname: &'static str,
        periph: Outsel,
        pad: Pad,
        pad_config: PadConfig,
    ) -> PinoutConfig {
        PinoutConfig {
            name,
            padname,
            pin: None,
            config: Config::Output {
                periph,
                pad,
                pad_config,
            },
        }
    }

    pub const fn func_io(
        name: &'static str,
        padname: &'static str,
        periph: PeriphIn,
        pad: Pad,
        pad_config: PadConfig,
    ) -> PinoutConfig {
        PinoutConfig {
            name,
            padname,
            pin: None,
            config: Config::Io {
                periph,
                pad,
                pad_config,
            },
        }
    }
}

pub const IN_PULL_NONE: PadConfig = PadConfig {
    pull: Pull::None,
    open_drain: false,
    invert: false,
};
pub const IN_PULL_UP: PadConfig = PadConfig {
    pull: Pull::Up,
    open_drain: false,
    invert: false,
};
pub const IN_PULL_DOWN: PadConfig = PadConfig {
    pull: Pull::Down,
    open_drain: false,
    invert: false,
};
pub const OUT_PUSH_PULL: PadConfig = PadConfig {
    pull: Pull::None,
    open_drain: false,
    invert: false,
};
pub const OUT_PULL_UP: PadConfig = PadConfig {
    pull: Pull::Up,
    open_drain: true,
    invert: false,
};
pub const OUT_PULL_DOWN: PadConfig = PadConfig {
    pull: Pull::Down,
    open_drain: true,
    invert: false,
};
