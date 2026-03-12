// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use app_test_uart_listener::{handle, signals};
use pw_status::{Error, Result};
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{entry, syscall};

use registers::uart::*;

fn handle_interrupt(uart: &mut Uart1, interrupts: Signals) -> Result<()> {
    if !interrupts.contains(signals::UART1_RX_WATERMARK) {
        pw_log::error!(
            "Interrupt on wrong signal. {} not in {}",
            signals::UART1_RX_WATERMARK.bits() as u32,
            interrupts.bits() as u32
        );
        return Err(Error::FailedPrecondition);
    }

    let regs = uart.regs_mut();
    if regs.status().read().rxempty() {
        pw_log::error!("No data to read");
        return Err(Error::FailedPrecondition);
    }
    let value = u32::from(regs.rdata().read());

    let _ = syscall::interrupt_ack(handle::UART_INTERRUPTS, interrupts);

    const SEND_BUF_LEN: usize = 1;
    const RECV_BUF_LEN: usize = 0;
    let mut send_buf = [0u8; SEND_BUF_LEN];
    let mut recv_buf = [0u8; RECV_BUF_LEN];

    send_buf[0] = value as u8;
    let len = syscall::channel_transact(handle::IPC, &send_buf, &mut recv_buf, Instant::MAX)?;
    if len != RECV_BUF_LEN {
        pw_log::error!(
            "Received {} bytes, {} expected",
            len as usize,
            RECV_BUF_LEN as usize
        );
        return Err(Error::OutOfRange);
    }
    Ok(())
}

fn wait_for_interrupts() -> Result<()> {
    let mut uart1 = unsafe { Uart1::new() };
    {
        let regs = uart1.regs_mut();
        // Enable RX watermark interrupt with a high watermark of 1 byte.
        regs.intr_enable().modify(|en| en.rx_watermark(true));
        regs.fifo_ctrl()
            .modify(|fifo| fifo.rxilvl(|lvl| lvl.rxlvl1()));
    }
    loop {
        let wait_return = syscall::object_wait(
            handle::UART_INTERRUPTS,
            signals::UART1_RX_WATERMARK,
            Instant::MAX,
        )?;
        if !wait_return
            .pending_signals
            .contains(signals::UART1_RX_WATERMARK)
            || wait_return.user_data != 0
        {
            pw_log::error!("Incorrect WaitReturn values");
            return Err(Error::Internal);
        } else {
            handle_interrupt(&mut uart1, wait_return.pending_signals)?;
        }
    }
}

#[entry]
fn entry() -> ! {
    // Since this is written as a test, shut down with the return status from `main()`.
    let ret = wait_for_interrupts();
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
