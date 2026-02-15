// Licensed under the Apache-2.0 license

//! AST1060-EVB UART console backend.
//!
//! Implements console output using the AST1060 UART peripheral via aspeed-ddk.

#![no_std]

use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

use pw_status::{Error, Result};
use ast1060_pac::Peripherals;
use embedded_io::Write;

use aspeed_ddk::uart::{Config, Parity, StopBits, UartController};

// Global UART controller instance
static mut UART_CONTROLLER: MaybeUninit<UartController<'static>> = MaybeUninit::uninit();
static UART_INITIALIZED: AtomicBool = AtomicBool::new(false);

struct DummyDelay;

impl embedded_hal::delay::DelayNs for DummyDelay {
    fn delay_ns(&mut self, _ns: u32) {
        // Simple spin loop since we don't have a reliable timer yet
        core::hint::spin_loop();
    }
}

static mut DELAY: DummyDelay = DummyDelay;

/// Initializes the UART console backend.
///
/// # Safety
///
/// This function must be called only once during kernel initialization.
/// It initializes the global UART controller.
#[unsafe(no_mangle)]
pub unsafe fn console_backend_init() {
    if UART_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    // Use steal() as recommended for aspeed-rust to avoid singleton check issues
    let peripherals = unsafe { Peripherals::steal() };

    let config = Config {
        baud_rate: 115200,
        word_length: 3, // 3 means 8 bits (00=5, 01=6, 10=7, 11=8)
        parity: Parity::None,
        stop_bits: StopBits::One,
        clock: 24_000_000, // Assuming 24MHz clock
    };

    #[allow(static_mut_refs)]
    let delay = unsafe { &mut DELAY };
    let controller = UartController::new(peripherals.uart, delay);
    unsafe {
        controller.init(&config);
    }

    unsafe {
        let p = core::ptr::addr_of_mut!(UART_CONTROLLER);
        core::ptr::write(p as *mut UartController<'static>, controller);
    }
    UART_INITIALIZED.store(true, Ordering::Release);
}

#[unsafe(no_mangle)]
pub fn console_backend_write_all(buf: &[u8]) -> Result<()> {
    if !UART_INITIALIZED.load(Ordering::Acquire) {
        return Err(Error::Unavailable);
    }

    // Safety: logical singlton access pattern guarded by UART_INITIALIZED.
    // In a real multi-threaded kernel we'd need a spinlock here.
    let controller = unsafe {
        &mut *(core::ptr::addr_of_mut!(UART_CONTROLLER) as *mut UartController<'static>)
    };

    match controller.write(buf) {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::DataLoss),
    }
}
