// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Erases, programs and reads back the FMC CS1 flash through the flash HAL,
//! without IPC. The AST10x0 counterpart of `target/veer/tests/flash`; the
//! `flash_service` test covers the same flash over IPC.
//!
//! The test works on two sectors of a flash that can hold data, so it saves
//! both first and restores them at the end.

#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use console_backend::console_backend_write_all;
use flash_backend::{Backend, NoWaitBlocking};
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use target_common::{declare_target, TargetInterface};
use util_types::PowerOf2Usize;
use {codegen as _, console_backend as _, entry as _};

const SECTOR: usize = 4096;
const PAGE: usize = 256;

// Two sectors 1 MiB in, the offset the smc write test uses. Clear of code.
const BLOCK_A: usize = 0x10_0000;
const BLOCK_B: usize = BLOCK_A + SECTOR;

// The data starts in the middle of a page and ends in the next page.
const DATA_START: usize = BLOCK_A + PAGE - 20;
const DATA_LEN: usize = 48;

/// Holds both sectors while the test runs.
static mut BACKUP: [u8; 2 * SECTOR] = [0; 2 * SECTOR];

fn addr(offset: usize) -> FlashAddress {
    FlashAddress::new(offset as u32)
}

fn pattern(i: usize) -> u8 {
    0xa5 ^ i as u8
}

/// Reads `N` bytes at `offset` and checks each against `expected`.
fn check<const N: usize>(
    flash: &mut impl Flash,
    offset: usize,
    expected: impl Fn(usize) -> u8,
) -> Result<(), &'static str> {
    let mut buf = [0; N];
    flash
        .read(addr(offset), &mut buf)
        .map_err(|_| "read failed")?;
    if buf.iter().enumerate().all(|(i, &byte)| byte == expected(i)) {
        Ok(())
    } else {
        Err("read back the wrong data")
    }
}

fn run(flash: &mut impl Flash) -> Result<(), &'static str> {
    let block = PowerOf2Usize::new(SECTOR).ok_or("sector size")?;
    let (size, page_size, erase_sizes) = flash.geometry().map_err(|_| "geometry failed")?;
    if page_size != block || erase_sizes != SECTOR as u32 {
        return Err("wrong geometry");
    }

    for start in [BLOCK_A, BLOCK_B] {
        flash
            .erase(addr(start), block)
            .map_err(|_| "erase failed")?;
    }
    check::<DATA_LEN>(flash, DATA_START, |_| 0xff)?;

    let mut data = [0; DATA_LEN];
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = pattern(i);
    }
    flash
        .program(addr(DATA_START), &data)
        .map_err(|_| "program failed")?;
    check::<DATA_LEN>(flash, DATA_START, pattern)?;
    // The bytes before and after the data are unchanged.
    check::<8>(flash, DATA_START - 8, |_| 0xff)?;
    check::<8>(flash, DATA_START + DATA_LEN, |_| 0xff)?;

    // Programming can only clear bits.
    flash
        .program(addr(DATA_START), &[0x0f; DATA_LEN])
        .map_err(|_| "second program failed")?;
    check::<DATA_LEN>(flash, DATA_START, |i| pattern(i) & 0x0f)?;

    // An erase clears its block and leaves the next block unchanged.
    for start in [BLOCK_A + SECTOR - 8, BLOCK_B] {
        flash
            .program(addr(start), &[0; 8])
            .map_err(|_| "program failed")?;
    }
    flash
        .erase(addr(BLOCK_A), block)
        .map_err(|_| "second erase failed")?;
    check::<DATA_LEN>(flash, DATA_START, |_| 0xff)?;
    check::<8>(flash, BLOCK_A + SECTOR - 8, |_| 0xff)?;
    check::<8>(flash, BLOCK_B, |_| 0)?;

    if flash.read(addr(size.get() - 1), &mut [0; 2]).is_ok() {
        return Err("read past the end succeeded");
    }
    if flash.erase(addr(BLOCK_A + PAGE), block).is_ok() {
        return Err("unaligned erase succeeded");
    }
    Ok(())
}

/// Writes `backup` back over both sectors and checks it.
fn restore(flash: &mut impl Flash, backup: &[u8]) -> Result<(), &'static str> {
    let block = PowerOf2Usize::new(SECTOR).ok_or("sector size")?;
    for (start, saved) in [BLOCK_A, BLOCK_B].into_iter().zip(backup.chunks(SECTOR)) {
        flash
            .erase(addr(start), block)
            .map_err(|_| "restore erase failed")?;
        flash
            .program(addr(start), saved)
            .map_err(|_| "restore program failed")?;
        for (i, expected) in saved.chunks(PAGE).enumerate() {
            let mut buf = [0; PAGE];
            flash
                .read(addr(start + i * PAGE), &mut buf)
                .map_err(|_| "restore read failed")?;
            if buf.as_slice() != expected {
                return Err("restore verify failed");
            }
        }
    }
    Ok(())
}

fn run_with_backup(flash: &mut impl Flash) -> Result<(), &'static str> {
    // SAFETY: the kernel runs this once on a single thread, and nothing else
    // refers to `BACKUP`.
    let backup = unsafe { &mut *addr_of_mut!(BACKUP) };
    flash
        .read(addr(BLOCK_A), backup)
        .map_err(|_| "backup read failed")?;
    let result = run(flash);
    // Restore even when the test failed, so it leaves the flash as it found it.
    let restored = restore(flash, backup);
    result.and(restored)
}

fn run_flash_test() -> Result<(), &'static str> {
    // SAFETY: kernel main() runs once, single-threaded, with exclusive
    // hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    // SAFETY: this image has no other user of the FMC, and this runs once.
    let driver = unsafe { Backend::new() }.map_err(|_| "FMC init failed")?;
    let mut flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    run_with_backup(&mut flash)
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Flash Test";

    fn main() -> ! {
        let sentinel: &[u8] = match run_flash_test() {
            Ok(()) => {
                pw_log::info!("PASS");
                b"TEST_RESULT:PASS\n"
            }
            Err(reason) => {
                pw_log::error!("FAIL: {}", reason as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
