// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Smoke test for Earlgrey SPI Host driver.
//!
//! Initializes the SPI Host, reads the JEDEC SFDP signature of the
//! external Flash, and verifies it matches the standard "SFDP" string.

#![no_std]
#![no_main]

use earlgrey_spi_host::{SpiConfig, SpiHost};
use embedded_hal::spi::SpiDevice;
use pw_status::{Error, Result};
use spi_host::{SpiHost0, SpiHost1};
use userspace::entry;

fn run_test() -> Result<()> {
    // 1. Initialize SPI Host (SPI_HOST0).
    // SAFETY: We have exclusive access to SPI_HOST0 in this test process.
    let spi_host0 = unsafe { SpiHost0::new() };
    let mut spi = SpiHost::new_spi_host0(spi_host0);
    spi.init(&SpiConfig::DEFAULT_SPI0).map_err(|_| {
        pw_log::error!("SPI init failed");
        Error::Internal
    })?;

    // 2. Read SFDP Signature (Command 0x5A, Address 0x000000, 8 Dummy Cycles -> 1 Dummy Byte).
    // The TX buffer contains: [Opcode, Addr[23:16], Addr[15:8], Addr[7:0], Dummy]
    let tx_buf = [0x5A, 0x00, 0x00, 0x00, 0x00];
    let mut rx_buf = [0u8; 4];

    let mut ops = [
        embedded_hal::spi::Operation::Write(&tx_buf),
        embedded_hal::spi::Operation::Read(&mut rx_buf),
    ];

    spi.transaction(&mut ops).map_err(|_| {
        pw_log::error!("SPI transaction 1 failed");
        Error::Internal
    })?;

    pw_log::info!(
        "SFDP signature read 1: {:02x} {:02x} {:02x} {:02x}",
        rx_buf[0],
        rx_buf[1],
        rx_buf[2],
        rx_buf[3]
    );

    // Read SFDP Signature again to verify FIFO is not corrupted by padded write remainders.
    let mut rx_buf2 = [0u8; 4];
    let mut ops2 = [
        embedded_hal::spi::Operation::Write(&tx_buf),
        embedded_hal::spi::Operation::Read(&mut rx_buf2),
    ];

    spi.transaction(&mut ops2).map_err(|_| {
        pw_log::error!("SPI transaction 2 failed");
        Error::Internal
    })?;

    pw_log::info!(
        "SFDP signature read 2: {:02x} {:02x} {:02x} {:02x}",
        rx_buf2[0],
        rx_buf2[1],
        rx_buf2[2],
        rx_buf2[3]
    );

    // 3. Verify SFDP Signature ("SFDP" -> [0x53, 0x46, 0x44, 0x50])
    let expected_sig = [0x53, 0x46, 0x44, 0x50]; // "SFDP" in ASCII (little-endian bytes)
    if rx_buf != expected_sig {
        pw_log::error!(
            "FAIL: Unexpected SFDP signature in read 1: [{:02x}, {:02x}, {:02x}, {:02x}] (expected [53, 46, 44, 50])",
            rx_buf[0],
            rx_buf[1],
            rx_buf[2],
            rx_buf[3]
        );
        return Err(Error::FailedPrecondition);
    }
    if rx_buf2 != expected_sig {
        pw_log::error!(
            "FAIL: Unexpected SFDP signature in read 2: [{:02x}, {:02x}, {:02x}, {:02x}] (expected [53, 46, 44, 50])",
            rx_buf2[0],
            rx_buf2[1],
            rx_buf2[2],
            rx_buf2[3]
        );
        return Err(Error::FailedPrecondition);
    }

    pw_log::info!("SFDP signature verified successfully!");

    // 4. Exercise SPI_HOST1 to verify multi-instance capability.
    // SAFETY: We have exclusive access to SPI_HOST1 in this test process.
    let spi_host1 = unsafe { SpiHost1::new() };
    let mut spi1 = SpiHost::new_spi_host1(spi_host1);
    spi1.init(&SpiConfig::DEFAULT_SPI1).map_err(|_| {
        pw_log::error!("SPI1 init failed");
        Error::Internal
    })?;

    // Perform a dummy transaction. MISO is likely floating or unmapped, so we expect
    // garbage bytes (usually 0x00 or 0xFF), but the transaction itself must succeed (no timeout/hang).
    let mut rx_buf_dummy = [0u8; 4];
    let mut ops_dummy = [
        embedded_hal::spi::Operation::Write(&tx_buf),
        embedded_hal::spi::Operation::Read(&mut rx_buf_dummy),
    ];

    spi1.transaction(&mut ops_dummy).map_err(|_| {
        pw_log::error!("SPI1 transaction failed");
        Error::Internal
    })?;

    pw_log::info!(
        "SPI1 dummy transaction returned: {:02x} {:02x} {:02x} {:02x}",
        rx_buf_dummy[0],
        rx_buf_dummy[1],
        rx_buf_dummy[2],
        rx_buf_dummy[3]
    );

    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("🔄 RUNNING SPI Host Smoke Test");
    let ret = run_test();

    if ret.is_err() {
        pw_log::error!("FAIL: Smoke test execution failed");
    } else {
        pw_log::info!("✅ PASS");
    }

    ret
}

util_panic::make_panic_handler!();
