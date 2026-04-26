#![no_std]

use core::num::NonZero;

use earlgrey_util::AsMubi;
use flash_ctrl_core::{self, regs::ControlWriteVal};
use hal_flash_driver::{FlashAddress, FlashDriver};
use util_error::{self as error, ErrorCode};
use util_regcpy::{copy_from_reg_unaligned, copy_to_reg_unaligned};

pub struct Permission {
    pub read: bool,
    pub write: bool,
    pub erase: bool,
}

impl Permission {
    pub const FULL_ACCESS: Permission = Permission {
        read: true,
        write: true,
        erase: true,
    };
    pub const READ_ONLY: Permission = Permission {
        read: true,
        write: false,
        erase: false,
    };
}

const FLASH_SIZE: NonZero<usize> = NonZero::new(1024 * 1024).unwrap();

pub struct EmbeddedFlash {
    mmio: flash_ctrl_core::FlashCtrl,
    busy: bool,
}
impl EmbeddedFlash {
    // TODO: unify these with the trait consts.
    const BYTES_PER_BANK: u32 = 0x80000;
    const BYTES_PER_PAGE: u32 = 2048;

    pub fn new(mmio: flash_ctrl_core::FlashCtrl) -> Self {
        Self { mmio, busy: false }
    }

    pub fn new_with_interrupts(mut mmio: flash_ctrl_core::FlashCtrl) -> Self {
        mmio.regs_mut().intr_state().write(|w| w.op_done_clear());
        mmio.regs_mut().intr_enable().write(|w| w.op_done(true));
        Self { mmio, busy: false }
    }

    pub fn set_default_permission(&mut self, perm: Permission) {
        let reg = self.mmio.regs_mut();
        reg.default_region().modify(|v| {
            v.rd_en(perm.read.as_mubi())
                .prog_en(perm.write.as_mubi())
                .erase_en(perm.erase.as_mubi())
        });
    }

    pub fn set_info_permission(
        &mut self,
        address: FlashAddress,
        perm: Permission,
    ) -> Result<(), ErrorCode> {
        if !address.is_info() {
            return Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
        }
        let reg = self.mmio.regs_mut();
        if address.bank() == 0 {
            reg.bank0_info0_page_cfg()
                .at(address.page() as usize)
                .modify(|v| {
                    v.en(true.as_mubi())
                        .rd_en(perm.read.as_mubi())
                        .rd_en(perm.read.as_mubi())
                        .prog_en(perm.write.as_mubi())
                        .erase_en(perm.erase.as_mubi())
                });
        } else {
            reg.bank1_info0_page_cfg()
                .at(address.page() as usize)
                .modify(|v| {
                    v.en(true.as_mubi())
                        .rd_en(perm.read.as_mubi())
                        .prog_en(perm.write.as_mubi())
                        .erase_en(perm.erase.as_mubi())
                });
        }
        Ok(())
    }
}

//fn u32_from_usize(val: usize) -> u32 {
//    u32::try_from(val).unwrap()
//}
impl FlashDriver for EmbeddedFlash {
    // BytesPerWord: 8
    // WordsPerPage: 256
    // BytesPerBank: 524288
    // program_resolution: 8 (max flash words to program at one time)
    // RegBusPgmResBytes = 64
    const PAGE_SIZE: usize = 2048;
    const PROGRAM_WINDOW_SIZE: usize = 64;
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 8;

    fn size(&self) -> core::num::NonZero<usize> {
        FLASH_SIZE
    }

    #[inline(never)]
    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        if buf.is_empty() {
            return Ok(());
        }
        let start_offset = if start_addr.is_info() {
            start_addr.bank() * Self::BYTES_PER_BANK
                + start_addr.page() * Self::BYTES_PER_PAGE
                + start_addr.offset()
        } else {
            start_addr.offset()
        };

        if (start_offset & 3) != 0 {
            return Err(error::FLASH_GENERIC_BAD_ALIGNMENT);
        }
        if buf.len() > Self::MAX_READ_SIZE {
            return Err(error::FLASH_GENERIC_READ_TOO_LONG);
        }

        self.check_busy()?;
        self.mmio.regs_mut().addr().write(|w| w.start(start_offset));
        self.start_op(|w| {
            w.op(|s| s.read())
                .partition_sel(start_addr.is_info())
                .num((buf.len() as u32 + 3) / 4 - 1)
        });
        copy_from_reg_unaligned(buf, &self.mmio.regs_mut().rd_fifo());
        while self.is_busy() {}
        self.complete_op()
    }

    fn start_erase_page(&mut self, start_addr: FlashAddress) -> Result<(), ErrorCode> {
        const {
            // Confirm this is a power of two
            assert!(Self::PAGE_SIZE.count_ones() == 1);
        }
        let start_offset = if start_addr.is_info() {
            start_addr.bank() * Self::BYTES_PER_BANK
                + start_addr.page() * Self::BYTES_PER_PAGE
                + start_addr.offset()
        } else {
            start_addr.offset()
        };
        let start_offset = start_offset as usize;

        if start_offset & (Self::PAGE_SIZE - 1) != 0 {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR);
        }
        self.check_busy()?;

        self.mmio
            .regs_mut()
            .addr()
            .write(|w| w.start(start_offset as u32));
        self.start_op(|w| {
            w.op(|s| s.erase())
                .erase_sel(|s| s.page_erase())
                .partition_sel(start_addr.is_info())
                .start(true)
        });
        Ok(())
    }

    fn start_program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode> {
        if data.is_empty() {
            return Ok(());
        }
        if data.len() > Self::PROGRAM_WINDOW_SIZE {
            return Err(error::FLASH_GENERIC_PROGRAM_EXCEEDS_WINDOW_SIZE);
        }
        let start_offset = if start_addr.is_info() {
            start_addr.bank() * Self::BYTES_PER_BANK
                + start_addr.page() * Self::BYTES_PER_PAGE
                + start_addr.offset()
        } else {
            start_addr.offset()
        };
        let start_offset = start_offset as usize;
        let end_offset = start_offset.wrapping_add(data.len());

        if start_offset / Self::PROGRAM_WINDOW_SIZE != (end_offset - 1) / Self::PROGRAM_WINDOW_SIZE
        {
            return Err(error::FLASH_GENERIC_PROGRAM_SPANS_WINDOW_BOUNDARY);
        }
        self.check_busy()?;
        // reset the op status register

        self.mmio
            .regs_mut()
            .addr()
            .write(|w| w.start(start_offset as u32));
        self.start_op(|w| {
            w.op(|s| s.prog())
                .prog_sel(|s| s.normal_program())
                .partition_sel(start_addr.is_info())
                .num(((data.len() + 3) / 4) as u32 - 1)
        });
        copy_to_reg_unaligned(&self.mmio.regs_mut().prog_fifo(), data);
        Ok(())
    }

    fn is_busy(&mut self) -> bool {
        if self.busy && self.mmio.regs_mut().intr_state().read().op_done() {
            self.busy = false;
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.op_done_clear());
        }
        self.busy
    }

    fn complete_op(&mut self) -> Result<(), ErrorCode> {
        if self.is_busy() {
            return Err(error::FLASH_GENERIC_BUSY);
        }
        let status = self.mmio.regs().op_status().read();
        if status.err() {
            let err_code = u32::from(self.mmio.regs().err_code().read());
            Err(error::FLASH_OPENTITAN.error(err_code as u16))
        } else {
            Ok(())
        }
    }
}
impl EmbeddedFlash {
    fn start_op(&mut self, f: impl FnOnce(ControlWriteVal) -> ControlWriteVal) {
        self.busy = true;
        self.mmio.regs_mut().err_code().write(|_| 0xff.into());
        self.mmio.regs_mut().op_status().write(|w| w);
        self.mmio
            .regs_mut()
            .intr_state()
            .write(|w| w.op_done_clear());
        self.mmio.regs_mut().control().write(|w| f(w).start(true));
    }
    fn check_busy(&mut self) -> Result<(), ErrorCode> {
        if self.is_busy() {
            Err(error::FLASH_GENERIC_BUSY)
        } else {
            Ok(())
        }
    }
}
