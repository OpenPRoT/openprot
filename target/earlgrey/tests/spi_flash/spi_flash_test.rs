// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use pw_status::Error;
use spi_flash_test_codegen::handle;
use userspace::entry;

use earlgrey_platform::flash_mux::FlashMuxClient;
use earlgrey_util::EarlgreyFlashAddress;
use hal_flash::{Flash, FlashAddress};
use services_flash_client::FlashIpcClient;
use util_error::{ErrorCode, KERNEL_ERROR_INTERNAL};
use util_ipc::IpcHandle;
use util_panic as _;

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

    pw_log::info!("--- Verifying Default Inaccessible State at Boot (bitmap = 0) ---");
    match FlashIpcClient::new(IpcHandle::new(handle::SPI_GENERIC_FLASH_SERVICE)) {
        Err(e) if e == util_error::FLASH_GENERIC_INACCESSIBLE => {
            pw_log::info!(
                "[GenericFlash] Correctly rejected at boot with FLASH_GENERIC_INACCESSIBLE"
            );
        }
        _ => {
            pw_log::error!("[GenericFlash] Expected FLASH_GENERIC_INACCESSIBLE at boot");
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }
    match FlashIpcClient::new(IpcHandle::new(handle::SPI_FLASH0_SERVICE)) {
        Err(e) if e == util_error::FLASH_GENERIC_INACCESSIBLE => {
            pw_log::info!("[Flash0] Correctly rejected at boot with FLASH_GENERIC_INACCESSIBLE");
        }
        _ => {
            pw_log::error!("[Flash0] Expected FLASH_GENERIC_INACCESSIBLE at boot");
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }

    pw_log::info!("--- Simulating Platform Service Initial MUX Notification (Enable Flash 0) ---");
    let flash_mux = FlashMuxClient::new(IpcHandle::new(handle::SPI_FLASH_MUX_SERVICE));
    flash_mux.switch_mux_fin_notice(0x1)?;

    pw_log::info!("--- Testing Generic and Dedicated SPI Flash Channels ---");
    let mut generic_flash = FlashIpcClient::new(IpcHandle::new(handle::SPI_GENERIC_FLASH_SERVICE))?;
    let (total_size, page_size, _) = generic_flash.geometry()?;
    pw_log::info!(
        "Generic SPI Flash size: {} bytes, page size: {} bytes",
        total_size.get(),
        page_size.get()
    );

    // 1. Verify Generic SPI Flash channel on 1MB offset
    erase_program_test(
        &mut generic_flash,
        FlashAddress::new(0x0010_0000),
        "Generic_SpiFlash",
    )?;

    // 2. Verify Dedicated Flash 0 channel succeeds while Flash 1 channel is rejected
    pw_log::info!("Verifying Dedicated Flash 0 channel on 2MB offset...");
    let mut flash0_client = FlashIpcClient::new(IpcHandle::new(handle::SPI_FLASH0_SERVICE))?;
    erase_program_test(
        &mut flash0_client,
        FlashAddress::new(0x0020_0000),
        "Dedicated_Flash0",
    )?;

    pw_log::info!("Verifying Dedicated Flash 1 channel is rejected when Flash 0 is active...");
    match FlashIpcClient::new(IpcHandle::new(handle::SPI_FLASH1_SERVICE)) {
        Err(e) if e == util_error::FLASH_GENERIC_INACCESSIBLE => {
            pw_log::info!(
                "[Flash1] Client connection correctly rejected with FLASH_GENERIC_INACCESSIBLE"
            );
        }
        _ => {
            pw_log::error!(
                "[Flash1] Client connection failed to return FLASH_GENERIC_INACCESSIBLE"
            );
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }

    pw_log::info!("--- Testing SPI MUX Handshake & Corner Cases via Control Channel ---");
    let mut check_buf = [0u8; 32];
    pw_log::info!("[FlashMux] Sending switch_mux_notice (Quiesce)...");
    flash_mux.switch_mux_notice()?;

    // Corner Case 1: Inaccessible Error during Quiescence on all channels
    pw_log::info!(
        "[FlashMux] Corner Case 1: verifying read fail with FLASH_GENERIC_INACCESSIBLE during quiescence..."
    );
    match generic_flash.read(FlashAddress::new(0x0010_0000), &mut check_buf) {
        Err(e) if e == util_error::FLASH_GENERIC_INACCESSIBLE => {
            pw_log::info!("[GenericFlash] Read correctly rejected with FLASH_GENERIC_INACCESSIBLE");
        }
        _ => {
            pw_log::error!("[GenericFlash] Read failed to return FLASH_GENERIC_INACCESSIBLE");
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }
    match flash0_client.read(FlashAddress::new(0x0010_0000), &mut check_buf) {
        Err(e) if e == util_error::FLASH_GENERIC_INACCESSIBLE => {
            pw_log::info!(
                "[Flash0] Read correctly rejected with FLASH_GENERIC_INACCESSIBLE during quiesce"
            );
        }
        _ => {
            pw_log::error!("[Flash0] Read failed to return FLASH_GENERIC_INACCESSIBLE");
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }

    // Corner Case 2: Zero Bitmap in FLXE must return error
    pw_log::info!(
        "[FlashMux] Corner Case 2: verifying switch_mux_fin_notice(0x0) returns error..."
    );
    match flash_mux.switch_mux_fin_notice(0x0) {
        Err(e) if e == util_error::IPC_ERROR_BAD_REQ => {
            pw_log::info!(
                "[FlashMux] switch_mux_fin_notice(0x0) correctly rejected with IPC_ERROR_BAD_REQ"
            );
        }
        _ => {
            pw_log::error!(
                "[FlashMux] switch_mux_fin_notice(0x0) failed to return IPC_ERROR_BAD_REQ"
            );
            return Err(KERNEL_ERROR_INTERNAL);
        }
    }

    // TODO: When dual-flash target hardware with a physical SPI Host 1 (Flash 1) chip
    // is available: do NOT send switch_mux_fin_notice(0x2) in isolation. The test
    // must integrate PlatformService (or invoke SpiMuxHandler::switch_mux_to(SpiMuxRoute::HostCpu0Earlgrey1))
    // to physically drive the GPIO pins (assert SPI_RESET_N, drive SPI_MUX_CTRL = LOW,
    // and release SPI_RESET_N) as part of the complete PlatformServer::switch_mux
    // sequence before verifying erase/program operations on Flash 1.
    // Normal switch completion: enable Flash 0 (0x1)
    pw_log::info!(
        "[FlashMux] Sending switch_mux_fin_notice(0x1) (Re-init 4B mode & enable Flash 0)..."
    );
    flash_mux.switch_mux_fin_notice(0x1)?;

    // Corner Case 3: Verify High Address (>16MB) in 4-Byte Address Mode on Flash 0
    pw_log::info!("--- Verifying External SPI Flash 0 High Address (>16MB) in 4-Byte Mode ---");
    erase_program_test(
        &mut generic_flash,
        FlashAddress::new(0x0110_0000),
        "GenericFlash_HighAddr_4Byte",
    )?;

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
