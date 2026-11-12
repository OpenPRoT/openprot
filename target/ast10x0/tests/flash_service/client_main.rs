// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Erases, programs and reads back flash through the flash service, and checks
//! that the server rejects requests that are not valid.
//!
//! The test works on a sector of a flash that can hold data, so it saves the
//! sector first and restores it at the end, even if a check failed.

#![no_main]
#![no_std]

use app_flash_client::handle;
use hal_flash::{Flash, FlashAddress};
use openprot_flash_client_ipc::{ChannelTransport, FlashIpcClient};
use pw_status::{Error, Result};
use userspace::{entry, syscall};
use util_ipc::IpcHandle;
use util_types::PowerOf2Usize;

/// 1 MiB in: same offset the on-hardware smc write test uses; clear of code.
const TEST_OFFSET: usize = 0x0010_0000;
const SECTOR: usize = 4096;
/// Starts 6 bytes before a 256-byte program page boundary, so the program
/// crosses it.
const PROGRAM_START: usize = 250;
const DATA_LEN: usize = 300;
/// The bytes the read back covers: the data plus 50 bytes of flash after it.
const READ_LEN: usize = 600;

fn addr(offset: usize) -> FlashAddress {
    FlashAddress::new(offset as u32)
}

fn pattern(i: usize) -> u8 {
    (i as u8).wrapping_mul(31).wrapping_add(7)
}

fn run(flash: &mut impl Flash) -> core::result::Result<(), &'static str> {
    let sector = PowerOf2Usize::new(SECTOR).ok_or("sector size")?;
    let small = PowerOf2Usize::new(512).ok_or("small size")?;

    // 1. The geometry matches the FMC flash.
    let (total, page, bitmap) = flash.geometry().map_err(|_| "geometry")?;
    if total.get() != 0x0400_0000 {
        return Err("total size");
    }
    if page != sector {
        return Err("page size");
    }
    if bitmap != 1 << 12 {
        return Err("erase bitmap");
    }

    // 2. Erase one sector, verify it reads back erased.
    flash.erase(addr(TEST_OFFSET), page).map_err(|_| "erase")?;
    let mut buf = [0u8; 64];
    flash
        .read(addr(TEST_OFFSET), &mut buf)
        .map_err(|_| "read after erase")?;
    if buf.iter().any(|&b| b != 0xff) {
        return Err("not erased");
    }

    // 3. Unaligned program crossing a 256-byte program-page boundary.
    let mut data = [0u8; DATA_LEN];
    for (i, b) in data.iter_mut().enumerate() {
        *b = pattern(i);
    }
    flash
        .program(addr(TEST_OFFSET + PROGRAM_START), &data)
        .map_err(|_| "program")?;

    // 4. Read back and verify, including the untouched prefix and suffix.
    let mut rb = [0u8; READ_LEN];
    flash
        .read(addr(TEST_OFFSET), &mut rb)
        .map_err(|_| "read back")?;
    let (prefix, rest) = rb.split_at(PROGRAM_START);
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

    // 5. Error paths: bad erase size, unaligned erase, out-of-bounds read.
    if flash.erase(addr(TEST_OFFSET), small).is_ok() {
        return Err("erase size not rejected");
    }
    if flash.erase(addr(TEST_OFFSET + 256), page).is_ok() {
        return Err("unaligned erase not rejected");
    }
    if flash.read(addr(total.get()), &mut [0u8; 16]).is_ok() {
        return Err("oob read not rejected");
    }

    Ok(())
}

/// Writes `backup` back over the sector and checks it.
fn restore(
    flash: &mut impl Flash,
    backup: &[u8; SECTOR],
) -> core::result::Result<(), &'static str> {
    let sector = PowerOf2Usize::new(SECTOR).ok_or("sector size")?;
    flash
        .erase(addr(TEST_OFFSET), sector)
        .map_err(|_| "restore erase")?;
    flash
        .program(addr(TEST_OFFSET), backup)
        .map_err(|_| "restore program")?;
    let mut restored = [0u8; SECTOR];
    flash
        .read(addr(TEST_OFFSET), &mut restored)
        .map_err(|_| "restore read")?;
    if restored != *backup {
        return Err("restore verify");
    }
    Ok(())
}

fn run_with_backup() -> core::result::Result<(), &'static str> {
    let transport = ChannelTransport::new(IpcHandle::new(handle::FLASH));
    let mut flash = FlashIpcClient::new(transport).map_err(|_| "connect/geometry")?;

    let mut backup = [0u8; SECTOR];
    flash
        .read(addr(TEST_OFFSET), &mut backup)
        .map_err(|_| "backup read")?;
    let result = run(&mut flash);
    // Restore even when a check failed, so the test leaves the flash as it
    // found it.
    let restored = restore(&mut flash, &backup);
    result.and(restored)
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
