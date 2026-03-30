// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]

use kernel::sync::spinlock::SpinLock;
use pw_status::Result;

struct Uart;

impl Uart {
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let tx = core::ptr::with_exposed_provenance_mut::<u8>(0x1000_1041);
        for &byte in buf.iter() {
            unsafe {
                tx.write_volatile(byte);
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
