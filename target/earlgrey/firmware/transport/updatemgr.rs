// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_util::error::EG_ERROR_UPDATE_NOT_FOUND;
use hal_flash::Flash;
use pw_status::Error;
use services_flash_client::FlashIpcClient;
use updatemgr_codegen::handle;
use userspace::process_entry;
use userspace::time::{sleep_until, Clock, Duration, Instant, SystemClock};
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
    format = "Update found! Staging slot: {staging_slot:c}, ROM_EXT offset: 0x{rom_ext_offset:x}, Owner offset: 0x{owner_offset:x}"
)]
struct UpdateTargetMapped {
    staging_slot: u32,
    rom_ext_offset: u32,
    owner_offset: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "Update manager attempt failed: 0x{status:08x}. Retrying in 1s...")]
struct UpdateAttemptFailed {
    status: u32,
}

fn try_update(
    spi_flash_client: &mut FlashIpcClient,
    update: &earlgrey_fw_update::FwUpdate,
) -> Result<(), ErrorCode> {
    let mut reader = spi_flash_client.random_reader();
    let size = reader.size()?;
    util_zfmt::info!(SpiFlashDetected { size: size as u32 });

    let Some(_bundle) = update.scan_firmware_bundle(&mut reader)? else {
        return Err(EG_ERROR_UPDATE_NOT_FOUND);
    };

    util_zfmt::info!(UpdateTargetMapped {
        staging_slot: update.rom_ext.0,
        rom_ext_offset: update.rom_ext_start,
        owner_offset: update.app_start,
    });

    Ok(())
}

fn updatemgr_process() -> Result<(), ErrorCode> {
    let sysmgr_client =
        earlgrey_sysmgr_client::SysmgrClient::new(IpcHandle::new(handle::SYSMGR_UPDATER_CLIENT));

    let info = sysmgr_client.get_boot_info()?;
    let update = earlgrey_fw_update::FwUpdate::new(&info)?;

    let spi_flash_handle = IpcHandle::new(handle::SPI_FLASH_UPDATEMGR);
    let mut spi_flash_client = FlashIpcClient::new(spi_flash_handle)?;
    loop {
        match try_update(&mut spi_flash_client, &update) {
            Ok(()) => break,
            Err(e) => {
                util_zfmt::warn!(UpdateAttemptFailed { status: e.0.get() });
                let _ = sleep_until(SystemClock::now() + Duration::from_secs(1));
            }
        }
    }

    let _ = sleep_until(Instant::MAX);
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
