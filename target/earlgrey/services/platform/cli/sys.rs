// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::cli::{CliContext, CliError, CommandHandler, TokenIter};

pub struct SysCommandHandler;

impl SysCommandHandler {
    pub const fn new() -> Self {
        Self
    }

    pub fn print_help(&self) {
        util_zfmt::debug!("System Commands:");
        util_zfmt::debug!("  info  - Display system and chip boot information");
        util_zfmt::debug!("  id    - Display OpenTitan 256-bit device ID");
        util_zfmt::debug!("  reset - Trigger system software reboot");
        util_zfmt::debug!("  help  - Display this help message");
    }

    fn handle_info(&self, ctx: &mut CliContext<'_>) -> Result<(), CliError> {
        let boot_info = ctx
            .sysmgr
            .get_boot_info()
            .map_err(|_| CliError::HardwareError)?;
        util_zfmt::debug!("System Information:");
        util_zfmt::debug!(
            "  Chip: OpenTitan {creator:04x}-{product:04x}-{rev:02x}",
            creator = boot_info.chip.creator_id,
            product = boot_info.chip.product_id,
            rev = boot_info.chip.revision
        );
        util_zfmt::debug!(
            "  ROM_EXT: {major}.{minor} (slot={slot})",
            major = boot_info.rom_ext.major,
            minor = boot_info.rom_ext.minor,
            slot = boot_info.rom_ext.boot_slot.as_str()
        );
        util_zfmt::debug!(
            "  App: size={size} (slot={slot}/pref={pref})",
            size = boot_info.app.size,
            slot = boot_info.app.boot_slot.as_str(),
            pref = boot_info.app.pref_slot.as_str()
        );
        util_zfmt::debug!(
            "  Reset: reason=0x{reason:02x}, straps HW=0x{hw:02x}, SW=0x{sw:02x}",
            reason = boot_info.reset.reason,
            hw = boot_info.reset.hardware_straps,
            sw = boot_info.reset.software_straps
        );
        Ok(())
    }

    fn handle_id(&self, ctx: &mut CliContext<'_>) -> Result<(), CliError> {
        let boot_info = ctx
            .sysmgr
            .get_boot_info()
            .map_err(|_| CliError::HardwareError)?;
        let id = &boot_info.chip.device_id;
        util_zfmt::debug!(
            "Device ID: {d7:08x}{d6:08x}{d5:08x}{d4:08x}{d3:08x}{d2:08x}{d1:08x}{d0:08x}",
            d7 = id[7],
            d6 = id[6],
            d5 = id[5],
            d4 = id[4],
            d3 = id[3],
            d2 = id[2],
            d1 = id[1],
            d0 = id[0]
        );
        Ok(())
    }

    fn handle_reset(&self, ctx: &mut CliContext<'_>) -> Result<(), CliError> {
        util_zfmt::debug!("Triggering system reset...");
        ctx.sysmgr
            .request_reboot()
            .map_err(|_| CliError::HardwareError)?;
        Ok(())
    }
}

impl CommandHandler for SysCommandHandler {
    fn name(&self) -> &'static str {
        "sys"
    }

    fn description(&self) -> &'static str {
        "System information and reset control"
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
            Some("id") => self.handle_id(context),
            Some("reset") => self.handle_reset(context),
            Some(_) => {
                util_zfmt::debug!(
                    "Unknown sys subcommand. Type 'sys help' for available commands."
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
    fn test_sys_command_metadata() {
        let handler = SysCommandHandler::new();
        assert_eq!(handler.name(), "sys");
        assert_eq!(
            handler.description(),
            "System information and reset control"
        );
    }
}
