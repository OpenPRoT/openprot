// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::cli::{CliContext, CliError, CommandHandler, TokenIter};
use util_ipc::IpcChannel;
use zerocopy::IntoBytes;

pub struct FlashCommandHandler;

impl FlashCommandHandler {
    pub const fn new() -> Self {
        Self
    }

    pub fn print_help(&self) {
        util_zfmt::debug!("Flash Commands:");
        util_zfmt::debug!("  info                - Display SPI flash multiplexer and pin status");
        util_zfmt::debug!("  mux <en|dis>        - Enable or disable SPI multiplexer");
        util_zfmt::debug!("  route <target|host> - Configure SPI multiplexer route");
        util_zfmt::debug!("  read-id [0|1]       - Read SPI flash JEDEC ID and chip information");
        util_zfmt::debug!("  help                - Display this help message");
    }

    fn handle_info(&self, ctx: &mut CliContext<'_>) -> Result<(), CliError> {
        let enabled = ctx
            .spi_mux
            .is_mux_enabled(ctx.gpio)
            .map_err(|_| CliError::HardwareError)?;
        let host = ctx
            .spi_mux
            .is_route_host(ctx.gpio)
            .map_err(|_| CliError::HardwareError)?;
        util_zfmt::debug!("SPI Flash Status:");
        util_zfmt::debug!(
            "  Mux:   {mux}",
            mux = if enabled { "enabled" } else { "disabled" }
        );
        util_zfmt::debug!(
            "  Route: {route}",
            route = if host { "host" } else { "target" }
        );
        util_zfmt::debug!(
            "  Pins:  en_n=GPIO {en}, ctrl=GPIO {ctrl}, rst_n=GPIO {rst}",
            en = u32::from(ctx.spi_mux.spi_mux_en_n),
            ctrl = u32::from(ctx.spi_mux.spi_mux_ctrl),
            rst = u32::from(ctx.spi_mux.spi_reset_n),
        );
        util_zfmt::debug!(
            "  WP:    host0_wp_n=GPIO {wp0}, host1_wp_n=GPIO {wp1}",
            wp0 = u32::from(ctx.spi_mux.spi_host0_wp_n),
            wp1 = u32::from(ctx.spi_mux.spi_host1_wp_n),
        );
        Ok(())
    }

    fn handle_mux(
        &self,
        tokens: &mut TokenIter<'_>,
        ctx: &mut CliContext<'_>,
    ) -> Result<(), CliError> {
        match tokens.next_token() {
            Some("en") | Some("enable") => {
                ctx.spi_mux
                    .set_mux_enabled(ctx.gpio, true)
                    .map_err(|_| CliError::HardwareError)?;
                util_zfmt::debug!("SPI multiplexer enabled");
                Ok(())
            }
            Some("dis") | Some("disable") => {
                ctx.spi_mux
                    .set_mux_enabled(ctx.gpio, false)
                    .map_err(|_| CliError::HardwareError)?;
                util_zfmt::debug!("SPI multiplexer disabled");
                Ok(())
            }
            _ => {
                util_zfmt::debug!("Usage: flash mux <en|dis>");
                Err(CliError::InvalidArguments)
            }
        }
    }

    fn handle_route(
        &self,
        tokens: &mut TokenIter<'_>,
        ctx: &mut CliContext<'_>,
    ) -> Result<(), CliError> {
        match tokens.next_token() {
            Some("target") => {
                ctx.spi_mux
                    .set_route_host(ctx.gpio, false)
                    .map_err(|_| CliError::HardwareError)?;
                util_zfmt::debug!("SPI multiplexer routed to target");
                Ok(())
            }
            Some("host") => {
                ctx.spi_mux
                    .set_route_host(ctx.gpio, true)
                    .map_err(|_| CliError::HardwareError)?;
                util_zfmt::debug!("SPI multiplexer routed to host");
                Ok(())
            }
            _ => {
                util_zfmt::debug!("Usage: flash route <target|host>");
                Err(CliError::InvalidArguments)
            }
        }
    }

    fn handle_read_id(
        &self,
        tokens: &mut TokenIter<'_>,
        ctx: &mut CliContext<'_>,
    ) -> Result<(), CliError> {
        let mux_enabled = ctx
            .spi_mux
            .is_mux_enabled(ctx.gpio)
            .map_err(|_| CliError::HardwareError)?;
        let route_host = ctx
            .spi_mux
            .is_route_host(ctx.gpio)
            .map_err(|_| CliError::HardwareError)?;

        let eeprom_idx = match tokens.next_token() {
            Some("0") => {
                if mux_enabled && !route_host {
                    util_zfmt::debug!("Error: EEPROM 0 is currently routed to upstream device. Change route with 'flash route' or disable mux first.");
                    return Err(CliError::HardwareError);
                }
                0u8
            }
            Some("1") => {
                if mux_enabled && route_host {
                    util_zfmt::debug!("Error: EEPROM 1 is currently routed to upstream device. Change route with 'flash route' or disable mux first.");
                    return Err(CliError::HardwareError);
                }
                1u8
            }
            Some(_) => {
                util_zfmt::debug!("Usage: flash read-id [0|1]");
                return Err(CliError::InvalidArguments);
            }
            None => {
                if mux_enabled {
                    if route_host {
                        0u8
                    } else {
                        1u8
                    }
                } else {
                    0u8
                }
            }
        };

        let op = services_flash_opcode::ReadIdOp {
            eeprom_index: eeprom_idx,
        };
        let mut status = 0u32;
        let mut jedec = services_flash_opcode::JedecIdResp::default();
        let _ = ctx
            .flash_ipc
            .transact(
                &[
                    services_flash_opcode::IPC_OP_FLASH_READ_ID.as_bytes(),
                    op.as_bytes(),
                ],
                &mut [status.as_mut_bytes(), jedec.as_mut_bytes()],
                userspace::time::Instant::MAX,
            )
            .map_err(|_| CliError::HardwareError)?;

        if status != 0 {
            util_zfmt::debug!(
                "Error: Failed to read JEDEC ID from EEPROM {idx} (error {status:08x}).",
                idx = u32::from(eeprom_idx),
                status = status
            );
            return Err(CliError::HardwareError);
        }

        let mfr = jedec.manufacturer;
        let mem_type = jedec.memory_type;
        let cap = jedec.capacity_code;

        if (mfr == 0xFF && mem_type == 0xFF && cap == 0xFF)
            || (mfr == 0 && mem_type == 0 && cap == 0)
        {
            util_zfmt::debug!(
                "Error: No response from EEPROM {idx} (JEDEC ID: {mfr:02x} {mem:02x} {cap:02x}).",
                idx = u32::from(eeprom_idx),
                mfr = u32::from(mfr),
                mem = u32::from(mem_type),
                cap = u32::from(cap),
            );
            return Err(CliError::HardwareError);
        }

        let mfr_str = match mfr {
            0xEF => "Winbond",
            0xC2 => "Macronix",
            0x20 => "Micron",
            0x1F => "Adesto",
            0x9D => "ISSI",
            0x01 => "Spansion",
            0xBF => "SST",
            0xC8 => "GigaDevice",
            _ => "Unknown",
        };

        let device_str = match (mfr, mem_type, cap) {
            (0xEF, _, 0x15) => "W25Q16",
            (0xEF, _, 0x16) => "W25Q32",
            (0xEF, _, 0x17) => "W25Q64",
            (0xEF, _, 0x18) => "W25Q128",
            (0xEF, _, 0x19) => "W25Q256",
            (0xEF, _, 0x20) => "W25Q512",
            (0xC2, 0x20, 0x15) => "MX25L16",
            (0xC2, 0x20, 0x16) => "MX25L32",
            (0xC2, 0x20, 0x17) => "MX25L64",
            (0xC2, 0x20, 0x18) => "MX25L128",
            (0xC2, 0x20, 0x19) => "MX25L256",
            (0xC2, 0x20, 0x1A) => "MX25L512",
            (0xC2, 0x25, 0x38) => "MX25U128",
            (0xC2, 0x25, 0x39) => "MX25U256",
            (0xC2, 0x25, 0x3A) => "MX25U51245G / MX66U51235F",
            (0xC2, _, 0x3A) => "MX25U512 / MX66U512",
            (0xC2, _, 0x1A) => "MX25L512",
            _ => "Generic NOR",
        };

        util_zfmt::debug!("EEPROM {idx} Status:", idx = u32::from(eeprom_idx));
        util_zfmt::debug!(
            "  JEDEC ID:     0x{mfr:02x} 0x{mem:02x} 0x{cap:02x}",
            mfr = u32::from(mfr),
            mem = u32::from(mem_type),
            cap = u32::from(cap),
        );
        util_zfmt::debug!(
            "  Manufacturer: {name} (0x{mfr:02x})",
            name = mfr_str,
            mfr = u32::from(mfr)
        );
        util_zfmt::debug!("  Device:       {dev}", dev = device_str);

        let density_power = if (0x10..=0x24).contains(&cap) {
            Some(cap)
        } else if (0x30..=0x3C).contains(&cap) {
            // Macronix 1.8V family uses 0x30 base (e.g. 0x3A -> 2^26 = 64 MiB / 512 Mbit)
            Some(0x10 + (cap & 0x0F))
        } else {
            None
        };

        if let Some(power) = density_power {
            let bytes = 1usize << power;
            let mib = bytes / (1024 * 1024);
            if mib > 0 {
                util_zfmt::debug!(
                    "  Density:      {mib} MiB ({bytes} bytes)",
                    mib = mib as u32,
                    bytes = bytes as u32
                );
            } else {
                let kib = bytes / 1024;
                util_zfmt::debug!(
                    "  Density:      {kib} KiB ({bytes} bytes)",
                    kib = kib as u32,
                    bytes = bytes as u32
                );
            }
        } else {
            util_zfmt::debug!("  Density:      Unknown");
        }

        Ok(())
    }
}

impl CommandHandler for FlashCommandHandler {
    fn name(&self) -> &'static str {
        "flash"
    }

    fn description(&self) -> &'static str {
        "Flash status and memory info"
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
            Some("route") => self.handle_route(tokens, context),
            Some("read-id") | Some("id") => self.handle_read_id(tokens, context),
            Some(_) => {
                util_zfmt::debug!(
                    "Unknown flash subcommand. Type 'flash help' for available commands."
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
    fn test_flash_command_metadata() {
        let handler = FlashCommandHandler::new();
        assert_eq!(handler.name(), "flash");
        assert_eq!(handler.description(), "Flash status and memory info");
    }
}
