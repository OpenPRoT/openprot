// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device Firmware Upgrade (DFU) tracker layout and state tracking helper.

#![no_std]

use earlgrey_sysmgr_client::BootInfo;
use earlgrey_util::error::EG_ERROR_BOOT_SLOT_UNKNOWN;
use earlgrey_util::manifest::Manifest;
use earlgrey_util::tags::{BootSlot, ManifestIdentifier};
use util_error::ErrorCode;
use util_io::RandomRead;
use zerocopy::{FromZeros, IntoBytes};

/// State of the firmware update process.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FwUpdateState {
    /// Idle, waiting for the first block of firmware.
    Idle,
    /// Flashing ROM_EXT.
    RomExt,
    /// Flashing Application.
    Application,
    /// Firmware download complete.
    Done,
}

/// Helper struct to track the progress and target partitions for a firmware update.
///
/// It uses an A/B partitioning scheme, targeting the inactive slot.
pub struct FwUpdate {
    /// Current state of the update process.
    pub state: FwUpdateState,
    /// Next expected block number that triggers a partition erase.
    pub next_erase: u32,
    /// The block number where the current image (ROM_EXT or App) download started.
    pub start_block: u32,
    /// Target boot slot for ROM_EXT.
    pub rom_ext: BootSlot,
    /// Start address of target ROM_EXT partition in flash.
    pub rom_ext_start: u32,
    /// End address of target ROM_EXT partition in flash.
    pub rom_ext_end: u32,
    /// Target boot slot for Application.
    pub app: BootSlot,
    /// Start address of target Application partition in flash.
    pub app_start: u32,
    /// End address of target Application partition in flash.
    pub app_end: u32,
    /// Size of target ROM_EXT partition in bytes.
    pub rom_ext_size: usize,
    /// Size of target Application partition in bytes.
    pub app_size: usize,
}

impl FwUpdate {
    /// Creates a new `FwUpdate` tracker.
    ///
    /// It queries the current boot info to determine the active slots,
    /// and targets the *opposite* (inactive) slots for the update.
    pub fn new(info: &BootInfo) -> Result<Self, ErrorCode> {
        let rom_ext = info
            .rom_ext
            .boot_slot
            .opposite()
            .ok_or(EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let rom_ext_start = FwUpdate::addr(rom_ext);
        let app = info
            .app
            .boot_slot
            .opposite()
            .ok_or(EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let app_start = FwUpdate::addr(app) + info.rom_ext.size;

        Ok(FwUpdate {
            state: FwUpdateState::Idle,
            next_erase: 0,
            start_block: 0,
            rom_ext,
            rom_ext_start,
            rom_ext_end: rom_ext_start + info.rom_ext.size,
            app,
            app_start,
            app_end: app_start + info.app.size,
            rom_ext_size: info.rom_ext.size as usize,
            app_size: info.app.size as usize,
        })
    }

    /// Returns the physical flash address offset for a given boot slot.
    pub fn addr(slot: BootSlot) -> u32 {
        match slot {
            BootSlot::SlotA => 0,
            BootSlot::SlotB => 0x80000,
            _ => unreachable!(),
        }
    }

    /// Scans the provided reader to find a compatible firmware bundle.
    pub fn scan_firmware_bundle(
        &self,
        flash: &mut impl RandomRead<Error = ErrorCode>,
    ) -> Result<Option<FirmwareBundle>, ErrorCode> {
        let flash_size = flash.size()?;
        let step = self.rom_ext_size;

        let mut offset = 0;
        while offset + self.rom_ext_size + self.app_size <= flash_size {
            if let Some(bundle) = self.try_read_bundle_at(flash, offset)? {
                return Ok(Some(bundle));
            }
            offset += step;
        }

        Ok(None)
    }

    fn try_read_bundle_at(
        &self,
        flash: &mut impl RandomRead<Error = ErrorCode>,
        offset: usize,
    ) -> Result<Option<FirmwareBundle>, ErrorCode> {
        let mut rom_ext_hdr = Manifest::new_zeroed();
        if flash.read(offset, rom_ext_hdr.as_mut_bytes()).is_err() {
            return Ok(None);
        }
        if rom_ext_hdr.identifier != ManifestIdentifier::ROM_EXT {
            return Ok(None);
        }
        let rom_ext_len = rom_ext_hdr.length as usize;
        if rom_ext_len > self.rom_ext_size {
            return Ok(None);
        }

        let mut owner_hdr = Manifest::new_zeroed();
        if flash
            .read(offset + self.rom_ext_size, owner_hdr.as_mut_bytes())
            .is_err()
        {
            return Ok(None);
        }
        if owner_hdr.identifier != ManifestIdentifier::APPLICATION {
            return Ok(None);
        }
        let owner_len = owner_hdr.length as usize;
        if owner_len > self.app_size {
            return Ok(None);
        }

        Ok(Some(FirmwareBundle {
            offset,
            rom_ext_len,
            owner_len,
        }))
    }
}

pub struct FirmwareBundle {
    pub offset: usize,
    pub rom_ext_len: usize,
    pub owner_len: usize,
}
