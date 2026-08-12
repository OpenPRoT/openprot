// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-side test harness for SPI Device E2E verification.
//!
//! Uses opentitanlib to query the Earlgrey SPI Device over the SPI bus:
//! 1. Reads and verifies JEDEC ID (Opcode 0x9F) matching Google continuation codes and ID.
//! 2. Reads and verifies SFDP Table (Opcode 0x5A) matching JESD216 signature and headers.

use anyhow::{ensure, Context, Result};
use clap::Parser;
use std::time::Duration;

use opentitanlib::io::spi::Transfer;
use opentitanlib::spiflash::SpiFlash;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;

#[derive(Debug, Parser)]
struct Opts {
    #[command(flatten)]
    init: InitializeTest,

    /// SPI interface name.
    #[arg(long, default_value = "BOOTSTRAP")]
    spi: String,

    /// Console receive timeout.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "180s")]
    timeout: Duration,
}

fn read_sfdp(spi: &dyn opentitanlib::io::spi::Target, offset: u32) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; 256];
    spi.run_transaction(&mut [
        // READ_SFDP (0x5A) always takes a 3-byte address followed by 1 dummy byte.
        Transfer::Write(&[
            SpiFlash::READ_SFDP,
            (offset >> 16) as u8,
            (offset >> 8) as u8,
            offset as u8,
            0x00, // Dummy byte
        ]),
        Transfer::Read(&mut buf),
    ])?;
    Ok(buf)
}

fn test_jedec_id(spi: &dyn opentitanlib::io::spi::Target) -> Result<()> {
    log::info!("Testing JEDEC ID readout (Opcode 0x9F)...");
    let jedec = SpiFlash::read_jedec_id(spi, 11)?;
    log::info!("Read JEDEC ID bytes: {:02x?}", jedec);

    ensure!(
        jedec.len() >= 11,
        "JEDEC ID response too short: expected 11 bytes, got {}",
        jedec.len()
    );

    // Verify 8 continuation codes (0x7F)
    for i in 0..8 {
        ensure!(
            jedec[i] == 0x7F,
            "Continuation code mismatch at index {}: expected 0x7F, got 0x{:02x}",
            i,
            jedec[i]
        );
    }

    // Verify Google Manufacturer ID (0x26)
    ensure!(
        jedec[8] == 0x26,
        "Manufacturer ID mismatch: expected 0x26 (Google), got 0x{:02x}",
        jedec[8]
    );

    // Verify Device ID (0x31, 0x17)
    ensure!(
        jedec[9] == 0x31 && jedec[10] == 0x17,
        "Device ID mismatch: expected [0x31, 0x17], got [0x{:02x}, 0x{:02x}]",
        jedec[9],
        jedec[10]
    );

    log::info!("✅ JEDEC ID verified successfully: 8x 0x7F, Manf 0x26, Dev 0x1731");
    Ok(())
}

fn test_sfdp_table(spi: &dyn opentitanlib::io::spi::Target) -> Result<()> {
    log::info!("Testing SFDP table readout (Opcode 0x5A)...");
    let sfdp = read_sfdp(spi, 0)?;

    // 1. Verify SFDP Header: Signature "SFDP" (0x50444653)
    let sig = &sfdp[0..4];
    ensure!(
        sig == b"SFDP",
        "SFDP signature mismatch: expected b\"SFDP\", got {:?}",
        sig
    );

    let minor_rev = sfdp[4];
    let major_rev = sfdp[5];
    let num_ph = sfdp[6];
    let access_protocol = sfdp[7];

    log::info!(
        "SFDP Header: Major {}, Minor {}, Param Headers {}, Access Protocol 0x{:02x}",
        major_rev,
        minor_rev,
        num_ph + 1,
        access_protocol
    );

    ensure!(major_rev >= 1, "Invalid SFDP major revision: {}", major_rev);

    // 2. Verify Parameter Header 0 (Basic Flash Parameters)
    let param_id_lsb = sfdp[8];
    let _param_minor = sfdp[9];
    let _param_major = sfdp[10];
    let param_len_dwords = sfdp[11];
    let param_ptr = sfdp[12] as u32 | ((sfdp[13] as u32) << 8) | ((sfdp[14] as u32) << 16);
    let param_id_msb = sfdp[15];

    ensure!(
        param_id_lsb == 0x00 && param_id_msb == 0xFF,
        "Parameter ID mismatch: expected 0xFF00, got 0x{:02x}{:02x}",
        param_id_msb,
        param_id_lsb
    );
    ensure!(
        param_ptr == 16,
        "Parameter table pointer mismatch: expected 16, got {}",
        param_ptr
    );
    ensure!(
        param_len_dwords >= 9,
        "Parameter table length too short: {} DWORDs",
        param_len_dwords
    );

    log::info!("✅ SFDP Table header & parameter headers verified successfully");
    Ok(())
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.init.init_logging();

    let transport = opts.init.init_target()?;
    let uart = transport.uart("console")?;

    log::info!("Waiting for target firmware message 'spidev: initialized SPI device in Flash mode with SFDP'...");
    UartConsole::wait_for(
        &*uart,
        r"spidev: initialized SPI device in Flash mode with SFDP",
        opts.timeout,
    )
    .context("Timeout waiting for SPI Device HWE firmware to boot")?;
    log::info!("Target firmware is ready.");

    let spi = transport.spi(&opts.spi)?;

    test_jedec_id(&*spi)?;
    test_sfdp_table(&*spi)?;

    log::info!("🎉 All SPI Device E2E tests passed successfully!");
    Ok(())
}
