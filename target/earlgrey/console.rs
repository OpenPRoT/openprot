// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]

use kernel::sync::spinlock::SpinLock;
use pw_status::Result;
use registers::uart;

struct Uart {
    device: uart::Uart0,
}

impl Uart {
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let reg = self.device.regs_mut();
        for &byte in buf.iter() {
            while reg.status().read().txfull() {
                // Wait while the FIFO is full.
            }
            reg.wdata().write(|w| w.wdata(byte as u32));
        }
        Ok(())
    }
}

static UART: SpinLock<arch_riscv::Arch, Uart> = SpinLock::new(Uart {
    device: unsafe { uart::Uart0::new() },
});

#[unsafe(no_mangle)]
pub fn console_backend_write_all(buf: &[u8]) -> Result<()> {
    let mut uart = UART.lock(arch_riscv::Arch);
    uart.write_all(buf)
}
