// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]

use kernel::sync::spinlock::SpinLock;
use pw_status::Result;

/// FPGA wrapper debug FIFO push register (`dbg_fifo_push`), absolute address
/// `0xA401_1014`. Writing a byte with bit 8 set pushes it into the FIFO the
/// ARM PS host drains for console output. Matches
/// `caliptra-mcu-sw/platforms/fpga/rom/src/io.rs::FPGA_UART_OUTPUT`, which
/// uses the identical mechanism on the same board.
#[cfg(feature = "fpga")]
const FPGA_DBG_FIFO_PUSH: *mut u32 = core::ptr::without_provenance_mut(0xA401_1014);

#[cfg(feature = "fpga")]
const FPGA_CHAR_VALID: u32 = 0x100;

struct Uart;

impl Uart {
    #[cfg(not(feature = "fpga"))]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let tx = core::ptr::with_exposed_provenance_mut::<u8>(0x1000_1041);
        for &byte in buf.iter() {
            unsafe {
                tx.write_volatile(byte);
            }
        }
        Ok(())
    }

    #[cfg(feature = "fpga")]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        for &byte in buf.iter() {
            unsafe {
                FPGA_DBG_FIFO_PUSH.write_volatile(byte as u32 | FPGA_CHAR_VALID);
            }
        }
        Ok(())
    }
}

static UART: SpinLock<arch_riscv::Arch, Uart> = SpinLock::new(Uart);

#[unsafe(no_mangle)]
pub fn console_backend_write_all(buf: &[u8]) -> Result<()> {
    let mut uart = UART.lock(arch_riscv::Arch);
    uart.write_all(buf)
}
