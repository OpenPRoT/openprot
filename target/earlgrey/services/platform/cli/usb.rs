// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::cli::{CliContext, CliError, CommandHandler, TokenIter};

pub struct UsbCommandHandler;

impl UsbCommandHandler {
    pub const fn new() -> Self {
        Self
    }

    pub fn print_help(&self) {
        util_zfmt::debug!("USB Commands:");
        util_zfmt::debug!("  info               - Display USB connection and multiplexer status");
        util_zfmt::debug!("  mux <host|device>  - Manually configure USB multiplexer route");
        util_zfmt::debug!("  help               - Display this help message");
    }

    fn handle_info(&self, ctx: &mut CliContext<'_>) -> Result<(), CliError> {
        let present = ctx
            .usb_mux
            .is_present(ctx.gpio)
            .map_err(|_| CliError::HardwareError)?;
        let host_routed = ctx
            .usb_mux
            .is_host_routed(ctx.gpio)
            .map_err(|_| CliError::HardwareError)?;
        util_zfmt::debug!("USB Status:");
        util_zfmt::debug!(
            "  Presence: {present}",
            present = if present { "connected" } else { "disconnected" }
        );
        util_zfmt::debug!(
            "  Route:    {route}",
            route = if host_routed { "host" } else { "device" }
        );
        util_zfmt::debug!(
            "  Pins:     presence=GPIO {pres}, mux_ctrl=GPIO {mux}",
            pres = u32::from(ctx.usb_mux.usb_presence_n),
            mux = u32::from(ctx.usb_mux.usb_mux_ctrl)
        );
        Ok(())
    }

    fn handle_mux(
        &self,
        tokens: &mut TokenIter<'_>,
        ctx: &mut CliContext<'_>,
    ) -> Result<(), CliError> {
        match tokens.next_token() {
            Some("host") => {
                ctx.usb_mux
                    .set_host_route(ctx.gpio, true)
                    .map_err(|_| CliError::HardwareError)?;
                util_zfmt::debug!("USB multiplexer routed to host");
                Ok(())
            }
            Some("device") => {
                ctx.usb_mux
                    .set_host_route(ctx.gpio, false)
                    .map_err(|_| CliError::HardwareError)?;
                util_zfmt::debug!("USB multiplexer routed to device");
                Ok(())
            }
            _ => {
                util_zfmt::debug!("Usage: usb mux <host|device>");
                Err(CliError::InvalidArguments)
            }
        }
    }
}

impl CommandHandler for UsbCommandHandler {
    fn name(&self) -> &'static str {
        "usb"
    }

    fn description(&self) -> &'static str {
        "USB status and multiplexer control"
    }

    fn execute(
        &mut self,
        tokens: &mut TokenIter<'_>,
        context: &mut CliContext<'_>,
    ) -> Result<(), CliError> {
        match tokens.next_token() {
            None | Some("help") => {
                self.print_help();
                Ok(())
            }
            Some("info") => self.handle_info(context),
            Some("mux") => self.handle_mux(tokens, context),
            Some(_) => {
                util_zfmt::debug!(
                    "Unknown usb subcommand. Type 'usb help' for available commands."
                );
                Err(CliError::UnknownCommand)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_command_metadata() {
        let handler = UsbCommandHandler::new();
        assert_eq!(handler.name(), "usb");
        assert_eq!(handler.description(), "USB status and multiplexer control");
    }
}
