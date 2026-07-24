// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Smoke test for generic SPI Flash driver on Earlgrey.
//!
//! Initializes SPI Host, SpiFlash driver, wraps them in BlockingFlash,
//! and performs Erase -> Program -> Read -> Verify cycle.

#![no_std]
#![no_main]

use hal_flash::Flash;
use hal_flash_driver::FlashAddress;
use pw_status::{Error, Result};
use spi_flash::SpiFlash;
use spi_host::SpiHost0;
use userspace::entry;

fn run_test() -> Result<()> {
    // 1. Initialize SPI Host.
    // SAFETY: We have exclusive access to SPI_HOST0 in this test process.
    let mmio0 = unsafe { spi_host::RegisterBlock::new(SpiHost0::PTR) };
    let mut spi_host = unsafe { earlgrey_spi_host::SpiHost::new(mmio0) };
    spi_host
        .init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI0)
        .map_err(|_| Error::Internal)?;

    // 2. Create SpiFlash driver.
    let mut flash = SpiFlash::new(spi_host);

    // 3. Initialize SpiFlash driver.
    flash.init().map_err(|_| Error::Internal)?;

    // 4. Print geometry.
    let (size, page_size, erase_bitmap) = flash.geometry().map_err(|_| Error::Internal)?;
    pw_log::info!(
        "Flash geometry: size={} bytes, page_size={} bytes, erase_bitmap=0x{:x}",
        size,
        page_size.get(),
        erase_bitmap
    );

    // 5. Test Erase -> Program -> Read -> Verify
    // We use address 0x00100000 (1MB offset) which is aligned to 64KB block boundary.
    let test_addr = FlashAddress::new(0x0010_0000);

    // Erase 4KB sector.
    let erase_size = util_types::PowerOf2Usize::new(4096).unwrap();
    pw_log::info!("Erasing 4KB at offset {}...", test_addr);
    flash
        .erase(test_addr, erase_size)
        .map_err(|_| Error::Internal)?;
    pw_log::info!("Erase complete.");

    // Read back and verify it is erased (all bytes 0xFF).
    let mut read_buf = [0u8; 256];
    flash
        .read(test_addr, &mut read_buf)
        .map_err(|_| Error::Internal)?;
    for (i, &b) in read_buf.iter().enumerate() {
        if b != 0xFF {
            pw_log::error!(
                "Erase verification failed at offset +{}: expected 0xFF, got 0x{:02x}",
                i,
                b
            );
            return Err(Error::FailedPrecondition);
        }
    }
    pw_log::info!("Erase verified (all 0xFF).");

    // Program a page (256 bytes).
    let mut write_buf = [0u8; 256];
    for (i, b) in write_buf.iter_mut().enumerate() {
        *b = i as u8;
    }
    pw_log::info!("Programming 256 bytes at offset {}...", test_addr);
    flash
        .program(test_addr, &write_buf)
        .map_err(|_| Error::Internal)?;
    pw_log::info!("Program complete.");

    // Read back and verify program.
    read_buf.fill(0);
    flash
        .read(test_addr, &mut read_buf)
        .map_err(|_| Error::Internal)?;
    for (i, (&w, &r)) in write_buf.iter().zip(read_buf.iter()).enumerate() {
        if w != r {
            pw_log::error!(
                "Verification failed at offset +{}: wrote 0x{:02x}, read 0x{:02x}",
                i,
                w,
                r
            );
            return Err(Error::FailedPrecondition);
        }
    }
    pw_log::info!("Program verified successfully!");

    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("🔄 RUNNING SPI Flash Smoke Test");
    let ret = run_test();

    if ret.is_err() {
        pw_log::error!("❌ FAIL");
    } else {
        pw_log::info!("✅ PASS");
    }

    ret
}

util_panic::make_panic_handler!();
