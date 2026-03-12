// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use app_test_uart::handle;
use pw_status::{Error, Result, StatusCode};
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{entry, syscall};

use registers::uart::*;

fn read_expected_value(expected_value: u8) -> Result<()> {
    // the UART listener responds on IPC with the value written to the UART.
    let wait_return = syscall::object_wait(handle::IPC, Signals::READABLE, Instant::MAX)?;

    if !wait_return.pending_signals.contains(Signals::READABLE) || wait_return.user_data != 0 {
        return Err(Error::Internal);
    }

    let mut buffer = [0u8; 1];
    let len = syscall::channel_read(handle::IPC, 0, &mut buffer)?;
    if len != 1 {
        return Err(Error::OutOfRange);
    };

    if buffer[0] != expected_value {
        pw_log::error!(
            "UART read() wrong value {} (expected {})",
            buffer[0] as u8,
            expected_value as u8
        );
        return Err(Error::Internal);
    }

    let response_buffer = [0u8; 0];
    syscall::channel_respond(handle::IPC, &response_buffer)?;

    Ok(())
}

fn test_uart_interrupts() -> Result<()> {
    let mut uart1 = unsafe { Uart1::new() };
    let regs = uart1.regs_mut();

    regs.ctrl()
        .modify(|ctrl| ctrl.tx(true).rx(true).slpbk(true).nco(0xffff));

    while !regs.status().read().rxempty() {
        let _ = regs.rdata();
    }

    for txval in 65..91 {
        regs.wdata().write(|w| w.wdata(txval as u32));
        read_expected_value(txval)?;
    }

    Ok(())
}

#[entry]
fn entry() -> ! {
    pw_log::info!("🔄 RUNNING");
    let ret = test_uart_interrupts();

    // Log that an error occurred so that the app that caused the shutdown is logged.
    if ret.is_err() {
        pw_log::error!("❌ FAILED: {}", ret.status_code() as u32);
    } else {
        pw_log::info!("✅ PASSED");
    }

    // Since this is written as a test, shut down with the return status from `main()`.
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
