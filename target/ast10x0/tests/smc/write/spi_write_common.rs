// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared destructive SPI flash test flow.

use ast10x0_peripherals::smc::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, SmcTopology, SpiReady,
    SpiTransaction, SpiUninit, TransferMode,
};
use ast10x0_peripherals::scu::SpiMonitorInstance;

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::dump_smc_read;

pub const FLASH_CONFIG: FlashConfig = FlashConfig {
    capacity_mb: 32,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
    spi_clock_mhz: 25,
};

const TEST_OFFSET: u32 = 0x10_0000;
const PAGE_LEN: usize = 256;
const SECTOR_LEN: usize = 4096;
const BACKUP_ADDR: usize = 0x41000;
const STATUS_WIP: u8 = 1;
const STATUS_MAX_POLLS: u32 = 1_000_000;

fn fill_test_pattern(out: &mut [u8; PAGE_LEN], salt: u8) {
    let mut i = 0usize;
    while i < out.len() {
        out[i] = (i as u8).wrapping_mul(17).wrapping_add(salt);
        i += 1;
    }
}

fn expect_erased(buf: &[u8]) -> Result<(), SmcError> {
    for &byte in buf {
        if byte != 0xff {
            return Err(SmcError::HardwareError);
        }
    }
    Ok(())
}

fn command_with_spim(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    cmd: &[u8],
    tx: &[u8],
    rx: &mut [u8],
) -> Result<(), SmcError> {
    SpiTransaction::transceive_user_with_spim(
        spi,
        spim,
        ChipSelect::Cs0,
        cmd,
        tx,
        rx,
        TransferMode::Mode111,
    )
}

fn read_with_spim(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
    buf: &mut [u8],
) -> Result<(), SmcError> {
    let n = SpiTransaction::read_with_spim(spi, spim, ChipSelect::Cs0, offset, buf)?;
    if n != buf.len() {
        return Err(SmcError::HardwareError);
    }
    Ok(())
}

fn write_enable(spi: &mut SpiReady, spim: SpiMonitorInstance) -> Result<(), SmcError> {
    command_with_spim(spi, spim, &[0x06], &[], &mut [])
}

fn wait_write_complete(spi: &mut SpiReady, spim: SpiMonitorInstance) -> Result<(), SmcError> {
    let mut polls = 0u32;
    while polls < STATUS_MAX_POLLS {
        let mut status = [0u8; 1];
        command_with_spim(spi, spim, &[0x05], &[], &mut status)?;
        if status[0] & STATUS_WIP == 0 {
            return Ok(());
        }
        polls += 1;
    }
    Err(SmcError::Timeout)
}

fn erase_sector(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
) -> Result<(), SmcError> {
    write_enable(spi, spim)?;
    let address = offset.to_be_bytes();
    let cmd = [0x21, address[0], address[1], address[2], address[3]];
    command_with_spim(spi, spim, &cmd, &[], &mut [])?;
    wait_write_complete(spi, spim)
}

fn program_page(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
    data: &[u8],
) -> Result<(), SmcError> {
    write_enable(spi, spim)?;
    let address = offset.to_be_bytes();
    let cmd = [0x12, address[0], address[1], address[2], address[3]];
    command_with_spim(spi, spim, &cmd, data, &mut [])?;
    wait_write_complete(spi, spim)
}

fn verify_data(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
    expected: &[u8],
) -> Result<(), SmcError> {
    let mut read_buf = [0u8; PAGE_LEN];
    let mut cursor = 0usize;
    while cursor < expected.len() {
        let len = core::cmp::min(PAGE_LEN, expected.len() - cursor);
        read_with_spim(
            spi,
            spim,
            offset + cursor as u32,
            &mut read_buf[..len],
        )?;
        if read_buf[..len] != expected[cursor..cursor + len] {
            return Err(SmcError::HardwareError);
        }
        cursor += len;
    }
    Ok(())
}

fn restore_sector(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    original: &[u8],
) -> Result<(), SmcError> {
    erase_sector(spi, spim, TEST_OFFSET)?;

    let mut cursor = 0usize;
    while cursor < original.len() {
        let len = core::cmp::min(PAGE_LEN, original.len() - cursor);
        program_page(
            spi,
            spim,
            TEST_OFFSET + cursor as u32,
            &original[cursor..cursor + len],
        )?;
        cursor += len;
    }

    verify_data(spi, spim, TEST_OFFSET, original)
}

fn destructive_test(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    salt: u8,
) -> Result<(), SmcError> {
    erase_sector(spi, spim, TEST_OFFSET)?;

    let mut read_buf = [0u8; PAGE_LEN];
    read_with_spim(spi, spim, TEST_OFFSET, &mut read_buf)?;
    expect_erased(&read_buf)?;
    dump_smc_read(&read_buf, PAGE_LEN as u32);

    let mut pattern = [0u8; PAGE_LEN];
    fill_test_pattern(&mut pattern, salt);
    program_page(spi, spim, TEST_OFFSET, &pattern)?;

    read_buf.fill(0);
    read_with_spim(spi, spim, TEST_OFFSET, &mut read_buf)?;
    if read_buf != pattern {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&read_buf, PAGE_LEN as u32);
    Ok(())
}

pub fn run_flash_test(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    salt: u8,
) -> Result<(), SmcError> {
    let mut jedec = [0u8; 3];
    command_with_spim(spi, spim, &[0x9f], &[], &mut jedec)?;
    pw_log::info!(
        "JEDEC ID: {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );
    if jedec[0] == 0xff {
        return Err(SmcError::HardwareError);
    }

    pw_log::info!("=== backup test sector ===");
    let original = unsafe { core::slice::from_raw_parts_mut(BACKUP_ADDR as *mut u8, SECTOR_LEN) };
    read_with_spim(spi, spim, TEST_OFFSET, original)?;

    pw_log::info!("=== erase/write/read test sector ===");
    let test_result = destructive_test(spi, spim, salt);

    pw_log::info!("=== restore test sector ===");
    let restore_result = restore_sector(spi, spim, original);

    test_result?;
    restore_result
}

pub fn new_spi(
    controller: SmcController,
    topology: SmcTopology,
) -> Result<SpiReady, SmcError> {
    let config = SmcConfig {
        controller_id: controller,
        cs0: Some(FLASH_CONFIG),
        cs1: None,
        dma_enabled: false,
        enable_interrupts: false,
        topology,
    };
    let spi = unsafe { SpiUninit::new(controller, config)? };
    let mut spi = spi.init()?;
    spi.spi_nor_read_init(ChipSelect::Cs0)?;
    Ok(spi)
}
