// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Erases, programs and reads back flash through the flash service, and checks
//! that the server rejects requests outside the flash or not aligned to an
//! erase block.

#![no_main]
#![no_std]

use app_flash_client::handle;
use hal_flash::{Flash, FlashAddress};
use openprot_flash_client_ipc::{ChannelTransport, FlashIpcClient};
use pw_status::{Error, Result};
use userspace::{entry, syscall};
use util_ipc::IpcHandle;
use veer_flash::{ERASE_SIZE, PAGE_SIZE};

// The second erase block.
const BLOCK: usize = ERASE_SIZE;

// The data starts in the middle of a page and ends in the next page.
const DATA_START: usize = BLOCK + PAGE_SIZE - 20;
const DATA_LEN: usize = 48;

fn addr(offset: usize) -> FlashAddress {
    FlashAddress::new(offset as u32)
}

fn pattern(i: usize) -> u8 {
    0x5a ^ i as u8
}

/// Reads `N` bytes at `offset` and checks each against `expected`.
fn check<const N: usize>(
    flash: &mut impl Flash,
    offset: usize,
    expected: impl Fn(usize) -> u8,
) -> core::result::Result<(), &'static str> {
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

fn run() -> core::result::Result<(), &'static str> {
    let transport = ChannelTransport::new(IpcHandle::new(handle::FLASH));
    let mut flash = FlashIpcClient::new(transport).map_err(|_| "geometry request failed")?;
    let (size, block, erase_sizes) = flash.geometry().map_err(|_| "geometry failed")?;
    if block.get() != ERASE_SIZE || erase_sizes != ERASE_SIZE as u32 {
        return Err("wrong geometry");
    }

    // The flash can hold data from an earlier run, so erase it first.
    flash
        .erase(addr(BLOCK), block)
        .map_err(|_| "erase failed")?;
    check::<DATA_LEN>(&mut flash, DATA_START, |_| 0xff)?;

    let mut data = [0; DATA_LEN];
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = pattern(i);
    }
    flash
        .program(addr(DATA_START), &data)
        .map_err(|_| "program failed")?;
    check::<DATA_LEN>(&mut flash, DATA_START, pattern)?;
    // The bytes before and after the data are unchanged.
    check::<8>(&mut flash, DATA_START - 8, |_| 0xff)?;
    check::<8>(&mut flash, DATA_START + DATA_LEN, |_| 0xff)?;

    if flash.read(addr(size.get() - 1), &mut [0; 2]).is_ok() {
        return Err("read past the end succeeded");
    }
    if flash.erase(addr(BLOCK + PAGE_SIZE), block).is_ok() {
        return Err("unaligned erase succeeded");
    }
    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    let result = run().map_err(|reason| {
        pw_log::error!("FAIL: {}", reason as &str);
        Error::Internal
    });
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}
