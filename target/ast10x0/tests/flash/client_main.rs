// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use app_flash_client::handle;
use hal_flash::{Flash, FlashAddress};
use services_flash_client::FlashIpcClient;
use userspace::entry;
use userspace::syscall;
use util_ipc::IpcHandle;

/// 1 MiB in: same offset the on-hardware smc write test uses; clear of code.
/// The test is non-destructive on hardware: the whole sector is backed up
/// before the first erase and restored + verified at the end, exactly as
/// //target/ast10x0/tests/smc/write does.
const TEST_OFFSET: u32 = 0x0010_0000;
const SECTOR: usize = 4096;

fn fail(msg: &str) -> ! {
    pw_log::error!("flash client FAIL: {}", msg as &str);
    let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
    loop {}
}

fn pattern(i: usize) -> u8 {
    (i as u8).wrapping_mul(31).wrapping_add(7)
}

#[entry]
fn entry() {
    let mut flash = match FlashIpcClient::new(IpcHandle::new(handle::FLASH)) {
        Ok(c) => c,
        Err(_) => fail("connect/geometry"),
    };

    // 1. Geometry matches the backend's CS0 config.
    let Ok((total, page, bitmap)) = flash.geometry() else {
        fail("geometry");
    };
    if total.get() != 8 * 1024 * 1024 {
        fail("total size");
    }
    if page.get() != SECTOR {
        fail("page size");
    }
    if bitmap != 1 << 12 {
        fail("erase bitmap");
    }

    // Back up the whole sector before any destructive op so the test restores
    // the original contents on real hardware (mirrors smc/write).
    let mut backup = [0u8; SECTOR];
    if flash
        .read(FlashAddress::new(TEST_OFFSET), &mut backup)
        .is_err()
    {
        fail("backup read");
    }

    // 2. Erase one sector, verify it reads back erased.
    if flash.erase(FlashAddress::new(TEST_OFFSET), page).is_err() {
        fail("erase");
    }
    let mut buf = [0u8; 64];
    if flash
        .read(FlashAddress::new(TEST_OFFSET), &mut buf)
        .is_err()
    {
        fail("read after erase");
    }
    if buf.iter().any(|&b| b != 0xff) {
        fail("not erased");
    }

    // 3. Unaligned program crossing a 256-byte program-page boundary:
    //    starts at +250, 300 bytes -> exercises BlockingFlash window
    //    splitting and the intra-page start relaxation.
    let mut data = [0u8; 300];
    for (i, b) in data.iter_mut().enumerate() {
        *b = pattern(i);
    }
    if flash
        .program(FlashAddress::new(TEST_OFFSET + 250), &data)
        .is_err()
    {
        fail("program");
    }

    // 4. Read back and verify, including the untouched prefix.
    let mut rb = [0u8; 600];
    if flash.read(FlashAddress::new(TEST_OFFSET), &mut rb).is_err() {
        fail("read back");
    }
    if rb[..250].iter().any(|&b| b != 0xff) {
        fail("prefix clobbered");
    }
    for i in 0..300 {
        if rb[250 + i] != pattern(i) {
            fail("data mismatch");
        }
    }
    if rb[550..].iter().any(|&b| b != 0xff) {
        fail("suffix clobbered");
    }

    // 5. Error paths: bad erase size, out-of-bounds read.
    if flash
        .erase(
            FlashAddress::new(TEST_OFFSET),
            util_types::PowerOf2Usize::new(512).unwrap(),
        )
        .is_ok()
    {
        fail("erase size not rejected");
    }
    let mut oob = [0u8; 16];
    if flash.read(FlashAddress::new(0x0100_0000), &mut oob).is_ok() {
        fail("oob read not rejected");
    }

    // Restore the original sector contents and verify (mirrors smc/write's
    // restore_sector: erase -> program original -> read-back compare).
    if flash.erase(FlashAddress::new(TEST_OFFSET), page).is_err() {
        fail("restore erase");
    }
    if flash
        .program(FlashAddress::new(TEST_OFFSET), &backup)
        .is_err()
    {
        fail("restore program");
    }
    let mut restored = [0u8; SECTOR];
    if flash
        .read(FlashAddress::new(TEST_OFFSET), &mut restored)
        .is_err()
    {
        fail("restore read");
    }
    if restored != backup {
        fail("restore verify");
    }

    pw_log::info!("flash client PASS");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
