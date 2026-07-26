// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::{EarlGreyGpio, EarlGreyPinConfig, GpioMask};
use earlgrey_pinmux::{EarlGreyPinmux, Pull};
use earlgrey_util_error::EG_PINMUX_INVALID_STRAP_CONFIG;
use openprot_hal_blocking::gpio_port::{GpioPort, PinMask};
use top_earlgrey::PinmuxOutsel as Outsel;
use userspace::time::{sleep_until, Clock, Duration, SystemClock};
use util_error::ErrorCode;

use crate::config::{Config, PinoutConfig};
use crate::swstraps::SwStraps;
use crate::Pinout;

impl Config {
    pub fn apply(&self, pinmux: &mut EarlGreyPinmux) -> Result<(), ErrorCode> {
        match self {
            Self::Input {
                periph,
                pad,
                pad_config,
            } => {
                pinmux.configure_pad(*pad, pad_config)?;
                pinmux.connect_input(*periph, *pad)?;
            }
            Self::Output {
                periph,
                pad,
                pad_config,
            } => {
                pinmux.configure_pad(*pad, pad_config)?;
                pinmux.connect_output(*pad, *periph)?;
            }
            Self::Io {
                periph,
                pad,
                pad_config,
            } => {
                pinmux.configure_pad(*pad, pad_config)?;
                pinmux.connect_input(*periph, *pad)?;
                // For the range of peripherals that are valid as input and output,
                // (Gpio0 to SpiHost1Sd3), the offset between PeriphIn and Outsel
                // is 3.
                // TODO: should we check this and return an error?
                let outsel = Outsel::try_from(*periph as u32 + 3).unwrap();
                pinmux.connect_output(*pad, outsel)?;
            }
        }
        Ok(())
    }
}

impl PinoutConfig {
    pub fn apply(&self, gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        self.config.apply(&mut gpio.pinmux)?;
        if let Some(gpio_pin) = self.pin {
            gpio.configure(
                gpio_pin.into(),
                EarlGreyPinConfig {
                    is_input: self.config.is_input(),
                    is_output: self.config.is_output(),
                    input_filter: false,
                    pad: None,
                    pull: Pull::None,
                },
            )
            .map_err(ErrorCode::from)?;
        }
        Ok(())
    }

    pub fn configure(pinout: &[Self], gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        for pin in pinout {
            pin.apply(gpio)?;
        }
        Ok(())
    }
}

impl SwStraps {
    const PAD_DELAY: Duration = Duration::from_micros(50);

    fn read_strap(pin: &PinoutConfig, gpio: &mut EarlGreyGpio) -> Result<u32, ErrorCode> {
        let Config::Input { pad, .. } = &pin.config else {
            return Err(EG_PINMUX_INVALID_STRAP_CONFIG);
        };
        let Some(gpio_pin) = pin.pin else {
            return Err(EG_PINMUX_INVALID_STRAP_CONFIG);
        };
        let pin_mask = GpioMask::from(gpio_pin);
        // 1. Configure for no pull.
        gpio.configure(
            pin_mask,
            EarlGreyPinConfig {
                is_input: pin.config.is_input(),
                is_output: pin.config.is_output(),
                input_filter: false,
                pad: Some(*pad),
                pull: Pull::None,
            },
        )
        .map_err(ErrorCode::from)?;

        // 2. Delay 50us
        let _ = sleep_until(SystemClock::now() + Self::PAD_DELAY);

        // 3. Read the high bit of the strap.
        let val1 = if gpio
            .read_input()
            .map_err(ErrorCode::from)?
            .contains(pin_mask)
        {
            2
        } else {
            0
        };

        // 4. Configure pull opposite to val1
        let pull = if val1 == 0 { Pull::Up } else { Pull::Down };
        gpio.configure(
            pin_mask,
            EarlGreyPinConfig {
                is_input: pin.config.is_input(),
                is_output: pin.config.is_output(),
                input_filter: false,
                pad: Some(*pad),
                pull,
            },
        )
        .map_err(ErrorCode::from)?;

        // 5. Delay 50us
        let _ = sleep_until(SystemClock::now() + Self::PAD_DELAY);

        // 6. Read the low bit of the strap.
        let val2 = if gpio
            .read_input()
            .map_err(ErrorCode::from)?
            .contains(pin_mask)
        {
            1
        } else {
            0
        };

        Ok(val1 | val2)
    }

    pub fn read_straps(gpio: &mut EarlGreyGpio) -> Result<u32, ErrorCode> {
        let mut result = 0;
        for pin in Self::PINOUT {
            result <<= 2;
            result |= Self::read_strap(pin, gpio)?;
        }
        Ok(result)
    }
}
