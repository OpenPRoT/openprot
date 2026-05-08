// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1060-EVB UART console backend.
//!
//! Implements console output using the AST10x0 peripheral USART driver.

#![no_std]

use ast10x0_peripherals::uart::Usart;
use ast1060_pac as device;
use embedded_io::Write;
use kernel::sync::spinlock::SpinLock;
use pw_status::{Error, Result};

/// MMIO base address of UART5 on the AST10x0 SoC (AST1060 TRM §28, Table 28-1).
const UART5_BASE: *const device::uart::RegisterBlock = 0x7e78_4000 as *const _;

// Global console lock to serialize UART register access.
static UART_LOCK: SpinLock<arch_arm_cortex_m::Arch, ()> = SpinLock::new(());

#[unsafe(no_mangle)]
pub fn console_backend_write_all(buf: &[u8]) -> Result<()> {
    let _lock = UART_LOCK.lock(arch_arm_cortex_m::Arch);
    // UART is configured by ROM/bootloader before firmware starts.
    let mut uart = unsafe { Usart::new_uninit(UART5_BASE) };
    uart.write_all(buf).map_err(|_| Error::DataLoss)
}
