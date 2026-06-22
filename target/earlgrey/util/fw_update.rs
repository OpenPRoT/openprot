// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device Firmware Upgrade (DFU) tracker layout and state tracking helper.

#![no_std]

use earlgrey_sysmgr_client::BootInfo;
use earlgrey_util::manifest::{Manifest, ManifestExtHeader, MANIFEST_EXT_ID_OWNER_TRANSFER_BLOB};
use earlgrey_util::tags::{BootSlot, ManifestIdentifier};
use util_error::ErrorCode;
use util_io::RandomRead;
use zerocopy::FromBytes;

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
    /// Base address of target Application boot slot in flash.
    pub app_slot_start: u32,
    /// Start address of target Application partition in flash.
    pub app_start: u32,
    /// End address of target Application partition in flash.
    pub app_end: u32,
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
            .ok_or(earlgrey_util_error::EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let rom_ext_start = FwUpdate::addr(rom_ext);
        let app = info
            .app
            .boot_slot
            .opposite()
            .ok_or(earlgrey_util_error::EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let app_slot_start = FwUpdate::addr(app);
        let app_start = app_slot_start + info.rom_ext.size;

        Ok(FwUpdate {
            state: FwUpdateState::Idle,
            next_erase: 0,
            start_block: 0,
            rom_ext,
            rom_ext_start,
            rom_ext_end: rom_ext_start + info.rom_ext.size,
            app,
            app_slot_start,
            app_start,
            app_end: app_start + info.app.size,
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

    /// Scans the provided reader to find a compatible firmware bundle using a provided work buffer.
    pub fn scan_firmware_bundle(
        &mut self,
        flash: &mut impl RandomRead<Error = ErrorCode>,
        buf: &mut [u8],
    ) -> Result<Option<FirmwareBundle>, ErrorCode> {
        const STEP_SIZE: usize = 64 * 1024;
        let flash_size = flash.size()?;
        let manifest_size = core::mem::size_of::<Manifest>();

        let mut offset = 0;
        while offset + manifest_size <= flash_size {
            if let Some(bundle) = self.try_read_bundle_at(flash, offset, flash_size, buf)? {
                self.app_start = self.app_slot_start + bundle.app_target_addr as u32;
                return Ok(Some(bundle));
            }
            offset += STEP_SIZE;
        }

        Ok(None)
    }

    /// Attempts to parse a firmware update bundle at a specific candidate `offset` in external flash.
    ///
    /// The bundle can be either:
    /// 1. A standalone Application image (no ROM_EXT).
    /// 2. A ROM_EXT image followed by an Application image.
    fn try_read_bundle_at(
        &self,
        flash: &mut impl RandomRead<Error = ErrorCode>,
        offset: usize,
        flash_size: usize,
        buf: &mut [u8],
    ) -> Result<Option<FirmwareBundle>, ErrorCode> {
        const APP_SCAN_START_MIN: usize = 64 * 1024;
        const APP_SCAN_END_REL: usize = 128 * 1024;
        const APP_SCAN_STEP: usize = 8 * 1024;

        // Case 1: Standalone Application
        // Check if an Application manifest exists directly at `offset`.
        if let Some(app) = Self::try_read_app_manifest(flash, offset, flash_size, buf) {
            return Ok(Some(FirmwareBundle {
                rom_ext_src_offset: None,
                rom_ext_len: None,
                app_src_offset: offset,
                app_len: app.app_len,
                app_target_addr: app.app_target_addr,
                owner_block_offset: app.owner_block_offset,
            }));
        }

        // Case 2: ROM_EXT + Application
        // Check if a valid ROM_EXT manifest exists at `offset`.
        let Some(rom_ext_len) = Self::try_read_rom_ext_manifest(flash, offset, flash_size, buf)
        else {
            return Ok(None);
        };

        // Calculate Application scan start relative to `offset`.
        // The Application begins after ROM_EXT, at least at 64KiB (APP_SCAN_START_MIN) or max(rom_ext_len, 64KiB),
        // aligned up to the nearest 8KiB boundary (APP_SCAN_STEP).
        let start_rel_raw = APP_SCAN_START_MIN.max(rom_ext_len);
        let start_rel = (start_rel_raw + APP_SCAN_STEP - 1) & !(APP_SCAN_STEP - 1);

        // Scan candidate Application locations from `start_rel` up to 128KiB in 8KiB steps.
        for app_rel_offset in (start_rel..=APP_SCAN_END_REL).step_by(APP_SCAN_STEP) {
            let app_abs_offset = offset + app_rel_offset;
            if let Some(app) = Self::try_read_app_manifest(flash, app_abs_offset, flash_size, buf) {
                // Ensure the Application target address in internal flash does not overlap ROM_EXT.
                if app.app_target_addr >= rom_ext_len {
                    return Ok(Some(FirmwareBundle {
                        rom_ext_src_offset: Some(offset),
                        rom_ext_len: Some(rom_ext_len),
                        app_src_offset: app_abs_offset,
                        app_len: app.app_len,
                        app_target_addr: app.app_target_addr,
                        owner_block_offset: app.owner_block_offset,
                    }));
                }
            }
        }

        Ok(None)
    }

    fn try_read_rom_ext_manifest(
        flash: &mut impl RandomRead<Error = ErrorCode>,
        abs_offset: usize,
        flash_size: usize,
        buf: &mut [u8],
    ) -> Option<usize> {
        const MAX_ROM_EXT_SIZE: usize = 128 * 1024;
        let manifest_size = core::mem::size_of::<Manifest>();

        if abs_offset + manifest_size > flash_size || buf.len() < manifest_size {
            return None;
        }

        if flash.read(abs_offset, &mut buf[..manifest_size]).is_err() {
            return None;
        }

        let Ok((hdr, _)) = Manifest::ref_from_prefix(&buf[..manifest_size]) else {
            return None;
        };

        if hdr.check().is_err() || hdr.identifier != ManifestIdentifier::ROM_EXT {
            return None;
        }

        let rom_ext_len = hdr.length as usize;
        if rom_ext_len > MAX_ROM_EXT_SIZE {
            return None;
        }

        Some(rom_ext_len)
    }

    fn try_read_app_manifest(
        flash: &mut impl RandomRead<Error = ErrorCode>,
        abs_offset: usize,
        flash_size: usize,
        buf: &mut [u8],
    ) -> Option<AppManifestInfo> {
        const MAX_SLOT_SIZE: usize = 512 * 1024;
        const MAX_ROM_EXT_SIZE: usize = 128 * 1024;
        let manifest_size = core::mem::size_of::<Manifest>();

        if abs_offset + manifest_size > flash_size || buf.len() < manifest_size {
            return None;
        }

        if flash.read(abs_offset, &mut buf[..manifest_size]).is_err() {
            return None;
        }

        let Ok((hdr, _)) = Manifest::ref_from_prefix(&buf[..manifest_size]) else {
            return None;
        };

        if hdr.check().is_err() || hdr.identifier != ManifestIdentifier::APPLICATION {
            return None;
        }

        let app_len = hdr.length as usize;
        let raw_addr = hdr.manifest_base_address;
        let app_target_addr = if raw_addr == 0xa5a5a5a5 {
            64 * 1024
        } else if raw_addr >= 0x100000 {
            (raw_addr & 0xfffff) as usize
        } else {
            raw_addr as usize
        };

        if app_target_addr > MAX_ROM_EXT_SIZE || app_target_addr + app_len > MAX_SLOT_SIZE {
            return None;
        }

        let owner_block_offset = Self::find_owner_block_offset(hdr, abs_offset);

        Some(AppManifestInfo {
            app_len,
            app_target_addr,
            owner_block_offset,
        })
    }

    fn find_owner_block_offset(hdr: &Manifest, base_offset: usize) -> Option<usize> {
        for entry in &hdr.extensions.entries {
            if entry.identifier == MANIFEST_EXT_ID_OWNER_TRANSFER_BLOB {
                let ext_offset = entry.offset as usize;
                if ext_offset > 0 {
                    return Some(
                        base_offset + ext_offset + core::mem::size_of::<ManifestExtHeader>(),
                    );
                }
            }
        }
        None
    }
}

struct AppManifestInfo {
    app_len: usize,
    app_target_addr: usize,
    owner_block_offset: Option<usize>,
}

pub struct FirmwareBundle {
    pub rom_ext_src_offset: Option<usize>,
    pub rom_ext_len: Option<usize>,
    pub app_src_offset: usize,
    pub app_len: usize,
    pub app_target_addr: usize,
    pub owner_block_offset: Option<usize>,
}
