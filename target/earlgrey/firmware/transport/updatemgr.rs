// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_util::error::EG_ERROR_UPDATE_NOT_FOUND;
use earlgrey_util::EarlgreyFlashAddress;
use hal_flash::{Flash, FlashAddress};
use pw_status::Error;
use services_flash_client::FlashIpcClient;
use updatemgr_codegen::handle;
use userspace::process_entry;
use userspace::time::{sleep_until, Clock, Duration, SystemClock};
use util_error::{AsStatus, ErrorCode};
use util_io::RandomRead;
use util_ipc::IpcHandle;
use util_zfmt::messages::{ProcessExit, ProcessStart};
use zfmt::Zfmt;

#[derive(Zfmt)]
#[zfmt(format = "SPI Flash detected. Size: {size} bytes")]
struct SpiFlashDetected {
    size: u32,
}

#[derive(Zfmt)]
#[zfmt(
    format = "Update found! ROM_EXT staging: {rom_ext_staging_slot:c} (offset: 0x{rom_ext_offset:x}), Owner staging: {owner_staging_slot:c} (offset: 0x{owner_offset:x})"
)]
struct UpdateTargetMapped {
    rom_ext_staging_slot: u32,
    rom_ext_offset: u32,
    owner_staging_slot: u32,
    owner_offset: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "Update manager attempt failed: 0x{status:08x}. Retrying in 1s...")]
struct UpdateAttemptFailed {
    status: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "Flashing {region} partition: EFLASH offset 0x{start:x} ({len} bytes)")]
struct FlashingRegion {
    region: &'static str,
    start: u32,
    len: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "Successfully wrote {region} partition to EFLASH!")]
struct FlashWriteSuccess {
    region: &'static str,
}

#[derive(Zfmt)]
#[zfmt(format = "Failed to write {region} partition! Status: 0x{status:08x}")]
struct FlashWriteFailed {
    region: &'static str,
    status: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "Firmware update installation complete! Rebooting into the new slot...")]
struct UpdateComplete {}

fn flash_write_partition(
    flash_client: &mut FlashIpcClient,
    spi_flash: &mut impl RandomRead<Error = ErrorCode>,
    src_offset: usize,
    dest_offset: u32,
    len: usize,
) -> Result<(), ErrorCode> {
    // Get EFLASH geometry to find page size
    let (_, page_size, _) = flash_client.geometry()?;
    let page_len = page_size.get();

    let mut erased = 0;
    while erased < len {
        let erase_addr = dest_offset + erased as u32;
        flash_client.erase(FlashAddress::data(erase_addr), page_size)?;
        erased += page_len;
    }

    let mut page_buf = [0u8; 2048];
    let mut written = 0;
    while written < len {
        let chunk_len = core::cmp::min(len - written, page_len);
        let src_addr = src_offset + written;
        let dest_addr = dest_offset + written as u32;

        page_buf.fill(0);
        spi_flash.read(src_addr, &mut page_buf[..chunk_len])?;
        flash_client.program(FlashAddress::data(dest_addr), &page_buf)?;

        written += chunk_len;
    }

    Ok(())
}

fn try_update(
    flash_client: &mut FlashIpcClient,
    spi_flash_client: &mut FlashIpcClient,
    update: &earlgrey_fw_update::FwUpdate,
) -> Result<(), ErrorCode> {
    let mut reader = spi_flash_client.random_reader();
    let size = reader.size()?;
    util_zfmt::info!(SpiFlashDetected { size: size as u32 });

    let Some(bundle) = update.scan_firmware_bundle(&mut reader)? else {
        return Err(EG_ERROR_UPDATE_NOT_FOUND);
    };

    util_zfmt::info!(UpdateTargetMapped {
        rom_ext_staging_slot: update.rom_ext.0,
        rom_ext_offset: update.rom_ext_start,
        owner_staging_slot: update.app.0,
        owner_offset: update.app_start,
    });

    util_zfmt::info!(FlashingRegion {
        region: "ROM_EXT",
        start: update.rom_ext_start,
        len: bundle.rom_ext_len as u32,
    });
    flash_write_partition(
        flash_client,
        &mut reader,
        bundle.offset,
        update.rom_ext_start,
        bundle.rom_ext_len,
    )
    .map_err(|e| {
        util_zfmt::error!(FlashWriteFailed {
            region: "ROM_EXT",
            status: e.0.get(),
        });
        e
    })?;
    util_zfmt::info!(FlashWriteSuccess { region: "ROM_EXT" });

    util_zfmt::info!(FlashingRegion {
        region: "Owner",
        start: update.app_start,
        len: bundle.owner_len as u32,
    });
    flash_write_partition(
        flash_client,
        &mut reader,
        bundle.offset + update.rom_ext_size,
        update.app_start,
        bundle.owner_len,
    )
    .map_err(|e| {
        util_zfmt::error!(FlashWriteFailed {
            region: "Owner",
            status: e.0.get(),
        });
        e
    })?;
    util_zfmt::info!(FlashWriteSuccess { region: "Owner" });

    Ok(())
}

fn updatemgr_process() -> Result<(), ErrorCode> {
    let sysmgr_client =
        earlgrey_sysmgr_client::SysmgrClient::new(IpcHandle::new(handle::SYSMGR_UPDATER_CLIENT));

    let info = sysmgr_client.get_boot_info()?;
    let update = earlgrey_fw_update::FwUpdate::new(&info)?;

    let spi_flash_handle = IpcHandle::new(handle::SPI_FLASH_UPDATEMGR);
    let mut spi_flash_client = FlashIpcClient::new(spi_flash_handle)?;
    let flash_ipc_handle = IpcHandle::new(handle::FLASH_UPDATEMGR);
    let mut flash_client = FlashIpcClient::new(flash_ipc_handle)?;

    loop {
        match try_update(&mut flash_client, &mut spi_flash_client, &update) {
            Ok(()) => break,
            Err(e) => {
                util_zfmt::warn!(UpdateAttemptFailed { status: e.0.get() });
                let _ = sleep_until(SystemClock::now() + Duration::from_secs(1));
            }
        }
    }

    let policy = earlgrey_sysmgr_client::BootPolicy {
        pref_slot: update.app,
        next_slot: update.app,
    };
    sysmgr_client.set_boot_policy(policy)?;
    util_zfmt::info!(UpdateComplete {});

    let _ = sysmgr_client.request_reboot();

    Ok(())
}

#[process_entry("updatemgr")]
fn entry() -> Result<(), Error> {
    util_zfmt::info!(ProcessStart { name: "updatemgr" });
    let ret = updatemgr_process();
    util_zfmt::error!(ProcessExit {
        name: "updatemgr",
        status: ret.as_status()
    });
    Err(Error::Unknown)
}
