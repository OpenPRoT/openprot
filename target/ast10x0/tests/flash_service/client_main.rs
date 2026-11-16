// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Erases, programs and reads back the CS0 and CS1 flash through their own flash
//! service channels, checks that the server rejects requests that are not
//! valid, and checks that each channel reaches only its own flash.
//!
//! The test works on a sector of each flash that can hold data, so it saves the
//! sector first and restores it at the end, even if a check failed. CS0 is the
//! boot flash on the EVB, so its sector is the last one of the flash, far from
//! the boot image; CS1 uses 1 MiB in.

#![no_main]
#![no_std]

use app_flash_client::handle;
use hal_flash::{Flash, FlashAddress};
use openprot_flash_client_ipc::{ChannelTransport, FlashIpcClient};
use pw_status::{Error, Result};
use userspace::{entry, syscall};
use util_ipc::IpcHandle;
use util_types::PowerOf2Usize;

/// CS1 test sector, 1 MiB in: same offset the on-hardware smc write test uses;
/// clear of code.
const CS1_TEST_OFFSET: usize = 0x0010_0000;
/// The bytes the isolation check programs at the start of each test sector.
const ISOLATION_LEN: usize = 64;
/// The largest sector the test saves, which sizes the backup. The backend
/// accepts only 4 KiB sectors today.
const MAX_SECTOR: usize = 4096;
/// The program data, and the flash read back before and after it.
const DATA_LEN: usize = 300;
const MARGIN: usize = 50;
const READ_LEN: usize = MARGIN + DATA_LEN + MARGIN;
/// The data starts this many bytes before the middle of the sector.
const BEFORE_MIDDLE: usize = 6;
/// The smallest sector that holds the read back around the middle.
const MIN_SECTOR: usize = 2 * (BEFORE_MIDDLE + DATA_LEN + MARGIN);
/// The chunk the restore compares at a time.
const CHUNK: usize = 256;

/// The flash a channel reaches.
#[derive(Clone, Copy)]
enum Cs {
    Cs0,
    Cs1,
}

/// The sector the test works on in one flash, and the flash's geometry.
#[derive(Clone, Copy)]
struct Target {
    /// The offset of the test sector.
    base: usize,
    sector: PowerOf2Usize,
    total: usize,
}

fn addr(offset: usize) -> FlashAddress {
    FlashAddress::new(offset as u32)
}

/// Where the program starts. A program window is a power of two smaller than
/// the sector, so it divides the middle of the sector, as long as the sector
/// is aligned to it. The service does not report the window, but a program
/// that starts just before the middle crosses a window boundary for any
/// window.
fn program_start(base: usize, sector: PowerOf2Usize) -> usize {
    base + sector.get() / 2 - BEFORE_MIDDLE
}

fn pattern(i: usize) -> u8 {
    (i as u8).wrapping_mul(31).wrapping_add(7)
}

/// Checks that the geometry the server reports, which the backend discovers
/// over SFDP, is consistent and that the test fits in the flash. Returns the
/// erase sector and the flash size.
fn check_geometry(flash: &mut impl Flash, cs: Cs) -> core::result::Result<Target, &'static str> {
    let (total, sector, erase_sizes) = flash.geometry().map_err(|_| "geometry")?;
    let sector_len = sector.get();
    if !(MIN_SECTOR..=MAX_SECTOR).contains(&sector_len) {
        return Err("unsupported sector size");
    }
    let base = match cs {
        Cs::Cs0 => total.get().saturating_sub(sector_len),
        Cs::Cs1 => CS1_TEST_OFFSET,
    };
    if base % sector_len != 0 {
        return Err("unsupported sector size");
    }
    // The only erase size is the sector.
    if erase_sizes != 1 << sector_len.trailing_zeros() {
        return Err("erase sizes do not match the sector");
    }
    if total.get() % sector_len != 0 || total.get() < base + sector_len {
        return Err("flash size does not fit the test");
    }
    Ok(Target {
        base,
        sector,
        total: total.get(),
    })
}

fn run(flash: &mut impl Flash, target: Target) -> core::result::Result<(), &'static str> {
    let Target {
        base,
        sector,
        total,
    } = target;
    // A size the flash cannot erase: half the sector.
    let small = PowerOf2Usize::new(sector.get() / 2).ok_or("small size")?;

    // 1. Erase one sector, verify it reads back erased.
    flash.erase(addr(base), sector).map_err(|_| "erase")?;
    let mut buf = [0u8; 64];
    flash
        .read(addr(base), &mut buf)
        .map_err(|_| "read after erase")?;
    if buf.iter().any(|&b| b != 0xff) {
        return Err("not erased");
    }

    // 2. Unaligned program crossing a program-page boundary.
    let mut data = [0u8; DATA_LEN];
    for (i, b) in data.iter_mut().enumerate() {
        *b = pattern(i);
    }
    flash
        .program(addr(program_start(base, sector)), &data)
        .map_err(|_| "program")?;

    // 3. Read back and verify, including the untouched prefix and suffix.
    let mut rb = [0u8; READ_LEN];
    flash
        .read(addr(program_start(base, sector) - MARGIN), &mut rb)
        .map_err(|_| "read back")?;
    let (prefix, rest) = rb.split_at(MARGIN);
    let (written, suffix) = rest.split_at(DATA_LEN);
    if prefix.iter().any(|&b| b != 0xff) {
        return Err("prefix clobbered");
    }
    if written.iter().enumerate().any(|(i, &b)| b != pattern(i)) {
        return Err("data mismatch");
    }
    if suffix.iter().any(|&b| b != 0xff) {
        return Err("suffix clobbered");
    }

    // 4. Error paths: bad erase size, unaligned erase, out-of-bounds read.
    if flash.erase(addr(base), small).is_ok() {
        return Err("erase size not rejected");
    }
    if flash.erase(addr(base + sector.get() / 2), sector).is_ok() {
        return Err("unaligned erase not rejected");
    }
    if flash.read(addr(total), &mut [0u8; 16]).is_ok() {
        return Err("oob read not rejected");
    }
    Ok(())
}

/// Writes `backup` back over the sector and checks it.
fn restore(
    flash: &mut impl Flash,
    target: Target,
    backup: &[u8],
) -> core::result::Result<(), &'static str> {
    let Target { base, sector, .. } = target;
    flash
        .erase(addr(base), sector)
        .map_err(|_| "restore erase")?;
    flash
        .program(addr(base), backup)
        .map_err(|_| "restore program")?;
    for (i, expected) in backup.chunks(CHUNK).enumerate() {
        let mut buf = [0u8; CHUNK];
        let buf = buf.get_mut(..expected.len()).ok_or("restore buffer")?;
        flash
            .read(addr(base + i * CHUNK), buf)
            .map_err(|_| "restore read")?;
        if buf != expected {
            return Err("restore verify");
        }
    }
    Ok(())
}

/// Runs `f` on the test sector of `flash`, then restores the sector.
fn with_backup<T: Flash>(
    flash: &mut T,
    target: Target,
    f: impl FnOnce(&mut T) -> core::result::Result<(), &'static str>,
) -> core::result::Result<(), &'static str> {
    let mut backup = [0u8; MAX_SECTOR];
    let backup = backup
        .get_mut(..target.sector.get())
        .ok_or("backup buffer")?;
    flash
        .read(addr(target.base), backup)
        .map_err(|_| "backup read")?;
    let result = f(flash);
    // Restore even when a check failed, so the test leaves the flash as it
    // found it.
    let restored = restore(flash, target, backup);
    result.and(restored)
}

fn fill(i: usize, salt: u8) -> u8 {
    pattern(i) ^ salt
}

/// Reads `ISOLATION_LEN` bytes at the start of the test sector and checks them
/// against `expected`.
fn check_start(
    flash: &mut impl Flash,
    target: Target,
    expected: impl Fn(usize) -> u8,
) -> core::result::Result<(), &'static str> {
    let mut buf = [0u8; ISOLATION_LEN];
    flash
        .read(addr(target.base), &mut buf)
        .map_err(|_| "isolation read")?;
    if buf.iter().enumerate().all(|(i, &b)| b == expected(i)) {
        Ok(())
    } else {
        Err("channel reached the wrong flash")
    }
}

/// Programs a pattern through one channel and checks the other channel's flash
/// is unchanged, in both directions.
fn run_isolation<T: Flash>(
    cs0: &mut T,
    t0: Target,
    cs1: &mut T,
    t1: Target,
) -> core::result::Result<(), &'static str> {
    let program = |flash: &mut T, target: Target, salt: u8| {
        let mut data = [0u8; ISOLATION_LEN];
        for (i, b) in data.iter_mut().enumerate() {
            *b = fill(i, salt);
        }
        flash
            .program(addr(target.base), &data)
            .map_err(|_| "isolation program")
    };
    cs0.erase(addr(t0.base), t0.sector)
        .map_err(|_| "isolation erase cs0")?;
    cs1.erase(addr(t1.base), t1.sector)
        .map_err(|_| "isolation erase cs1")?;

    program(cs0, t0, 0x11)?;
    check_start(cs0, t0, |i| fill(i, 0x11))?;
    check_start(cs1, t1, |_| 0xff)?;

    program(cs1, t1, 0x22)?;
    check_start(cs1, t1, |i| fill(i, 0x22))?;
    check_start(cs0, t0, |i| fill(i, 0x11))
}

fn run_with_backup() -> core::result::Result<(), &'static str> {
    let mut cs0 = FlashIpcClient::new(ChannelTransport::new(IpcHandle::new(handle::FLASH_CS0)))
        .map_err(|_| "connect/geometry cs0")?;
    let mut cs1 = FlashIpcClient::new(ChannelTransport::new(IpcHandle::new(handle::FLASH_CS1)))
        .map_err(|_| "connect/geometry cs1")?;
    let t0 = check_geometry(&mut cs0, Cs::Cs0)?;
    let t1 = check_geometry(&mut cs1, Cs::Cs1)?;

    with_backup(&mut cs0, t0, |flash| run(flash, t0)).map_err(|e| {
        pw_log::error!("cs0 FAIL");
        e
    })?;
    with_backup(&mut cs1, t1, |flash| run(flash, t1)).map_err(|e| {
        pw_log::error!("cs1 FAIL");
        e
    })?;

    // Back up both sectors, since the check programs both.
    with_backup(&mut cs0, t0, |cs0| {
        with_backup(&mut cs1, t1, |cs1| run_isolation(cs0, t0, cs1, t1))
    })
}

#[entry]
fn entry() -> Result<()> {
    let result = run_with_backup().map_err(|reason| {
        pw_log::error!("flash client FAIL: {}", reason as &str);
        Error::Internal
    });
    if result.is_ok() {
        pw_log::info!("flash client PASS");
    }
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}
