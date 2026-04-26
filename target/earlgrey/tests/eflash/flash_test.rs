// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use flash_test_codegen::handle;
use pw_status::Error;
use userspace::{entry, syscall};

use earlgrey_util::PersoCertificate;
use hal_flash::{Flash, FlashAddress};
use services_flash_client::FlashIpcClient;
use util_error::ErrorCode;
use util_ipc::IpcChannel;

fn get_manifest(flash: &mut FlashIpcClient) -> Result<(), ErrorCode> {
    let mut buf = [0u8; 1024];
    flash.read(FlashAddress::data(0), &mut buf)?;
    pw_log::info!("ROM_EXT manifest header:");
    util_console::hexdump::hexdump(&buf);
    Ok(())
}

fn get_certificates(flash: &mut FlashIpcClient) -> Result<(), ErrorCode> {
    const BEGIN_CERT: &'static str = "-----BEGIN CERTIFICATE-----";
    const END_CERT: &'static str = "-----END CERTIFICATE-----";

    let mut buf = [0u8; 1024];
    let mut output = [0u8; 1200];

    pw_log::info!("Reading UDS cert");
    // Read out the UDS and print it if it exists.
    // The UDS (factory) cert is located in bank=0, page=9.
    if flash.read(FlashAddress::info(0, 9, 0), &mut buf).is_ok() {
        let cert = PersoCertificate::from_bytes(&buf);
        if let Ok((uds, _rest)) = cert {
            pw_log::info!(
                "Certificate: {}\n{}\n{}\n{}",
                uds.name,
                BEGIN_CERT as &str,
                pw_base64::encode_str(uds.certificate, &mut output)
                    .map_err(ErrorCode::kernel_error)? as &str,
                END_CERT as &str,
            );
        }
    }

    pw_log::info!("Reading CDI certs");
    // Read out the CDI certificates and print them.
    // The CDI (dice) certs are located in bank=1, page=9.
    let mut offset = 0usize;
    loop {
        let sz = core::cmp::min(2048 - offset, buf.len());
        flash.read(FlashAddress::info(1, 9, offset as u32), &mut buf[..sz])?;
        match PersoCertificate::from_bytes(&buf) {
            Ok((cdi, _)) => {
                pw_log::info!(
                    "Certificate: {}\n{}\n{}\n{}",
                    cdi.name,
                    BEGIN_CERT as &str,
                    pw_base64::encode_str(cdi.certificate, &mut output)
                        .map_err(ErrorCode::kernel_error)? as &str,
                    END_CERT as &str,
                );
                offset += (cdi.obj_size + 7) & !7;
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn erase_program_test(flash: &mut FlashIpcClient, addr: FlashAddress) -> Result<(), ErrorCode> {
    pw_log::info!("Erasing {:08x}", addr.offset());
    flash.erase_page(addr)?;
    pw_log::info!("Reading {:08x}", addr.offset());
    let mut buf = [0u8; 32];
    flash.read(addr, &mut buf)?;
    util_console::hexdump::hexdump(&buf);

    pw_log::info!("Programming {:08x}", addr.offset());
    flash.program(addr, b"This is a test.")?;

    pw_log::info!("Reading {:08x}", addr.offset());
    flash.read(addr, &mut buf)?;
    util_console::hexdump::hexdump(&buf);

    Ok(())
}

fn flash_test() -> Result<(), ErrorCode> {
    let mut flash = FlashIpcClient::new(IpcChannel::new(handle::FLASH_SERVICE))?;

    pw_log::info!("Flash size: {}", flash.size().get() as usize);
    pw_log::info!("Flash page size: {}", flash.page_size().get() as usize);

    get_manifest(&mut flash)?;
    get_certificates(&mut flash)?;

    // We're currently executing in SlotA, so we should be able to access SlotB.
    erase_program_test(&mut flash, FlashAddress::data(0x90000))?;
    erase_program_test(&mut flash, FlashAddress::info(0, 5, 0))?;
    Ok(())
}

#[entry]
fn entry() -> ! {
    pw_log::info!("🔄 RUNNING");
    let ret = flash_test();

    // Log that an error occurred so that the app that caused the shutdown is logged.
    let ret = match ret {
        Ok(()) => {
            pw_log::info!("✅ PASSED");
            Ok(())
        }
        Err(e) => {
            pw_log::error!("❌ FAILED: {:08x}", u32::from(e) as u32);
            Err(Error::Unknown)
        }
    };

    // Since this is written as a test, shut down with the return status from `main()`.
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
