// Licensed under the Apache-2.0 license

//! Entry point for ASPEED AST1060-EVB target.

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn pw_assert_HandleFailure() -> ! {
    use kernel::Arch as _;
    Arch::panic();
}

// ── Interrupt Handler Stubs ──
// These are required by the ast1060-pac's __INTERRUPTS vector table.

macro_rules! default_handler {
    ($($name:ident),*) => {
        $(
            #[unsafe(no_mangle)]
            pub extern "C" fn $name() {
                // Default: infinite loop
                loop {}
            }
        )*
    };
}

// Handlers that have real implementations in aspeed-ddk
#[unsafe(no_mangle)]
pub extern "C" fn i3c() {
    aspeed_ddk::i3c::ast1060_i3c::i3c_irq_handler();
}

#[unsafe(no_mangle)]
pub extern "C" fn i3c1() {
    aspeed_ddk::i3c::ast1060_i3c::i3c1_irq_handler();
}

#[unsafe(no_mangle)]
pub extern "C" fn i3c2() {
    aspeed_ddk::i3c::ast1060_i3c::i3c2_irq_handler();
}

#[unsafe(no_mangle)]
pub extern "C" fn i3c3() {
    aspeed_ddk::i3c::ast1060_i3c::i3c3_irq_handler();
}

// Default stub handlers for peripherals not yet implemented
default_handler!(
    fmc, gpio, hace,
    i2c, i2c1, i2c2, i2c3, i2c4, i2c5, i2c6, i2c7, i2c8, i2c9, i2c10, i2c11, i2c12, i2c13,
    i2cfilter,
    scu, sgpiom,
    spi, spi1, spipf1, spipf2, spipf3,
    timer, timer1, timer2, timer3, timer4, timer5, timer6, timer7,
    uart, uartdma, wdt
);

mod console_backend {
    unsafe extern "Rust" {
        pub fn console_backend_init();
        pub fn console_backend_write_all(buf: &[u8]) -> pw_status::Result<()>;
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    kernel::static_init_state!(static mut INIT_STATE: InitKernelState<Arch>);

    // SAFETY: `main` is only executed once, so we never generate more than one
    // `&mut` reference to `INIT_STATE`.
    #[allow(static_mut_refs)]
    unsafe {
        // Initialize UART console
        console_backend::console_backend_init();
        let _ = console_backend::console_backend_write_all(b"\r\nHello World!\r\n");
        let _ = console_backend::console_backend_write_all(b"ast1060 pigweed fw is running!\r\n");
        kernel::main(Arch, &mut INIT_STATE)
    };
}
