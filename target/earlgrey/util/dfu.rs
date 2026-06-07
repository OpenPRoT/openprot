// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device Firmware Upgrade (DFU) handler for Earlgrey.
//!
//! This module implements the USB DFU protocol for Earlgrey, supporting firmware
//! updates (ROM_EXT and Application) and reading device certificates (UDS, CDI0, CDI1).

#![no_std]

use earlgrey_sysmgr_client::{BootInfo, SysmgrClient};
use earlgrey_util::tags::BootSlot;
use earlgrey_util::tags::ManifestIdentifier;
use earlgrey_util::EarlgreyFlashAddress;
use earlgrey_util::PersoCertificate;
use hal_flash::{Flash, FlashAddress};
use services_flash_client::FlashIpcClient;
use util_error::ErrorCode;
use util_ipc::IpcChannel;
use zerocopy::FromBytes;

use protocol_usb_dfu::{DfuHandler, DfuStatus};
use zfmt::Zfmt;

#[derive(Zfmt)]
#[zfmt(format = "Flashing {region} region at {start:x}-{end:x}")]
struct FlashingRegion {
    region: &'static str,
    start: u32,
    end: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "Unknown manifest ID: {id:08x}")]
struct UnknownManifestId {
    id: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "{dir}: alt={alt}, block={block}, len={len}")]
struct DfuTransfer {
    dir: &'static str,
    alt: u8,
    block: u16,
    len: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "DFU Manifestation")]
struct DfuManifest;

#[derive(Zfmt)]
#[zfmt(format = "DFU Abort")]
struct DfuAbort;

/// USB string descriptor for the Firmware DFU interface (Alt 0).
pub const DFU_FIRMWARE: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("Firmware").as_ref();
/// USB string descriptor for the UDS Certificate DFU interface (Alt 1).
pub const DFU_UDS_CERT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("UDS Certificate").as_ref();
/// USB string descriptor for the CDI0 Certificate DFU interface (Alt 2).
pub const DFU_CDI0_CERT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDI0 Certificate").as_ref();
/// USB string descriptor for the CDI1 Certificate DFU interface (Alt 3).
pub const DFU_CDI1_CERT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDI1 Certificate").as_ref();

/// Retrieves a certificate from the info partition in flash.
///
/// # Arguments
///
/// * `flash` - The flash IPC client used to read from flash.
/// * `n` - The certificate index: 0 for UDS, 1 for CDI0, 2 for CDI1.
/// * `data` - The buffer to write the certificate into.
///
/// # Returns
///
/// The size of the certificate in bytes, or a DFU error status.
fn get_certificate(flash: &mut FlashIpcClient, n: u8, data: &mut [u8]) -> Result<usize, DfuStatus> {
    util_zfmt::debug!("Reading certificate {n}");
    let (partition, mut n) = match n {
        0 => (0, 0), // The UDS (dice) cert is located in bank=0, page=9.
        1 => (1, 0), // The CDI (dice) certs are located in bank=1, page=9.
        2 => (1, 1), // CDI1 is the second cert in bank=1, page=9.
        _ => return Err(DfuStatus::ErrFile),
    };
    let mut offset = 0usize;
    let mut buf = [0u8; 1024];
    loop {
        let sz = core::cmp::min(2048 - offset, buf.len());
        flash
            .read(
                FlashAddress::info(partition, 9, offset as u32),
                &mut buf[..sz],
            )
            .map_err(|_| DfuStatus::ErrUnknown)?;
        match PersoCertificate::from_bytes(&buf) {
            Ok((cert, _)) => {
                if n == 0 {
                    let len = cert.certificate.len();
                    let l = len as u32;
                    util_zfmt::debug!("Found cert: {l} bytes");
                    let dest = data.get_mut(..len).ok_or(DfuStatus::ErrUnknown)?;
                    dest.copy_from_slice(cert.certificate);
                    return Ok(len);
                }
                offset += (cert.obj_size + 7) & !7;
                n -= 1;
            }
            Err(_) => break,
        }
    }
    Err(DfuStatus::ErrUnknown)
}

/// State of the firmware update process.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FwUpdateState {
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
struct FwUpdate {
    /// Current state of the update process.
    state: FwUpdateState,
    /// Next expected block number that triggers a partition erase.
    next_erase: u32,
    /// The block number where the current image (ROM_EXT or App) download started.
    start_block: u32,
    /// Target boot slot for ROM_EXT.
    _rom_ext: BootSlot,
    /// Start address of target ROM_EXT partition in flash.
    rom_ext_start: u32,
    /// End address of target ROM_EXT partition in flash.
    rom_ext_end: u32,
    /// Target boot slot for Application.
    app: BootSlot,
    /// Start address of target Application partition in flash.
    app_start: u32,
    /// End address of target Application partition in flash.
    app_end: u32,
}

impl FwUpdate {
    /// Creates a new `FwUpdate` tracker.
    ///
    /// It queries the current boot info to determine the active slots,
    /// and targets the *opposite* (inactive) slots for the update.
    fn new(info: &BootInfo) -> Result<Self, ErrorCode> {
        let rom_ext = info
            .rom_ext
            .boot_slot
            .opposite()
            .ok_or(earlgrey_util::error::EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let rom_ext_start = FwUpdate::addr(rom_ext);
        let app = info
            .app
            .boot_slot
            .opposite()
            .ok_or(earlgrey_util::error::EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let app_start = FwUpdate::addr(app) + info.rom_ext.size;

        Ok(FwUpdate {
            state: FwUpdateState::Idle,
            next_erase: 0,
            start_block: 0,
            _rom_ext: rom_ext,
            rom_ext_start,
            rom_ext_end: rom_ext_start + info.rom_ext.size,
            app,
            app_start,
            app_end: app_start + info.app.size,
        })
    }

    /// Returns the physical flash address offset for a given boot slot.
    fn addr(slot: BootSlot) -> u32 {
        match slot {
            BootSlot::SlotA => 0,
            BootSlot::SlotB => 0x80000,
            _ => unreachable!(),
        }
    }
}

/// DFU handler for Earlgrey, managing firmware updates and certificate uploads.
pub struct EarlgreyDfuHandler<IPC: IpcChannel> {
    flash: FlashIpcClient,
    sysmgr: SysmgrClient<IPC>,
    update: FwUpdate,
}

impl<IPC: IpcChannel> EarlgreyDfuHandler<IPC> {
    /// Creates a new DFU handler.
    pub fn new(
        flash: FlashIpcClient,
        sysmgr: SysmgrClient<IPC>,
        info: &BootInfo,
    ) -> Result<Self, ErrorCode> {
        Ok(EarlgreyDfuHandler {
            flash,
            sysmgr,
            update: FwUpdate::new(info)?,
        })
    }

    /// Erases the flash partition from `start` to `end` address (exclusive).
    ///
    /// The erase is performed page-by-page.
    fn flash_erase(&mut self, mut start: u32, end: u32) -> Result<(), ErrorCode> {
        let (_, page_size, _) = self.flash.geometry()?;
        while start < end {
            self.flash.erase(FlashAddress::data(start), page_size)?;
            start += page_size.get() as u32;
        }
        Ok(())
    }

    /// Handles writing a block of firmware to flash.
    ///
    /// This function handles:
    /// 1. Detecting the type of image (ROM_EXT or Application) from the manifest in the first block.
    /// 2. Erasing the target partition upon receiving the first block.
    /// 3. Programming subsequent blocks into the target partition.
    /// 4. Transitioning the state to `Done` when a short block (less than 2048 bytes) is received.
    fn flash_fw_block(&mut self, block_num: u32, data: &[u8]) -> Result<(), DfuStatus> {
        if block_num == self.update.next_erase {
            // TODO: this is the offset of the ManifestIdentifier.  We should have
            // a full zerocopy Manifest struct so we can check on other fields.
            let id_bytes = data.get(0x334..0x338).ok_or(DfuStatus::ErrFirmware)?;
            let id = ManifestIdentifier::read_from_bytes(id_bytes)
                .map_err(|_| DfuStatus::ErrFirmware)?;
            match id {
                ManifestIdentifier::ROM_EXT => {
                    util_zfmt::info!(FlashingRegion {
                        region: "ROM_EXT",
                        start: self.update.rom_ext_start,
                        end: self.update.rom_ext_end,
                    });
                    self.flash_erase(self.update.rom_ext_start, self.update.rom_ext_end)
                        .map_err(|_| DfuStatus::ErrErase)?;
                    self.update.state = FwUpdateState::RomExt;
                    self.update.next_erase =
                        (self.update.rom_ext_end - self.update.rom_ext_start) / 2048;
                    self.update.start_block = block_num;
                }
                ManifestIdentifier::APPLICATION => {
                    util_zfmt::info!(FlashingRegion {
                        region: "Application",
                        start: self.update.app_start,
                        end: self.update.app_end,
                    });
                    self.flash_erase(self.update.app_start, self.update.app_end)
                        .map_err(|_| DfuStatus::ErrErase)?;
                    self.update.state = FwUpdateState::Application;
                    self.update.start_block = block_num;
                }
                _ => {
                    util_zfmt::error!(UnknownManifestId { id: id.0 as u32 });
                    return Err(DfuStatus::ErrUnknown);
                }
            }
        }

        let block = block_num - self.update.start_block;
        match self.update.state {
            FwUpdateState::RomExt => {
                let address = self.update.rom_ext_start + block * 2048;
                if address > self.update.rom_ext_end {
                    return Err(DfuStatus::ErrAddress);
                }
                self.flash
                    .program(FlashAddress::data(address), data)
                    .map_err(|_| DfuStatus::ErrProg)?;
            }
            FwUpdateState::Application => {
                let address = self.update.app_end + block * 2048;
                if address > self.update.rom_ext_end {
                    return Err(DfuStatus::ErrAddress);
                }
                self.flash
                    .program(FlashAddress::data(address), data)
                    .map_err(|_| DfuStatus::ErrProg)?;
            }
            _ => {
                return Err(DfuStatus::ErrUnknown);
            }
        }

        if data.len() < 2048 {
            self.update.state = FwUpdateState::Done;
        }
        Ok(())
    }
}

impl<IPC: IpcChannel> DfuHandler for EarlgreyDfuHandler<IPC> {
    /// Handles a DFU download (DNLOAD) request.
    ///
    /// Accepts firmware blocks on Alt setting 0.
    fn dnload(&mut self, alt: u8, block_num: u16, data: &[u8]) -> Result<(), DfuStatus> {
        util_zfmt::info!(DfuTransfer {
            dir: "DNLOAD",
            alt,
            block: block_num,
            len: data.len() as u32,
        });
        if alt == 0 {
            self.flash_fw_block(block_num as u32, data)
        } else {
            Err(DfuStatus::ErrFile)
        }
    }

    /// Handles a DFU upload (UPLOAD) request.
    ///
    /// Returns device certificates on Alt settings 1, 2, and 3.
    fn upload(&mut self, alt: u8, block_num: u16, data: &mut [u8]) -> Result<usize, DfuStatus> {
        util_zfmt::info!(DfuTransfer {
            dir: "UPLOAD",
            alt,
            block: block_num,
            len: data.len() as u32,
        });
        match alt {
            1 | 2 | 3 => get_certificate(&mut self.flash, alt - 1, data),
            _ => Err(DfuStatus::ErrFile),
        }
    }

    /// Handles DFU manifestation.
    ///
    /// If the firmware update succeeded, updates the boot policy to prefer the new
    /// slot and requests a reboot.
    fn manifest(&mut self) -> Result<(), DfuStatus> {
        util_zfmt::info!(DfuManifest);
        if self.update.state == FwUpdateState::Done {
            // TODO: check for errors.
            let _ = self
                .sysmgr
                .set_boot_policy(earlgrey_sysmgr_client::BootPolicy {
                    pref_slot: self.update.app,
                    next_slot: BootSlot::Unspecified,
                });
            let _ = self.sysmgr.request_reboot();
        }
        Ok(())
    }

    /// Handles DFU abort.
    fn abort(&mut self) {
        util_zfmt::info!(DfuAbort);
    }
}
