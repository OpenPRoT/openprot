// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Erases, programs and reads back the emulator's primary flash through the
//! flash HAL.
//!
//! The emulator stores flash contents in a file, which can exist from an
//! earlier run, so the test does not assume that the flash starts out erased.

#![no_std]
#![no_main]

use entry::exit;
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use target_common::{declare_target, TargetInterface};
use util_types::PowerOf2Usize;
use veer_flash::{EmulatorFlashCtrl, NoWait, SoftFlash, ERASE_SIZE, PAGE_SIZE};
use {codegen as _, console_backend as _};

// The second and third erase blocks.
const BLOCK_A: usize = ERASE_SIZE;
const BLOCK_B: usize = 2 * ERASE_SIZE;

// The data starts in the middle of a page and ends in the next page.
const DATA_START: usize = BLOCK_A + PAGE_SIZE - 20;
const DATA_LEN: usize = 48;

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
    let block = PowerOf2Usize::new(ERASE_SIZE).unwrap();
    let (size, page_size, erase_sizes) = flash.geometry().map_err(|_| "geometry failed")?;
    if page_size != block || erase_sizes != ERASE_SIZE as u32 {
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
    for start in [BLOCK_A + ERASE_SIZE - 8, BLOCK_B] {
        flash
            .program(addr(start), &[0; 8])
            .map_err(|_| "program failed")?;
    }
    flash
        .erase(addr(BLOCK_A), block)
        .map_err(|_| "second erase failed")?;
    check::<DATA_LEN>(flash, DATA_START, |_| 0xff)?;
    check::<8>(flash, BLOCK_A + ERASE_SIZE - 8, |_| 0xff)?;
    check::<8>(flash, BLOCK_B, |_| 0)?;

    if flash.read(addr(size.get() - 1), &mut [0; 2]).is_ok() {
        return Err("read past the end succeeded");
    }
    if flash.erase(addr(BLOCK_A + PAGE_SIZE), block).is_ok() {
        return Err("unaligned erase succeeded");
    }
    Ok(())
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU Software Flash";

    fn main() -> ! {
        // SAFETY: this code runs in the kernel, which can access the
        // registers, and this image has no other user of the controller.
        let driver = SoftFlash::new(unsafe { EmulatorFlashCtrl::primary() });
        let mut flash = BlockingFlash {
            driver,
            blocking: NoWait,
        };
        match run(&mut flash) {
            Ok(()) => {
                pw_log::info!("PASS");
                exit(0);
            }
            Err(reason) => {
                pw_log::info!("FAIL: {}", reason as &str);
                exit(1);
            }
        }
    }
}

declare_target!(Target);
