// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The flash controllers of the Caliptra MCU FPGA.
//!
//! The controllers' registers are in the FPGA wrapper, which is a block of
//! registers that the MCU and the board's ARM cores can both access.
//! caliptra-sw's `ModelFpgaSubsystem` runs on the ARM cores (the host). A
//! program on the host calls its `step` function in a loop, and each call
//! runs `handle_flash`, which does the work of the controllers. The flash
//! contents are in host memory, the page buffer is part of the registers, and
//! an erase clears one page.

use core::num::NonZero;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::primary_flash_ctrl::bits::OpStatus;
use util_error::{self as error, ErrorCode};

use crate::{
    page_range, Mmio, MmioBlock, Op, PageController, Reg, FLASH_VEER_OP_FAILED, FLASH_VEER_TIMEOUT,
    PAGE_SIZE, POLL_LIMIT,
};

/// Address of the primary flash controller's registers on the MCU's bus. The
/// registers are in the FPGA wrapper. The address is from caliptra-mcu-sw
/// `platforms/fpga/rom/src/flash/flash_drv.rs`.
pub const FPGA_PRIMARY_FLASH_CTRL_ADDR: usize = 0xA401_2000;

/// Address of the secondary flash controller's registers on the MCU's bus,
/// from the same file.
pub const FPGA_SECONDARY_FLASH_CTRL_ADDR: usize = 0xA401_3000;

/// Offset of the FPGA controller's page buffer from the controller's base
/// address. The buffer is 64 little-endian words.
const FLASH_BUF_OFFSET: usize = 0x100;

/// A word of the FPGA controller's page buffer. The value is the word's offset
/// from the start of the buffer. The offset is a multiple of 4 and less than
/// [`PAGE_SIZE`].
#[derive(Clone, Copy)]
pub struct BufWord(usize);

impl BufWord {
    /// Returns the word that holds the byte at `offset` in the page, or `None`
    /// if `offset` is outside the page.
    pub const fn containing(offset: usize) -> Option<Self> {
        if offset < PAGE_SIZE {
            Some(Self(offset & !3))
        } else {
            None
        }
    }
}

/// Access to the registers of an FPGA controller, which include the page
/// buffer.
pub trait FpgaMmio: Mmio {
    /// Reads `word` of the page buffer.
    fn read_buf(&self, word: BufWord) -> u32;
    /// Writes `value` to `word` of the page buffer.
    fn write_buf(&mut self, word: BufWord, value: u32);
}

/// An FPGA controller's memory-mapped registers.
pub struct FpgaMmioBlock {
    regs: MmioBlock,
}

impl FpgaMmioBlock {
    /// # Safety
    ///
    /// `base` must be the address of an FPGA flash controller's registers,
    /// which are 0x200 bytes at a multiple of 4 and include the page buffer.
    /// They must be mapped for the caller, and no other code on the MCU may
    /// access them.
    pub const unsafe fn new(base: usize) -> Self {
        // SAFETY: the caller upholds the contract of `MmioBlock::new`, because
        // `MmioBlock` accesses only the first 0x20 bytes of the registers at
        // `base`.
        Self {
            regs: unsafe { MmioBlock::new(base) },
        }
    }

    /// Returns the address of `word` of the page buffer.
    fn buf_addr(&self, word: BufWord) -> usize {
        self.regs
            .base
            .wrapping_add(FLASH_BUF_OFFSET)
            .wrapping_add(word.0)
    }
}

impl Mmio for FpgaMmioBlock {
    fn read(&self, reg: Reg) -> u32 {
        self.regs.read(reg)
    }

    fn write(&mut self, reg: Reg, value: u32) {
        self.regs.write(reg, value);
    }
}

impl FpgaMmio for FpgaMmioBlock {
    fn read_buf(&self, word: BufWord) -> u32 {
        // SAFETY: every `BufWord` is a word of the page buffer, which is inside
        // the registers that the caller of `new` guaranteed are mapped.
        unsafe { with_exposed_provenance::<u32>(self.buf_addr(word)).read_volatile() }
    }

    fn write_buf(&mut self, word: BufWord, value: u32) {
        // SAFETY: every `BufWord` is a word of the page buffer, which is inside
        // the registers that the caller of `new` guaranteed are mapped.
        unsafe { with_exposed_provenance_mut::<u32>(self.buf_addr(word)).write_volatile(value) }
    }
}

/// One of the FPGA's flash controllers.
///
/// It works only while a program on the host calls `ModelFpgaSubsystem::step`
/// in a loop. Without that program, operations fail with
/// [`FLASH_VEER_TIMEOUT`].
pub struct FpgaFlashCtrl<M: FpgaMmio> {
    mmio: M,
}

impl FpgaFlashCtrl<FpgaMmioBlock> {
    /// # Safety
    ///
    /// The primary flash controller's registers must be mapped for the caller,
    /// and no other code on the MCU may access them.
    pub unsafe fn primary() -> Self {
        // SAFETY: the address is the controller's and is a multiple of 4. The
        // caller guarantees the rest of `FpgaMmioBlock::new`'s contract.
        Self::new(unsafe { FpgaMmioBlock::new(FPGA_PRIMARY_FLASH_CTRL_ADDR) })
    }

    /// # Safety
    ///
    /// The secondary flash controller's registers must be mapped for the
    /// caller, and no other code on the MCU may access them.
    pub unsafe fn secondary() -> Self {
        // SAFETY: the address is the controller's and is a multiple of 4. The
        // caller guarantees the rest of `FpgaMmioBlock::new`'s contract.
        Self::new(unsafe { FpgaMmioBlock::new(FPGA_SECONDARY_FLASH_CTRL_ADDR) })
    }
}

impl<M: FpgaMmio> FpgaFlashCtrl<M> {
    pub fn new(mut mmio: M) -> Self {
        // The driver polls for completion, so disable the interrupts.
        mmio.write(Reg::FlInterruptEnable, 0);
        // Clear START and the status of any earlier operation.
        mmio.write(Reg::FlControl, 0);
        mmio.write(Reg::OpStatus, 0);
        Self { mmio }
    }

    fn run(&mut self, op: Op, page: u32) -> Result<(), ErrorCode> {
        // The host can read these registers at any time, so clear START
        // first, then write the page number, and set START last. OP_STATUS
        // can hold DONE from an operation that the host completed just after
        // the driver timed out, so clear it before starting.
        self.mmio.write(Reg::FlControl, 0);
        self.mmio.write(Reg::OpStatus, 0);
        self.mmio.write(Reg::PageNum, page);
        self.mmio.write(Reg::FlControl, op.start());

        // The host sets DONE when an operation ends, whether it succeeded or
        // failed, and sets ERR to a non-zero value when it failed. The host
        // does not use CTRL_REGWEN.
        for _ in 0..POLL_LIMIT {
            let status = self.mmio.read(Reg::OpStatus);
            if OpStatus::Done.is_set(status) {
                self.mmio.write(Reg::OpStatus, 0);
                return if OpStatus::Err.read(status) != 0 {
                    Err(FLASH_VEER_OP_FAILED)
                } else {
                    Ok(())
                };
            }
        }
        // Clear START so that a host that starts later does not run the
        // operation.
        self.mmio.write(Reg::FlControl, 0);
        Err(FLASH_VEER_TIMEOUT)
    }
}

impl<M: FpgaMmio> PageController for FpgaFlashCtrl<M> {
    // This is `DEFAULT_FLASH_SIZE` in caliptra-sw
    // `hw-model/src/model_fpga_subsystem.rs`. The wrapper also has a
    // FLASH_SIZE register, which resets to the same value. The host does not
    // update it, so the driver does not read it.
    const SIZE: NonZero<usize> = NonZero::new(16 * 1024 * 1024).unwrap();
    const ERASE_OP_SIZE: usize = PAGE_SIZE;

    fn load(&mut self, page: u32) -> Result<(), ErrorCode> {
        self.run(Op::Read, page)
    }

    fn store(&mut self, page: u32) -> Result<(), ErrorCode> {
        self.run(Op::Write, page)
    }

    fn erase(&mut self, page: u32) -> Result<(), ErrorCode> {
        self.run(Op::Erase, page)
    }

    fn read_buffer(&self, offset: usize, out: &mut [u8]) -> Result<(), ErrorCode> {
        let range = page_range(offset, out.len())?;
        let mut value = 0;
        for (byte, index) in out.iter_mut().zip(range) {
            // Read a buffer word when the loop reaches the first byte that it
            // needs from that word.
            if index == offset || index % 4 == 0 {
                let word =
                    BufWord::containing(index).ok_or(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)?;
                value = self.mmio.read_buf(word);
            }
            *byte = (value >> (8 * (index % 4))) as u8;
        }
        Ok(())
    }

    fn program_buffer(&mut self, offset: usize, data: &[u8]) -> Result<(), ErrorCode> {
        let range = page_range(offset, data.len())?;
        if range.is_empty() {
            return Ok(());
        }
        // Read, modify and write each buffer word that the range overlaps.
        for word_start in (range.start & !3..range.end).step_by(4) {
            let word =
                BufWord::containing(word_start).ok_or(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)?;
            let mut value = self.mmio.read_buf(word);
            for byte_in_word in 0..4 {
                // `get` returns `None` for the bytes of this word that are
                // outside `data`.
                let index = word_start + byte_in_word;
                if let Some(&byte) = data.get(index.wrapping_sub(offset)) {
                    let shift = 8 * byte_in_word;
                    value &= (u32::from(byte) << shift) | !(0xff << shift);
                }
            }
            self.mmio.write_buf(word, value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGS: [Reg; 8] = [
        Reg::FlInterruptState,
        Reg::FlInterruptEnable,
        Reg::PageSize,
        Reg::PageNum,
        Reg::PageAddr,
        Reg::FlControl,
        Reg::OpStatus,
        Reg::CtrlRegwen,
    ];

    #[test]
    fn mmio_blocks_access_each_register_at_its_offset() {
        // An FPGA controller's registers, as words.
        let mut words = [0u32; 0x200 / 4];
        let base = (&raw mut words).expose_provenance();
        // SAFETY: `words` is 0x200 bytes at a multiple of 4, and only `block`
        // accesses them until its last use.
        let mut block = unsafe { FpgaMmioBlock::new(base) };
        for (i, reg) in REGS.into_iter().enumerate() {
            block.write(reg, 0x100 + i as u32);
            assert_eq!(block.read(reg), 0x100 + i as u32);
        }
        for offset in (0..PAGE_SIZE).step_by(4) {
            // Any byte of a word selects the word.
            let word = BufWord::containing(offset + 3).unwrap();
            block.write_buf(word, 0x200 + offset as u32);
            assert_eq!(block.read_buf(word), 0x200 + offset as u32);
        }

        // The registers are the first 8 words, and the page buffer starts at
        // offset 0x100.
        for (i, word) in words[..8].iter().enumerate() {
            assert_eq!(*word, 0x100 + i as u32);
        }
        assert_eq!(words[8..0x40], [0; 0x38]);
        for (i, word) in words[0x40..].iter().enumerate() {
            assert_eq!(*word, 0x200 + 4 * i as u32);
        }
    }

    #[test]
    fn buf_word_rejects_offsets_outside_the_page() {
        assert!(BufWord::containing(PAGE_SIZE - 1).is_some());
        assert!(BufWord::containing(PAGE_SIZE).is_none());
        assert!(BufWord::containing(usize::MAX).is_none());
    }
}
