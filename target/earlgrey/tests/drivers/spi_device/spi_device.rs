// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Smoke test for Earlgrey SPI Device driver.
//!
//! Initializes the SPI Device peripheral in Flash mode with JEDEC ID and SFDP interception,
//! and verifies hardware register configuration and SRAM buffer write operations.

#![no_std]
#![no_main]

use aligned::{Aligned, A4};
use earlgrey_spi_device::{JedecIdConfig, Mode, SpiDev, SpiDevCfg, CMD_INFO_SFDP};
use pw_status::{Error, Result};
use spi_device::SpiDevice;
use userspace::entry;
use util_panic as _;

fn run_test() -> Result<()> {
    // 1. Initialize SPI Device driver.
    // SAFETY: We have exclusive access to SPI_DEVICE in this test process.
    let mut dev = unsafe { SpiDev::new(spi_device::RegisterBlock::new(SpiDevice::PTR)) };

    let cfg = SpiDevCfg {
        jedec: JedecIdConfig::GOOGLE,
        mailbox: Some(0x7FF0000),
        mode: Mode::Flashmode,
        initial_address_mode_4b: true,
    };

    dev.init(&cfg).map_err(|_| {
        pw_log::error!("SPI Device init failed");
        Error::Internal
    })?;

    // 2. Verify register configuration.
    // SAFETY: We have exclusive access to SPI_DEVICE registers in this test process.
    let dev_raw = unsafe { SpiDevice::new() };
    let regs = dev_raw.regs();

    let ctrl = regs.control().read();
    if ctrl.mode() != spi_device::enums::Mode::Flashmode {
        pw_log::error!("FAIL: unexpected mode");
        return Err(Error::FailedPrecondition);
    }

    let jedec_cc = regs.jedec_cc().read();
    if jedec_cc.cc() != 0x7F || jedec_cc.num_cc() != 8 {
        pw_log::error!(
            "FAIL: unexpected jedec_cc: cc=0x{:x}, num_cc={}",
            jedec_cc.cc(),
            jedec_cc.num_cc()
        );
        return Err(Error::FailedPrecondition);
    }

    let jedec_id = regs.jedec_id().read();
    if jedec_id.mf() != 0x26 || jedec_id.id() != ((0x17 << 8) | 0x31) {
        pw_log::error!(
            "FAIL: unexpected jedec_id: mf=0x{:x}, id=0x{:x}",
            jedec_id.mf(),
            jedec_id.id()
        );
        return Err(Error::FailedPrecondition);
    }

    let intercept = regs.intercept_en().read();
    if !intercept.sfdp() || !intercept.jedec() || !intercept.status() || !intercept.mbx() {
        pw_log::error!("FAIL: intercept_en bits not set properly");
        return Err(Error::FailedPrecondition);
    }

    let cmd_sfdp = regs.cmd_info().at(CMD_INFO_SFDP.into()).read();
    if !cmd_sfdp.valid()
        || cmd_sfdp.opcode() != 0x5A
        || !cmd_sfdp.dummy_en()
        || cmd_sfdp.dummy_size() != 7
    {
        pw_log::error!("FAIL: cmd_info SFDP slot not configured properly");
        return Err(Error::FailedPrecondition);
    }

    // 3. Test SFDP table loading into egress buffer via set_sfdp.
    let mut test_sfdp_table: Aligned<A4, [u8; 256]> = Aligned([0u8; 256]);
    test_sfdp_table[0..4].copy_from_slice(b"SFDP"); // Signature 0x50444653
    test_sfdp_table[4] = 0x00; // Minor rev 0
    test_sfdp_table[5] = 0x01; // Major rev 1
    test_sfdp_table[6] = 0x00; // 1 parameter header (0-based)
    test_sfdp_table[7] = 0xFF; // Access protocol legacy

    dev.set_sfdp(&test_sfdp_table);

    // 4. Test mailbox write into egress buffer.
    let mbx_payload: Aligned<A4, [u8; 64]> = Aligned([0x5A; 64]);
    dev.write_to_mbx(&mbx_payload);

    pw_log::info!("SPI Device driver smoke test passed successfully!");
    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("🔄 RUNNING SPI Device Smoke Test");
    let ret = run_test();

    if ret.is_err() {
        pw_log::error!("FAIL: Smoke test execution failed");
    } else {
        pw_log::info!("✅ PASS");
    }

    ret
}
