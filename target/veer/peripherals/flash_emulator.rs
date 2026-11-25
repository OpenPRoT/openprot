// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The flash controllers of the Caliptra MCU emulator.

use core::num::NonZero;

use caliptra_ss_registers::primary_flash_ctrl::bits::{CtrlRegwen, FlInterruptState};
use caliptra_ss_registers::primary_flash_ctrl::PRIMARY_FLASH_CTRL_ADDR;
use caliptra_ss_registers::secondary_flash_ctrl::SECONDARY_FLASH_CTRL_ADDR;
use util_error::{self as error, ErrorCode};

use crate::{
    page_range, Mmio, MmioBlock, Op, PageController, Reg, FLASH_VEER_OP_FAILED, FLASH_VEER_TIMEOUT,
    PAGE_SIZE, POLL_LIMIT,
};

/// One of the emulator's flash controllers.
pub struct EmulatorFlashCtrl<M: Mmio> {
    mmio: M,
    /// The page buffer. The controller reads and writes it without the
    /// compiler's knowledge, so the driver only accesses it with volatile
    /// operations.
    page: [u8; PAGE_SIZE],
}

impl EmulatorFlashCtrl<MmioBlock> {
    /// # Safety
    ///
    /// The primary flash controller's registers must be mapped for the caller,
    /// and no other code on the MCU may access them.
    pub unsafe fn primary() -> Self {
        // SAFETY: the address is the controller's and is a multiple of 4. The
        // caller guarantees the rest of `MmioBlock::new`'s contract.
        Self::new(unsafe { MmioBlock::new(PRIMARY_FLASH_CTRL_ADDR as usize) })
    }

    /// # Safety
    ///
    /// The secondary flash controller's registers must be mapped for the
    /// caller, and no other code on the MCU may access them.
    pub unsafe fn secondary() -> Self {
        // SAFETY: the address is the controller's and is a multiple of 4. The
        // caller guarantees the rest of `MmioBlock::new`'s contract.
        Self::new(unsafe { MmioBlock::new(SECONDARY_FLASH_CTRL_ADDR as usize) })
    }
}

impl<M: Mmio> EmulatorFlashCtrl<M> {
    /// The controller copies pages to and from the page buffer in the returned
    /// value, and fails an operation if the buffer is not in MCU RAM. The
    /// driver gives the controller the buffer's address at the start of each
    /// operation, so the value can be moved between operations.
    pub fn new(mut mmio: M) -> Self {
        // The driver polls for completion, so disable the interrupts.
        mmio.write(Reg::FlInterruptEnable, 0);
        // Clear the status of any earlier operation. FL_INTERRUPT_STATE is
        // write-one-to-clear.
        mmio.write(Reg::OpStatus, 0);
        mmio.write(
            Reg::FlInterruptState,
            (FlInterruptState::Error::SET + FlInterruptState::Event::SET).into(),
        );
        Self {
            mmio,
            page: [0; PAGE_SIZE],
        }
    }

    fn run(&mut self, op: Op, page: u32) -> Result<(), ErrorCode> {
        // The controller clears CTRL_REGWEN when an operation starts, and
        // ignores writes to FL_CONTROL until the driver acknowledges that
        // operation by clearing FL_INTERRUPT_STATE.
        if !CtrlRegwen::En.is_set(self.mmio.read(Reg::CtrlRegwen)) {
            return Err(error::FLASH_GENERIC_BUSY);
        }
        self.mmio.write(Reg::FlControl, 0);
        self.mmio.write(Reg::PageNum, page);
        self.mmio.write(
            Reg::PageAddr,
            (&raw mut self.page).expose_provenance() as u32,
        );
        self.mmio.write(Reg::PageSize, PAGE_SIZE as u32);
        self.mmio.write(Reg::FlControl, op.start());

        // On a read error the controller sets OP_STATUS.ERR to 0, which is
        // also its value on success, so the driver uses FL_INTERRUPT_STATE to
        // detect completion and failure.
        for _ in 0..POLL_LIMIT {
            let state = self.mmio.read(Reg::FlInterruptState);
            let failed = FlInterruptState::Error.is_set(state);
            if failed || FlInterruptState::Event.is_set(state) {
                // The controller does not clear OP_STATUS itself.
                self.mmio.write(Reg::OpStatus, 0);
                // Acknowledge the operation. The controller then sets
                // CTRL_REGWEN again.
                self.mmio.write(Reg::FlInterruptState, state);
                return if failed {
                    Err(FLASH_VEER_OP_FAILED)
                } else {
                    Ok(())
                };
            }
        }
        Err(FLASH_VEER_TIMEOUT)
    }
}

impl<M: Mmio> PageController for EmulatorFlashCtrl<M> {
    // `DummyFlashCtrl::MAX_PAGES * PAGE_SIZE` in caliptra-mcu-sw.
    const SIZE: NonZero<usize> = NonZero::new(64 * 1024 * 1024).unwrap();
    // `DummyFlashCtrl::ERASE_SECTOR_SIZE` in caliptra-mcu-sw.
    const ERASE_OP_SIZE: usize = 4096;

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
        let page = (&raw const self.page).cast::<u8>();
        for (byte, index) in out.iter_mut().zip(range) {
            // SAFETY: `page_range` checked that `index` is inside `self.page`.
            *byte = unsafe { page.add(index).read_volatile() };
        }
        Ok(())
    }

    fn program_buffer(&mut self, offset: usize, data: &[u8]) -> Result<(), ErrorCode> {
        let range = page_range(offset, data.len())?;
        let page = (&raw mut self.page).cast::<u8>();
        for (&byte, index) in data.iter().zip(range) {
            // SAFETY: `page_range` checked that `index` is inside `self.page`.
            unsafe {
                let cell = page.add(index);
                cell.write_volatile(cell.read_volatile() & byte);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Registers that read as zero and ignore writes.
    struct NoMmio;

    impl Mmio for NoMmio {
        fn read(&self, _reg: Reg) -> u32 {
            0
        }

        fn write(&mut self, _reg: Reg, _value: u32) {}
    }

    #[test]
    fn page_buffer_round_trip() {
        let mut controller = EmulatorFlashCtrl::new(NoMmio);
        controller.page = [0xff; PAGE_SIZE];
        let data = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];
        controller.program_buffer(250, &data).unwrap();
        controller.program_buffer(250, &[0x0f]).unwrap();
        let mut buf = [0; 8];
        controller.read_buffer(248, &mut buf).unwrap();
        assert_eq!(buf, [0xff, 0xff, 0x02, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
    }

    #[test]
    fn page_buffer_rejects_ranges_outside_the_page() {
        let mut controller = EmulatorFlashCtrl::new(NoMmio);
        let out_of_bounds = Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
        assert_eq!(controller.read_buffer(PAGE_SIZE - 1, &mut [0; 1]), Ok(()));
        assert_eq!(
            controller.read_buffer(PAGE_SIZE - 1, &mut [0; 2]),
            out_of_bounds
        );
        assert_eq!(
            controller.read_buffer(usize::MAX, &mut [0; 2]),
            out_of_bounds
        );
        assert_eq!(controller.program_buffer(PAGE_SIZE, &[0]), out_of_bounds);
    }
}
