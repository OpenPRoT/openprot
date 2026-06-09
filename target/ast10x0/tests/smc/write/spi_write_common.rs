// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared destructive SPI flash test flow.

use core::cell::UnsafeCell;

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
const STATUS_WIP: u8 = 1;
const STATUS_WEL: u8 = 1 << 1;
const STATUS_MAX_POLLS: u32 = 1_000_000;
const STATUS_CLEAR_SAMPLES: u8 = 3;
const STATUS_POLL_DELAY: u32 = 64;
const DMA_BUFFER: u32 = 0x41500;

#[repr(align(256))]
struct BackupBuffer(UnsafeCell<[u8; SECTOR_LEN]>);

#[repr(align(256))]
struct PageBuffer([u8; PAGE_LEN]);

// The test runs synchronously on one core and borrows this buffer only for the
// duration of run_flash_test().
unsafe impl Sync for BackupBuffer {}

static BACKUP_BUFFER: BackupBuffer = BackupBuffer(UnsafeCell::new([0; SECTOR_LEN]));

fn fill_test_pattern(out: &mut [u8; PAGE_LEN], salt: u8) {
    let mut i = 0usize;
    while i < out.len() {
        out[i] = (i as u8).wrapping_mul(17).wrapping_add(salt);
        i += 1;
    }
}

fn expect_erased(buf: &[u8]) -> Result<(), SmcError> {
    for (index, &byte) in buf.iter().enumerate() {
        if byte != 0xff {
            pw_log::info!(
                "erase verify failed at 0x{:08x}: expected ff, got {:02x}",
                (TEST_OFFSET as usize + index) as u32,
                byte as u32
            );
            return Err(SmcError::HardwareError);
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn is_erased(buf: &[u8]) -> bool {
    buf.iter().all(|&byte| byte == 0xff)
}

fn command_with_spim(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    cmd: &[u8],
    tx: &[u8],
    rx: &mut [u8],
    mode: TransferMode,
) -> Result<(), SmcError> {
    SpiTransaction::transceive_user_with_spim(
        spi,
        spim,
        ChipSelect::Cs0,
        cmd,
        tx,
        rx,
        mode,
    )
}

fn dma_read_with_spim(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
    buf: &mut [u8],
) -> Result<(), SmcError> {
    let mut dma_txn = SpiTransaction::dma_read_with_spim(
        spi,
        spim,
        ChipSelect::Cs0,
        offset,
        buf.as_mut_ptr() as usize,
        u32::try_from(buf.len()).map_err(|_| SmcError::InvalidCapacity)?,
    )?;

    loop {
        match dma_txn.poll_dma_completion() {
            core::task::Poll::Pending => core::hint::spin_loop(),
            core::task::Poll::Ready(result) => return result,
        }
    }
}

fn read_status(spi: &mut SpiReady, spim: SpiMonitorInstance) -> Result<u8, SmcError> {
    let mut status = [0u8; 1];
    command_with_spim(
        spi,
        spim,
        &[0x05],
        &[],
        &mut status,
        TransferMode::Mode111,
    )?;
    Ok(status[0])
}

fn write_enable(spi: &mut SpiReady, spim: SpiMonitorInstance) -> Result<(), SmcError> {
    command_with_spim(
        spi,
        spim,
        &[0x06],
        &[],
        &mut [],
        TransferMode::Mode111,
    )?;

    let status = read_status(spi, spim)?;
    if status & STATUS_WEL == 0 {
        pw_log::info!("write enable failed: SR1=0x{:02x}", status as u32);
        return Err(SmcError::HardwareError);
    }
    Ok(())
}

fn status_poll_delay() {
    let mut cycles = 0u32;
    while cycles < STATUS_POLL_DELAY {
        core::hint::spin_loop();
        cycles += 1;
    }
}

fn wait_write_complete(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    initial_status: u8,
) -> Result<(), SmcError> {
    let mut polls = 0u32;
    let mut clear_samples = 0u8;
    let mut status = initial_status;

    while polls < STATUS_MAX_POLLS {
        if status & STATUS_WIP == 0 {
            clear_samples += 1;
            if clear_samples == STATUS_CLEAR_SAMPLES {
                pw_log::info!(
                    "write complete: SR1=0x{:02x}, polls={}",
                    status as u32,
                    polls as u32
                );
                return Ok(());
            }
        } else {
            clear_samples = 0;
        }

        status_poll_delay();
        status = read_status(spi, spim)?;
        polls += 1;
    }
    Err(SmcError::Timeout)
}

fn read_fast_with_spim(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
    buf: &mut [u8],
) -> Result<(), SmcError> {
    let address = offset.to_be_bytes();
    // 0x0C: 4-byte-address Fast Read, followed by one dummy byte.
    let cmd = [
        0x0c, address[0], address[1], address[2], address[3], 0x00,
    ];
    command_with_spim(
        spi,
        spim,
        &cmd,
        &[],
        buf,
        TransferMode::Mode114,
    )
}

fn erase_sector(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
) -> Result<(), SmcError> {
    write_enable(spi, spim)?;
    let address = offset.to_be_bytes();
    let cmd = [0x21, address[0], address[1], address[2], address[3]];
    command_with_spim(
        spi,
        spim,
        &cmd,
        &[],
        &mut [],
        TransferMode::Mode111,
    )?;
    let status = read_status(spi, spim)?;
    pw_log::info!("erase command SR1=0x{:02x}", status as u32);
    if status & STATUS_WIP == 0 {
        pw_log::info!("erase command did not enter busy state");
        return Err(SmcError::HardwareError);
    }
    wait_write_complete(spi, spim, status)
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
    command_with_spim(
        spi,
        spim,
        &cmd,
        data,
        &mut [],
        TransferMode::Mode114,
    )?;
    let status = read_status(spi, spim)?;
    pw_log::info!("program command SR1=0x{:02x}", status as u32);
    wait_write_complete(spi, spim, status)
}

#[allow(dead_code)]
fn verify_data(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    offset: u32,
    expected: &[u8],
) -> Result<(), SmcError> {
    let dma_buf = unsafe { core::slice::from_raw_parts_mut(DMA_BUFFER as *mut u8, 256) };
    let mut cursor = 0usize;
    while cursor < expected.len() {
        let len = core::cmp::min(PAGE_LEN, expected.len() - cursor);
        dma_read_with_spim(
            spi,
            spim,
            offset + cursor as u32,
            &mut dma_buf[..len],
        )?;
        if dma_buf[..len] != expected[cursor..cursor + len] {
            let mut mismatch = 0usize;
            while mismatch < len
                && dma_buf[mismatch] == expected[cursor + mismatch]
            {
                mismatch += 1;
            }
            pw_log::info!(
                "data verify failed at 0x{:08x}: expected {:02x}, got {:02x}",
                (offset as usize + cursor + mismatch) as u32,
                expected[cursor + mismatch] as u32,
                dma_buf[mismatch] as u32
            );
            return Err(SmcError::HardwareError);
        }
        cursor += len;
    }
    Ok(())
}

#[allow(dead_code)]
fn restore_sector(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    original: &[u8],
) -> Result<(), SmcError> {
    erase_sector(spi, spim, TEST_OFFSET)?;

    let mut cursor = 0usize;
    while cursor < original.len() {
        let len = core::cmp::min(PAGE_LEN, original.len() - cursor);
        let page = &original[cursor..cursor + len];
        if !is_erased(page) {
            program_page(spi, spim, TEST_OFFSET + cursor as u32, page)?;
        }
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

    let mut read_buf = PageBuffer([0u8; PAGE_LEN]);
    
    read_fast_with_spim(spi, spim, TEST_OFFSET, &mut read_buf.0)?;
    if let Err(error) = expect_erased(&read_buf.0) {
        pw_log::info!("user-mode erase verification failed");
        return Err(error);
    }
    pw_log::info!("user-mode erase verification passed");

    let dma_buf = unsafe { core::slice::from_raw_parts_mut((DMA_BUFFER+0x100) as *mut u8, 256) };
    dma_read_with_spim(spi, spim, TEST_OFFSET, dma_buf)?;
    expect_erased(dma_buf)?;

    //dump_smc_read(dma_buf, PAGE_LEN as u32);

    let mut pattern = [0u8; PAGE_LEN];
    fill_test_pattern(&mut pattern, salt);
    program_page(spi, spim, TEST_OFFSET, &pattern)?;

    dma_read_with_spim(spi, spim, TEST_OFFSET, dma_buf)?;

    
    if dma_buf != pattern {
        let mut mismatch = 0usize;
        while mismatch < PAGE_LEN && dma_buf[mismatch] == pattern[mismatch] {
            mismatch += 1;
        }
        pw_log::info!(
            "program verify failed at 0x{:08x}: expected {:02x}, got {:02x}",
            (TEST_OFFSET as usize + mismatch) as u32,
            pattern[mismatch] as u32,
            dma_buf[mismatch] as u32
        );
        pw_log::info!("pattern::");
        dump_smc_read(&pattern, PAGE_LEN as u32);
        pw_log::info!("dma buffer::");
        dump_smc_read(dma_buf, PAGE_LEN as u32);
        return Err(SmcError::HardwareError);
    }
    
    Ok(())
}

pub fn run_flash_test(
    spi: &mut SpiReady,
    spim: SpiMonitorInstance,
    salt: u8,
) -> Result<(), SmcError> {
    let mut jedec = [0u8; 3];
    command_with_spim(
        spi,
        spim,
        &[0x9f],
        &[],
        &mut jedec,
        TransferMode::Mode111,
    )?;
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
    // SAFETY: This test has exclusive access to BACKUP_BUFFER until it returns.
    let original = unsafe { &mut *BACKUP_BUFFER.0.get() };
    dma_read_with_spim(spi, spim, TEST_OFFSET, original)?;

    pw_log::info!("=== erase/write/read test sector ===");
    let test_result = destructive_test(spi, spim, salt);
    /*
    pw_log::info!("=== restore test sector ===");
    let restore_result = restore_sector(spi, spim, original);

    test_result?;
    restore_result
    */
    test_result
}

pub fn new_spi(
    controller: SmcController,
    topology: SmcTopology,
) -> Result<SpiReady, SmcError> {
    let config = SmcConfig {
        controller_id: controller,
        cs0: Some(FLASH_CONFIG),
        cs1: None,
        dma_enabled: true,
        enable_interrupts: false,
        topology,
    };
    let spi = unsafe { SpiUninit::new(controller, config)? };
    let mut spi = spi.init()?;
    spi.spi_nor_read_init(ChipSelect::Cs0)?;
    Ok(spi)
}
