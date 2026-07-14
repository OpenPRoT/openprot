// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use pw_status::Error;
use spi_flash_test_codegen::handle;
use userspace::entry;

use earlgrey_util::EarlgreyFlashAddress;
use hal_flash::{Flash, FlashAddress};
use services_flash_client::FlashIpcClient;
use util_error::{ErrorCode, KERNEL_ERROR_INTERNAL};
use util_ipc::IpcHandle;

fn erase_program_test(
    flash: &mut FlashIpcClient,
    addr: FlashAddress,
    flash_type: &str,
) -> Result<(), ErrorCode> {
    let (_total_size, page_size, _erasable_sizes_bitmap) = flash.geometry()?;
    pw_log::info!(
        "[{}] Erasing at offset 0x{:08x}...",
        flash_type,
        addr.offset()
    );
    flash.erase(addr, page_size)?;

    pw_log::info!("[{}] Reading after erase...", flash_type);
    let mut buf = [0u8; 32];
    flash.read(addr, &mut buf)?;
    util_misc::hexdump(&buf);
    for &byte in buf.iter() {
        if byte != 0xFF {
            pw_log::error!(
                "[{}] Erase check failed: byte is 0x{:02x}, expected 0xFF",
                flash_type,
                byte
            );
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }

    let payload = b"Dual Flash IPC Test Payload!!!  "; // 32 bytes (aligned)
    pw_log::info!(
        "[{}] Programming 32 bytes at offset 0x{:08x}...",
        flash_type,
        addr.offset()
    );
    flash.program(addr, payload)?;

    pw_log::info!("[{}] Reading back program results...", flash_type);
    flash.read(addr, &mut buf)?;
    util_misc::hexdump(&buf);

    if buf != *payload {
        pw_log::error!("[{}] Verify failed: content mismatch", flash_type);
        return Err(KERNEL_ERROR_INTERNAL);
    }
    pw_log::info!("[{}] Program verified successfully!", flash_type);
    Ok(())
}

fn flash_test() -> Result<(), ErrorCode> {
    pw_log::info!("--- Testing Internal EFlash ---");
    let mut eflash = FlashIpcClient::new(IpcHandle::new(handle::EFLASH_SERVICE))?;
    let (total_size, page_size, _) = eflash.geometry()?;
    pw_log::info!(
        "EFlash size: {} bytes, page size: {} bytes",
        total_size.get(),
        page_size.get()
    );
    // Test on Slot B area (offset 0x90000)
    erase_program_test(&mut eflash, FlashAddress::data(0x0009_0000), "EFlash")?;

    pw_log::info!("--- Testing External SPI Flash ---");
    let mut spi_flash = FlashIpcClient::new(IpcHandle::new(handle::FLASH_SERVICE))?;
    let (total_size, page_size, _) = spi_flash.geometry()?;
    pw_log::info!(
        "SPI Flash size: {} bytes, page size: {} bytes",
        total_size.get(),
        page_size.get()
    );
    // Test on 1MB offset
    erase_program_test(&mut spi_flash, FlashAddress::new(0x0010_0000), "SpiFlash")?;

    Ok(())
}

#[entry]
fn entry() -> Result<(), Error> {
    pw_log::info!("🔄 DUAL FLASH TEST CLIENT START");
    let ret = flash_test();

    let ret = match ret {
        Ok(()) => {
            pw_log::info!("✅ DUAL FLASH TEST CLIENT PASS");
            Ok(())
        }
        Err(e) => {
            pw_log::error!("❌ DUAL FLASH TEST CLIENT FAIL: {:08x}", u32::from(e));
            Err(Error::Unknown)
        }
    };

    ret
}

util_panic::make_panic_handler!();
