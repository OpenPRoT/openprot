// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::cli::{CliContext, CliError, CommandHandler, TokenIter};
use earlgrey_gpio::{EarlGreyPinConfig, GpioMask};
use earlgrey_pinmux::{Pad, Pull};
use openprot_hal_blocking::gpio_port::GpioPort;

pub struct GpioPinDesc {
    pub index: u32,
    pub name: &'static str,
    pub pad: Option<Pad>,
}

pub const KNOWN_PINS: &[GpioPinDesc] = &[
    GpioPinDesc {
        index: 0,
        name: "RST_CTRL0_N",
        pad: Some(Pad::IOA0),
    },
    GpioPinDesc {
        index: 1,
        name: "RST_CTRL1_N",
        pad: Some(Pad::IOA1),
    },
    GpioPinDesc {
        index: 2,
        name: "SPI_RESET_N",
        pad: Some(Pad::IOA7),
    },
    GpioPinDesc {
        index: 3,
        name: "SPI_MUX_EN_N",
        pad: Some(Pad::IOB7),
    },
    GpioPinDesc {
        index: 4,
        name: "SPI_MUX_CTRL",
        pad: Some(Pad::IOB8),
    },
    GpioPinDesc {
        index: 5,
        name: "SPI_HOST0_WP_N",
        pad: Some(Pad::IOA3),
    },
    GpioPinDesc {
        index: 6,
        name: "SPI_HOST1_WP_N",
        pad: Some(Pad::IOA6),
    },
    GpioPinDesc {
        index: 7,
        name: "USB_MUX_CTRL",
        pad: Some(Pad::IOC6),
    },
    GpioPinDesc {
        index: 8,
        name: "EXT_DEBUG_N",
        pad: Some(Pad::IOC9),
    },
    GpioPinDesc {
        index: 16,
        name: "USB_PRESENCE_N",
        pad: Some(Pad::IOR11),
    },
    GpioPinDesc {
        index: 17,
        name: "RST_MON0_N",
        pad: Some(Pad::IOA2),
    },
    GpioPinDesc {
        index: 18,
        name: "RST_MON1_N",
        pad: Some(Pad::IOA5),
    },
    GpioPinDesc {
        index: 22,
        name: "SW_STRAP0",
        pad: Some(Pad::IOC0),
    },
    GpioPinDesc {
        index: 23,
        name: "SW_STRAP1",
        pad: Some(Pad::IOC1),
    },
    GpioPinDesc {
        index: 24,
        name: "SW_STRAP2",
        pad: Some(Pad::IOC2),
    },
];

pub fn pad_name(pad: Pad) -> &'static str {
    match pad {
        Pad::IOA0 => "IOA0",
        Pad::IOA1 => "IOA1",
        Pad::IOA2 => "IOA2",
        Pad::IOA3 => "IOA3",
        Pad::IOA4 => "IOA4",
        Pad::IOA5 => "IOA5",
        Pad::IOA6 => "IOA6",
        Pad::IOA7 => "IOA7",
        Pad::IOA8 => "IOA8",
        Pad::IOB0 => "IOB0",
        Pad::IOB1 => "IOB1",
        Pad::IOB2 => "IOB2",
        Pad::IOB3 => "IOB3",
        Pad::IOB4 => "IOB4",
        Pad::IOB5 => "IOB5",
        Pad::IOB6 => "IOB6",
        Pad::IOB7 => "IOB7",
        Pad::IOB8 => "IOB8",
        Pad::IOB9 => "IOB9",
        Pad::IOB10 => "IOB10",
        Pad::IOB11 => "IOB11",
        Pad::IOB12 => "IOB12",
        Pad::IOC0 => "IOC0",
        Pad::IOC1 => "IOC1",
        Pad::IOC2 => "IOC2",
        Pad::IOC3 => "IOC3",
        Pad::IOC4 => "IOC4",
        Pad::IOC5 => "IOC5",
        Pad::IOC6 => "IOC6",
        Pad::IOC7 => "IOC7",
        Pad::IOC8 => "IOC8",
        Pad::IOC9 => "IOC9",
        Pad::IOC10 => "IOC10",
        Pad::IOC11 => "IOC11",
        Pad::IOC12 => "IOC12",
        Pad::IOR0 => "IOR0",
        Pad::IOR1 => "IOR1",
        Pad::IOR2 => "IOR2",
        Pad::IOR3 => "IOR3",
        Pad::IOR4 => "IOR4",
        Pad::IOR5 => "IOR5",
        Pad::IOR6 => "IOR6",
        Pad::IOR7 => "IOR7",
        Pad::IOR10 => "IOR10",
        Pad::IOR11 => "IOR11",
        Pad::IOR12 => "IOR12",
        Pad::IOR13 => "IOR13",
        _ => "OTHER",
    }
}

pub fn resolve_pin(token: &str) -> Option<(u32, Option<Pad>)> {
    if let Ok(idx) = token.parse::<u32>() {
        if idx < 32 {
            let pad = KNOWN_PINS
                .iter()
                .find(|p| p.index == idx)
                .and_then(|p| p.pad);
            return Some((idx, pad));
        }
    }
    if token.len() > 4 && token[..4].eq_ignore_ascii_case("gpio") {
        if let Ok(idx) = token[4..].parse::<u32>() {
            if idx < 32 {
                let pad = KNOWN_PINS
                    .iter()
                    .find(|p| p.index == idx)
                    .and_then(|p| p.pad);
                return Some((idx, pad));
            }
        }
    }
    for p in KNOWN_PINS {
        if p.name.eq_ignore_ascii_case(token) {
            return Some((p.index, p.pad));
        }
        if let Some(pad) = p.pad {
            if pad_name(pad).eq_ignore_ascii_case(token) {
                return Some((p.index, p.pad));
            }
        }
    }
    None
}

pub struct GpioCommandHandler;

impl GpioCommandHandler {
    pub const fn new() -> Self {
        Self
    }

    pub fn print_help(&self) {
        util_zfmt::debug!("GPIO Commands:");
        util_zfmt::debug!(
            "  list                              - List configured GPIO pins and status"
        );
        util_zfmt::debug!("  read <pin>                        - Read GPIO input/output state");
        util_zfmt::debug!("  write <pin> <0|1>                 - Drive output level with readback");
        util_zfmt::debug!(
            "  config <pin> <in|out|inout> [pull] - Configure direction and pull (none/up/down)"
        );
        util_zfmt::debug!(
            "  attr <pin> <od|pp>                - Configure pad open-drain or push-pull"
        );
        util_zfmt::debug!("  help                              - Display this help message");
    }
}

impl CommandHandler for GpioCommandHandler {
    fn name(&self) -> &'static str {
        "gpio"
    }

    fn description(&self) -> &'static str {
        "GPIO pin configuration and control"
    }

    fn execute(
        &mut self,
        tokens: &mut TokenIter<'_>,
        context: &mut CliContext<'_>,
    ) -> Result<(), CliError> {
        let Some(subcmd) = tokens.next_token() else {
            self.print_help();
            return Ok(());
        };

        match subcmd {
            "help" => {
                self.print_help();
                Ok(())
            }
            "list" => {
                let in_mask = context
                    .gpio
                    .read_input()
                    .map_err(|_| CliError::HardwareError)?
                    .0;
                let out_mask = context
                    .gpio
                    .read_output()
                    .map_err(|_| CliError::HardwareError)?
                    .0;
                let oe_mask = context
                    .gpio
                    .read_oe()
                    .map_err(|_| CliError::HardwareError)?
                    .0;

                util_zfmt::debug!("GPIO Pin Status:");
                for p in KNOWN_PINS {
                    let in_val = ((in_mask >> p.index) & 1) as u32;
                    let out_val = ((out_mask >> p.index) & 1) as u32;
                    let oe_val = ((oe_mask >> p.index) & 1) as u32;
                    let dir = if oe_val != 0 { "OUT" } else { "IN" };
                    let pad_str = p.pad.map(pad_name).unwrap_or("none");
                    util_zfmt::debug!(
                        "  GPIO {idx:02} ({name}/{pad}): dir={dir}, in={in_val}, out={out_val}",
                        idx = p.index,
                        name = p.name,
                        pad = pad_str,
                        dir = dir,
                        in_val = in_val,
                        out_val = out_val,
                    );
                }
                Ok(())
            }
            "read" => {
                let Some(pin_token) = tokens.next_token() else {
                    util_zfmt::debug!("Usage: gpio read <pin>");
                    return Err(CliError::MissingArguments);
                };
                let Some((pin_idx, _)) = resolve_pin(pin_token) else {
                    util_zfmt::debug!("Unknown pin: {pin}", pin = pin_token);
                    return Err(CliError::InvalidArguments);
                };
                let in_val = ((context
                    .gpio
                    .read_input()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                let out_val = ((context
                    .gpio
                    .read_output()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                let oe_val = ((context
                    .gpio
                    .read_oe()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                util_zfmt::debug!(
                    "GPIO {pin}: in={in_val} (oe={oe_val}, out={out_val})",
                    pin = pin_token,
                    in_val = in_val,
                    oe_val = oe_val,
                    out_val = out_val,
                );
                Ok(())
            }
            "write" => {
                let Some(pin_token) = tokens.next_token() else {
                    util_zfmt::debug!("Usage: gpio write <pin> <0|1>");
                    return Err(CliError::MissingArguments);
                };
                let Some(val_token) = tokens.next_token() else {
                    util_zfmt::debug!("Usage: gpio write <pin> <0|1>");
                    return Err(CliError::MissingArguments);
                };
                let Some((pin_idx, _)) = resolve_pin(pin_token) else {
                    util_zfmt::debug!("Unknown pin: {pin}", pin = pin_token);
                    return Err(CliError::InvalidArguments);
                };
                let val = match val_token {
                    "0" | "low" => 0u32,
                    "1" | "high" => 1u32,
                    _ => {
                        util_zfmt::debug!("Invalid level: {val}. Expected 0 or 1", val = val_token);
                        return Err(CliError::InvalidArguments);
                    }
                };

                let mask = GpioMask(1 << pin_idx);
                if val == 1 {
                    context
                        .gpio
                        .set_reset(mask, GpioMask(0))
                        .map_err(|_| CliError::HardwareError)?;
                } else {
                    context
                        .gpio
                        .set_reset(GpioMask(0), mask)
                        .map_err(|_| CliError::HardwareError)?;
                }

                // Immediate readback
                let out_val = ((context
                    .gpio
                    .read_output()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                let in_val = ((context
                    .gpio
                    .read_input()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                let oe_val = ((context
                    .gpio
                    .read_oe()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                util_zfmt::debug!(
                    "GPIO {pin} written {val} -> readback: out={out_val}, in={in_val}, oe={oe_val}",
                    pin = pin_token,
                    val = val,
                    out_val = out_val,
                    in_val = in_val,
                    oe_val = oe_val,
                );
                Ok(())
            }
            "config" => {
                let Some(pin_token) = tokens.next_token() else {
                    util_zfmt::debug!(
                        "Usage: gpio config <pin> <in|out|inout> [none|pullup|pulldown]"
                    );
                    return Err(CliError::MissingArguments);
                };
                let Some(dir_token) = tokens.next_token() else {
                    util_zfmt::debug!(
                        "Usage: gpio config <pin> <in|out|inout> [none|pullup|pulldown]"
                    );
                    return Err(CliError::MissingArguments);
                };
                let Some((pin_idx, pad)) = resolve_pin(pin_token) else {
                    util_zfmt::debug!("Unknown pin: {pin}", pin = pin_token);
                    return Err(CliError::InvalidArguments);
                };
                let (is_input, is_output) = match dir_token {
                    "in" | "input" => (true, false),
                    "out" | "output" => (false, true),
                    "inout" => (true, true),
                    _ => {
                        util_zfmt::debug!(
                            "Invalid direction: {dir}. Expected in, out, or inout",
                            dir = dir_token
                        );
                        return Err(CliError::InvalidArguments);
                    }
                };
                let pull_token = tokens.next_token().unwrap_or("none");
                let pull = match pull_token {
                    "none" => Pull::None,
                    "up" | "pullup" => Pull::Up,
                    "down" | "pulldown" => Pull::Down,
                    _ => {
                        util_zfmt::debug!(
                            "Invalid pull: {pull}. Expected none, up, or down",
                            pull = pull_token
                        );
                        return Err(CliError::InvalidArguments);
                    }
                };

                let mask = GpioMask(1 << pin_idx);
                let cfg = EarlGreyPinConfig {
                    is_input,
                    is_output,
                    input_filter: false,
                    pad,
                    pull,
                };
                context
                    .gpio
                    .configure(mask, cfg)
                    .map_err(|_| CliError::HardwareError)?;

                // Immediate readback
                let oe_val = ((context
                    .gpio
                    .read_oe()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                let in_val = ((context
                    .gpio
                    .read_input()
                    .map_err(|_| CliError::HardwareError)?
                    .0
                    >> pin_idx)
                    & 1) as u32;
                util_zfmt::debug!(
                    "GPIO {pin} configured -> oe={oe_val}, in={in_val}",
                    pin = pin_token,
                    oe_val = oe_val,
                    in_val = in_val,
                );
                Ok(())
            }
            "attr" => {
                let Some(pin_token) = tokens.next_token() else {
                    util_zfmt::debug!("Usage: gpio attr <pin> <od|pp>");
                    return Err(CliError::MissingArguments);
                };
                let Some(mode_token) = tokens.next_token() else {
                    util_zfmt::debug!("Usage: gpio attr <pin> <od|pp>");
                    return Err(CliError::MissingArguments);
                };
                let Some((_pin_idx, pad)) = resolve_pin(pin_token) else {
                    util_zfmt::debug!("Unknown pin: {pin}", pin = pin_token);
                    return Err(CliError::InvalidArguments);
                };
                let open_drain = match mode_token {
                    "od" | "opendrain" => true,
                    "pp" | "pushpull" => false,
                    _ => {
                        util_zfmt::debug!(
                            "Invalid attr mode: {mode}. Expected od or pp",
                            mode = mode_token
                        );
                        return Err(CliError::InvalidArguments);
                    }
                };
                let Some(pad) = pad else {
                    util_zfmt::debug!("GPIO {pin} has no associated pad", pin = pin_token);
                    return Err(CliError::InvalidArguments);
                };

                let mut pad_cfg = context
                    .gpio
                    .pinmux
                    .get_pad_config(pad)
                    .map_err(|_| CliError::HardwareError)?;
                pad_cfg.open_drain = open_drain;
                context
                    .gpio
                    .pinmux
                    .configure_pad(pad, &pad_cfg)
                    .map_err(|_| CliError::HardwareError)?;

                // Immediate readback
                let rb = context
                    .gpio
                    .pinmux
                    .get_pad_config(pad)
                    .map_err(|_| CliError::HardwareError)?;
                let mode_str = if rb.open_drain {
                    "open-drain"
                } else {
                    "push-pull"
                };
                util_zfmt::debug!(
                    "GPIO {pin} ({pad}) pad attr -> {mode}",
                    pin = pin_token,
                    pad = pad_name(pad),
                    mode = mode_str,
                );
                Ok(())
            }
            _ => {
                util_zfmt::debug!("Unknown gpio subcommand. Type 'gpio help' for usage.");
                Err(CliError::UnknownCommand)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_pin_numeric() {
        assert_eq!(resolve_pin("0"), Some((0, Some(Pad::IOA0))));
        assert_eq!(resolve_pin("1"), Some((1, Some(Pad::IOA1))));
        assert_eq!(resolve_pin("16"), Some((16, Some(Pad::IOR11))));
        assert_eq!(resolve_pin("31"), Some((31, None)));
        assert_eq!(resolve_pin("32"), None);
    }

    #[test]
    fn test_resolve_pin_gpio_prefix() {
        assert_eq!(resolve_pin("gpio0"), Some((0, Some(Pad::IOA0))));
        assert_eq!(resolve_pin("GPIO1"), Some((1, Some(Pad::IOA1))));
        assert_eq!(resolve_pin("gpio16"), Some((16, Some(Pad::IOR11))));
    }

    #[test]
    fn test_resolve_pin_name() {
        assert_eq!(resolve_pin("RST_CTRL0_N"), Some((0, Some(Pad::IOA0))));
        assert_eq!(resolve_pin("rst_ctrl0_n"), Some((0, Some(Pad::IOA0))));
        assert_eq!(resolve_pin("EXT_DEBUG_N"), Some((8, Some(Pad::IOC9))));
    }

    #[test]
    fn test_resolve_pin_pad() {
        assert_eq!(resolve_pin("IOA0"), Some((0, Some(Pad::IOA0))));
        assert_eq!(resolve_pin("ioa0"), Some((0, Some(Pad::IOA0))));
        assert_eq!(resolve_pin("IOC9"), Some((8, Some(Pad::IOC9))));
    }
}
