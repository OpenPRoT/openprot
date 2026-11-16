// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Erases, programs and reads back the FMC CS0 and CS1 flash through the flash
//! HAL, without IPC, using one driver per chip select on a shared controller. The AST10x0 counterpart of `target/veer/tests/flash`; the
//! `flash_service` test covers the same flash over IPC.
//!
//! The test works on two sectors of each flash, so it saves both first and
//! restores them at the end. CS0 is the boot flash on the EVB, so its sectors
//! are the last two of the flash, far from the boot image; CS1 uses 1 MiB in.

#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use ast10x0_peripherals::smc::ChipSelect;
use console_backend::console_backend_write_all;
use flash_backend::{FmcController, NoWaitBlocking};
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use hal_flash_driver::FlashDriver;
use target_common::{declare_target, TargetInterface};
use util_types::PowerOf2Usize;
use {codegen as _, console_backend as _, entry as _};

/// The largest sector the test saves, which sizes the backup. The backend
/// accepts only 4 KiB sectors today.
const MAX_SECTOR: usize = 4096;

/// Two sectors 1 MiB in, the offset the smc write test uses. Clear of code.
const CS1_BLOCK_A: usize = 0x10_0000;

const DATA_LEN: usize = 48;

/// Holds both sectors while the test runs.
static mut BACKUP: [u8; 2 * MAX_SECTOR] = [0; 2 * MAX_SECTOR];

/// The flash's geometry, discovered from the device over SFDP.
#[derive(Clone, Copy)]
struct Geometry {
    /// The first of the two sectors the test works on.
    block_a: usize,
    /// The erase sector.
    sector: PowerOf2Usize,
    /// The program page: a program must not cross its boundary.
    page: usize,
}

impl Geometry {
    fn block_b(&self) -> usize {
        self.block_a + self.sector.get()
    }

    /// The data starts in the middle of a page and ends in the next page.
    fn data_start(&self) -> usize {
        self.block_a + self.page - 20
    }
}

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

/// Checks that the geometry the flash reports is consistent, and that the test
/// fits in the flash.
fn check_geometry(
    flash: &mut impl Flash,
    cs: ChipSelect,
    page: usize,
) -> Result<Geometry, &'static str> {
    let (size, sector, erase_sizes) = flash.geometry().map_err(|_| "geometry failed")?;
    let sector_len = sector.get();
    let block_a = match cs {
        ChipSelect::Cs0 => size.get().saturating_sub(sector_len.saturating_mul(2)),
        ChipSelect::Cs1 => CS1_BLOCK_A,
    };
    let geometry = Geometry {
        block_a,
        sector,
        page,
    };
    if sector_len > MAX_SECTOR || page < 20 + DATA_LEN || page >= sector_len {
        return Err("unsupported geometry");
    }
    // The only erase size is the sector.
    if erase_sizes != 1 << sector_len.trailing_zeros() {
        return Err("erase sizes do not match the sector");
    }
    if size.get() % sector_len != 0
        || block_a % sector_len != 0
        || size.get() < geometry.block_b() + sector_len
    {
        return Err("flash size does not fit the test");
    }
    Ok(geometry)
}

fn run(flash: &mut impl Flash, geometry: Geometry) -> Result<(), &'static str> {
    let block = geometry.sector;
    let sector = block.get();
    let page = geometry.page;
    let (block_a, block_b) = (geometry.block_a, geometry.block_b());
    let data_start = geometry.data_start();
    let (size, _, _) = flash.geometry().map_err(|_| "geometry failed")?;

    for start in [block_a, block_b] {
        flash
            .erase(addr(start), block)
            .map_err(|_| "erase failed")?;
    }
    check::<DATA_LEN>(flash, data_start, |_| 0xff)?;

    let mut data = [0; DATA_LEN];
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = pattern(i);
    }
    flash
        .program(addr(data_start), &data)
        .map_err(|_| "program failed")?;
    check::<DATA_LEN>(flash, data_start, pattern)?;
    // The bytes before and after the data are unchanged.
    check::<8>(flash, data_start - 8, |_| 0xff)?;
    check::<8>(flash, data_start + DATA_LEN, |_| 0xff)?;

    // Programming can only clear bits.
    flash
        .program(addr(data_start), &[0x0f; DATA_LEN])
        .map_err(|_| "second program failed")?;
    check::<DATA_LEN>(flash, data_start, |i| pattern(i) & 0x0f)?;

    // An erase clears its block and leaves the next block unchanged.
    for start in [block_a + sector - 8, block_b] {
        flash
            .program(addr(start), &[0; 8])
            .map_err(|_| "program failed")?;
    }
    flash
        .erase(addr(block_a), block)
        .map_err(|_| "second erase failed")?;
    check::<DATA_LEN>(flash, data_start, |_| 0xff)?;
    check::<8>(flash, block_a + sector - 8, |_| 0xff)?;
    check::<8>(flash, block_b, |_| 0)?;

    if flash.read(addr(size.get() - 1), &mut [0; 2]).is_ok() {
        return Err("read past the end succeeded");
    }
    if flash.erase(addr(block_a + page), block).is_ok() {
        return Err("unaligned erase succeeded");
    }
    Ok(())
}

/// Writes `backup` back over both sectors and checks it.
fn restore(flash: &mut impl Flash, geometry: Geometry, backup: &[u8]) -> Result<(), &'static str> {
    let sector = geometry.sector.get();
    let starts = [geometry.block_a, geometry.block_b()];
    for (start, saved) in starts.into_iter().zip(backup.chunks(sector)) {
        flash
            .erase(addr(start), geometry.sector)
            .map_err(|_| "restore erase failed")?;
        flash
            .program(addr(start), saved)
            .map_err(|_| "restore program failed")?;
        for (i, expected) in saved.chunks(geometry.page).enumerate() {
            let mut buf = [0; MAX_SECTOR];
            let buf = buf.get_mut(..expected.len()).ok_or("restore buffer")?;
            flash
                .read(addr(start + i * geometry.page), buf)
                .map_err(|_| "restore read failed")?;
            if buf != expected {
                return Err("restore verify failed");
            }
        }
    }
    Ok(())
}

fn run_with_backup(
    flash: &mut impl Flash,
    cs: ChipSelect,
    program_page: usize,
) -> Result<(), &'static str> {
    let geometry = check_geometry(flash, cs, program_page)?;
    // SAFETY: the kernel runs this once on a single thread, and nothing else
    // refers to `BACKUP`.
    let backup = unsafe { &mut *addr_of_mut!(BACKUP) };
    let backup = backup
        .get_mut(..2 * geometry.sector.get())
        .ok_or("backup buffer")?;
    flash
        .read(addr(geometry.block_a), backup)
        .map_err(|_| "backup read failed")?;
    let result = run(flash, geometry);
    // Restore even when the test failed, so it leaves the flash as it found it.
    let restored = restore(flash, geometry, backup);
    result.and(restored)
}

fn run_flash_test() -> Result<(), &'static str> {
    // SAFETY: kernel main() runs once, single-threaded, with exclusive
    // hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    // SAFETY: this image has no other user of the FMC, and this runs once.
    let controller = unsafe { FmcController::new() }.map_err(|_| "FMC init failed")?;
    for (cs, name) in [(ChipSelect::Cs0, "CS0"), (ChipSelect::Cs1, "CS1")] {
        let driver = controller
            .driver(cs)
            .map_err(|_| "flash driver init failed")?;
        let mut flash = BlockingFlash {
            driver,
            blocking: NoWaitBlocking,
        };
        // The SFDP-derived program page the driver reports.
        let program_page = flash.driver.program_window_size().get();
        pw_log::info!("testing {}", name as &str);
        run_with_backup(&mut flash, cs, program_page)?;
    }
    Ok(())
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
