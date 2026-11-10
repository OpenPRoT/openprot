// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash driver for the flash controllers of the Caliptra MCU emulator.
//!
//! The emulator has a primary and a secondary flash controller. They are
//! implemented in software by `DummyFlashCtrl` in caliptra-mcu-sw
//! `emulator/periph`, which keeps the flash contents in a file on the host.
//! A controller transfers one 256-byte page at a time between that file and a
//! page buffer in MCU RAM. Its registers are described in caliptra-mcu-sw
//! `hw/flash_ctrl.rdl`.
//!
//! [`SoftFlash`] implements `hal_flash_driver::FlashDriver` for a controller.
//! It presents the controller as NOR flash with a 256-byte program window and
//! 4 KiB erase blocks. `start_erase` and `start_program` return after the
//! operation has completed, so use [`NoWait`] as the `Blocking` of a
//! `hal_flash::BlockingFlash`.

#![cfg_attr(not(test), no_std)]

use core::num::NonZero;
use core::ops::Range;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::primary_flash_ctrl::bits::FlControl;
use hal_flash_driver::{FlashAddress, FlashDriver};
use pw_status::Error;
use util_error::{self as error, ErrorCode, ErrorModule};
use util_types::{Blocking, PowerOf2Usize};

mod flash_emulator;

pub use flash_emulator::EmulatorFlashCtrl;

/// Size of the page that a controller transfers, and of the program window.
pub const PAGE_SIZE: usize = 256;

/// Size of an erase block.
pub const ERASE_SIZE: usize = 4096;

/// Error module for the software flash controllers.
pub const FLASH_VEER: ErrorModule = ErrorModule::new(0x4656); // ascii `FV`.
/// The controller did not complete an operation within the poll limit.
pub const FLASH_VEER_TIMEOUT: ErrorCode = FLASH_VEER.from_pw(0, Error::DeadlineExceeded);
/// The controller reported that an operation failed.
pub const FLASH_VEER_OP_FAILED: ErrorCode = FLASH_VEER.from_pw(1, Error::Internal);

/// Maximum number of status polls for one operation. The emulator completes an
/// operation within a few hundred instructions.
const POLL_LIMIT: u32 = 10_000_000;

/// A controller register. Each value is the register's offset, from
/// caliptra-mcu-sw `hw/flash_ctrl.rdl`.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Reg {
    FlInterruptState = 0x00,
    FlInterruptEnable = 0x04,
    PageSize = 0x08,
    PageNum = 0x0c,
    PageAddr = 0x10,
    FlControl = 0x14,
    OpStatus = 0x18,
    CtrlRegwen = 0x1c,
}

/// `FL_CONTROL.OP` values.
#[derive(Clone, Copy)]
enum Op {
    Read = 1,
    Write = 2,
    Erase = 3,
}

impl Op {
    /// The `FL_CONTROL` value that starts this operation.
    fn start(self) -> u32 {
        (FlControl::Op.val(self as u32) + FlControl::Start::SET).into()
    }
}

/// Access to a controller's registers.
pub trait Mmio {
    fn read(&self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
}

/// A controller's memory-mapped registers.
pub struct MmioBlock {
    base: usize,
}

impl MmioBlock {
    /// # Safety
    ///
    /// `base` must be the address of a flash controller's registers, which are
    /// 0x20 bytes at a multiple of 4. They must be mapped for the caller, and
    /// no other code on the MCU may access them.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }
}

impl Mmio for MmioBlock {
    fn read(&self, reg: Reg) -> u32 {
        let addr = self.base.wrapping_add(reg as usize);
        // SAFETY: every `Reg` is the offset of a register inside the registers
        // that the caller of `new` guaranteed are mapped.
        unsafe { with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        let addr = self.base.wrapping_add(reg as usize);
        // SAFETY: every `Reg` is the offset of a register inside the registers
        // that the caller of `new` guaranteed are mapped.
        unsafe { with_exposed_provenance_mut::<u32>(addr).write_volatile(value) }
    }
}

/// Returns the range of `len` bytes at `offset` in a page, or an error if the
/// range does not fit in a page.
fn page_range(offset: usize, len: usize) -> Result<Range<usize>, ErrorCode> {
    match offset.checked_add(len) {
        Some(end) if end <= PAGE_SIZE => Ok(offset..end),
        _ => Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS),
    }
}

/// A software flash controller. It transfers one page at a time between flash
/// and a page buffer. Each method returns after its operation has completed.
pub trait PageController {
    /// Capacity in bytes of the controller's flash.
    const SIZE: NonZero<usize>;

    /// Number of bytes cleared by one [`erase`](Self::erase). [`ERASE_SIZE`]
    /// must be a multiple of it.
    const ERASE_OP_SIZE: usize;

    /// Copies `page` from flash into the page buffer.
    fn load(&mut self, page: u32) -> Result<(), ErrorCode>;

    /// Replaces `page` in flash with the page buffer.
    fn store(&mut self, page: u32) -> Result<(), ErrorCode>;

    /// Erases the block of `ERASE_OP_SIZE` bytes that contains `page`.
    fn erase(&mut self, page: u32) -> Result<(), ErrorCode>;

    /// Copies the page buffer from `offset` into `out`. Fails if `out` does
    /// not fit in the page at `offset`.
    fn read_buffer(&self, offset: usize, out: &mut [u8]) -> Result<(), ErrorCode>;

    /// ANDs `data` into the page buffer at `offset`, as programming NOR flash
    /// would. Fails if `data` does not fit in the page at `offset`.
    fn program_buffer(&mut self, offset: usize, data: &[u8]) -> Result<(), ErrorCode>;
}

/// Flash driver for a software flash controller.
pub struct SoftFlash<C: PageController> {
    controller: C,
}

impl<C: PageController> SoftFlash<C> {
    pub fn new(controller: C) -> Self {
        const { assert!(ERASE_SIZE % C::ERASE_OP_SIZE == 0) };
        Self { controller }
    }

    /// Returns the offset of `start` in the flash, or an error if the `len`
    /// bytes at `start` do not fit in the flash.
    fn check_range(start: FlashAddress, len: usize) -> Result<usize, ErrorCode> {
        let offset = start.offset() as usize;
        match offset.checked_add(len) {
            Some(end) if end <= C::SIZE.get() => Ok(offset),
            _ => Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS),
        }
    }
}

impl<C: PageController> FlashDriver for SoftFlash<C> {
    type Error = ErrorCode;

    // In `FlashDriver`, a page is the smallest erase block. Elsewhere in this
    // module, a page is the 256 bytes that a controller transfers.
    const PAGE_SIZE: usize = ERASE_SIZE;
    const PROGRAM_WINDOW_SIZE: usize = PAGE_SIZE;
    // `BlockingFlash` splits a read into chunks of this size.
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 1;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        C::SIZE
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error> {
        // Bit n stands for an erase size of 2^n bytes. `ERASE_SIZE` is the
        // only erase size.
        Ok(ERASE_SIZE as u32)
    }

    fn read(&mut self, start_addr: FlashAddress, mut buf: &mut [u8]) -> Result<(), Self::Error> {
        let mut offset = Self::check_range(start_addr, buf.len())?;
        while !buf.is_empty() {
            let in_page = offset % PAGE_SIZE;
            let (chunk, rest) = buf.split_at_mut(rest_of_page(in_page, buf.len()));
            self.controller.load((offset / PAGE_SIZE) as u32)?;
            self.controller.read_buffer(in_page, chunk)?;
            offset += chunk.len();
            buf = rest;
        }
        Ok(())
    }

    fn start_erase(
        &mut self,
        start_addr: FlashAddress,
        size: PowerOf2Usize,
    ) -> Result<(), Self::Error> {
        if size.get() != ERASE_SIZE {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        }
        let offset = Self::check_range(start_addr, ERASE_SIZE)?;
        if offset % ERASE_SIZE != 0 {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR);
        }
        for op_offset in (offset..offset + ERASE_SIZE).step_by(C::ERASE_OP_SIZE) {
            self.controller.erase((op_offset / PAGE_SIZE) as u32)?;
        }
        Ok(())
    }

    fn start_program(
        &mut self,
        start_address: FlashAddress,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        if data.is_empty() {
            return Ok(());
        }
        if data.len() > PAGE_SIZE {
            return Err(error::FLASH_GENERIC_PROGRAM_EXCEEDS_WINDOW_SIZE);
        }
        let offset = Self::check_range(start_address, data.len())?;
        let in_page = offset % PAGE_SIZE;
        if in_page + data.len() > PAGE_SIZE {
            return Err(error::FLASH_GENERIC_PROGRAM_SPANS_WINDOW_BOUNDARY);
        }
        // The controller writes whole pages, so read the page, modify it, and
        // write it back.
        let page = (offset / PAGE_SIZE) as u32;
        self.controller.load(page)?;
        self.controller.program_buffer(in_page, data)?;
        self.controller.store(page)
    }

    fn is_busy(&mut self) -> bool {
        false
    }

    fn complete_op(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Returns the number of bytes, at most `len`, between offset `in_page` of a
/// page and the end of that page.
fn rest_of_page(in_page: usize, len: usize) -> usize {
    core::cmp::min(PAGE_SIZE - in_page, len)
}

/// A [`Blocking`] that returns immediately, for use with [`SoftFlash`].
pub struct NoWait;

impl Blocking for NoWait {
    fn wait_for_notification(&self) {}
}

#[cfg(test)]
mod tests {
    use hal_flash::{BlockingFlash, Flash};

    use super::*;

    fn addr(offset: usize) -> FlashAddress {
        FlashAddress::new(offset as u32)
    }

    fn block() -> PowerOf2Usize {
        PowerOf2Usize::new(ERASE_SIZE).unwrap()
    }

    /// A controller that stores its flash in a `Vec` and erases 1 KiB per
    /// operation.
    struct FakeController {
        flash: Vec<u8>,
        page: [u8; PAGE_SIZE],
    }

    impl FakeController {
        fn new() -> Self {
            Self {
                flash: vec![0; Self::SIZE.get()],
                page: [0; PAGE_SIZE],
            }
        }

        fn stored(&mut self, page: u32) -> &mut [u8] {
            &mut self.flash[page as usize * PAGE_SIZE..][..PAGE_SIZE]
        }
    }

    impl PageController for FakeController {
        const SIZE: NonZero<usize> = NonZero::new(4 * ERASE_SIZE).unwrap();
        const ERASE_OP_SIZE: usize = 1024;

        fn load(&mut self, page: u32) -> Result<(), ErrorCode> {
            let stored = self.stored(page).to_vec();
            self.page.copy_from_slice(&stored);
            Ok(())
        }

        fn store(&mut self, page: u32) -> Result<(), ErrorCode> {
            let buffer = self.page;
            self.stored(page).copy_from_slice(&buffer);
            Ok(())
        }

        fn erase(&mut self, page: u32) -> Result<(), ErrorCode> {
            let start = page as usize * PAGE_SIZE / Self::ERASE_OP_SIZE * Self::ERASE_OP_SIZE;
            self.flash[start..][..Self::ERASE_OP_SIZE].fill(0xff);
            Ok(())
        }

        fn read_buffer(&self, offset: usize, out: &mut [u8]) -> Result<(), ErrorCode> {
            out.copy_from_slice(&self.page[offset..][..out.len()]);
            Ok(())
        }

        fn program_buffer(&mut self, offset: usize, data: &[u8]) -> Result<(), ErrorCode> {
            for (byte, data) in self.page[offset..][..data.len()].iter_mut().zip(data) {
                *byte &= data;
            }
            Ok(())
        }
    }

    #[test]
    fn works_as_blocking_flash() {
        let mut flash = BlockingFlash {
            driver: SoftFlash::new(FakeController::new()),
            blocking: NoWait,
        };
        let (size, page_size, erase_sizes) = flash.geometry().unwrap();
        assert_eq!(size, FakeController::SIZE);
        assert_eq!(page_size, block());
        assert_eq!(erase_sizes, 1 << 12);

        // The data starts and ends in the middle of a page, and spans four
        // pages.
        flash.erase(addr(0), block()).unwrap();
        let data: Vec<u8> = (0..700).map(|i| (i % 251) as u8).collect();
        flash.program(addr(0x1b3), &data).unwrap();
        let mut buf = [0; 700];
        flash.read(addr(0x1b3), &mut buf).unwrap();
        assert_eq!(buf[..], data);
    }

    #[test]
    fn erase_clears_one_block() {
        let mut flash = SoftFlash::new(FakeController::new());
        flash.start_erase(addr(ERASE_SIZE), block()).unwrap();
        let stored = &flash.controller.flash;
        assert_eq!(stored[..ERASE_SIZE], [0; ERASE_SIZE]);
        assert_eq!(stored[ERASE_SIZE..2 * ERASE_SIZE], [0xff; ERASE_SIZE]);
        assert_eq!(stored[2 * ERASE_SIZE..], [0; 2 * ERASE_SIZE]);
    }

    #[test]
    fn rejects_bad_requests() {
        let mut flash = SoftFlash::new(FakeController::new());
        let size = FakeController::SIZE.get();
        let out_of_bounds = Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
        assert_eq!(flash.read(addr(size - 1), &mut [0; 2]), out_of_bounds);
        assert_eq!(flash.start_program(addr(size), &[0; 1]), out_of_bounds);
        assert_eq!(flash.start_erase(addr(size), block()), out_of_bounds);
        assert_eq!(
            flash.start_program(addr(0), &[0xff; PAGE_SIZE + 1]),
            Err(error::FLASH_GENERIC_PROGRAM_EXCEEDS_WINDOW_SIZE)
        );
        assert_eq!(
            flash.start_program(addr(0x1ff), &[0xff; 2]),
            Err(error::FLASH_GENERIC_PROGRAM_SPANS_WINDOW_BOUNDARY)
        );
        assert_eq!(
            flash.start_erase(addr(0), PowerOf2Usize::new(PAGE_SIZE).unwrap()),
            Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE)
        );
        assert_eq!(
            flash.start_erase(addr(PAGE_SIZE), block()),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        assert_eq!(flash.controller.flash, [0; 4 * ERASE_SIZE]);
    }
}
