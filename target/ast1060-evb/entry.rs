// Licensed under the Apache-2.0 license

//! Entry point for ASPEED AST1060-EVB target.

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;

use core::ptr::{self, addr_of, write_volatile};

/// Configure cache to exclude non-cached RAM region for DMA buffers.
///
/// The AST1060 cache controller (SCUA50) uses 24KB granularity per bit.
/// Setting bits 0-25 caches 0x00000-0x9BFFF (624KB), leaving 0x9C000+
/// uncached for DMA buffers. This minimizes waste compared to the
/// aspeed-ddk reference (which uses bits 0-19, wasting 160KB).
///
/// # Safety
///
/// Must be called from pre_init before RAM is initialized.
unsafe fn configure_cache() {
    // Cache controller registers (SCU base = 0x7e6e_2000)
    const CACHE_CTRL: u32 = 0x7e6e_2a58;  // SCUA58: Function Control
    const CACHE_AREA: u32 = 0x7e6e_2a50;  // SCUA50: Cacheable Area (24KB/bit)
    const CACHE_INVAL: u32 = 0x7e6e_2a54; // SCUA54: Invalidation Control

    // SAFETY: Called from pre_init before RAM initialization.
    // These registers control the cache controller and must be configured
    // before DMA operations to prevent cache coherency issues.
    unsafe {
        // Disable cache
        write_volatile(CACHE_CTRL as *mut u32, 0);

        // Configure cache area: bits 0-25 set = 26 × 24KB = 624KB cached
        // Covers 0x00000 - 0x9BFFF (cached), leaves 0x9C000+ uncached
        // This wastes only 16KB vs aspeed-ddk's 160KB waste
        write_volatile(CACHE_AREA as *mut u32, 0x03ff_ffff);

        // Invalidate cache
        write_volatile(CACHE_INVAL as *mut u32, 0x81e0_0000);

        // Re-enable cache
        write_volatile(CACHE_CTRL as *mut u32, 1);
    }
}

/// Pre-kernel hardware initialization
/// Runs before RAM is initialized, before main()
#[cortex_m_rt::pre_init]
unsafe fn pre_kernel_init() {
    // SAFETY: Called once at boot before RAM initialization.
    // Configure cache to exclude RAM_NC region (0xA0000+) for DMA buffers
    unsafe { configure_cache(); }

    #[cfg(feature = "jtag-halt")]
    {
        // Enable JTAG pins via SCU pinmux - must happen very early
        // Scu::steal() is safe here: it's a zero-sized type with no RAM allocation
        let scu = unsafe { ast1060_pac::Scu::steal() };

        // SCU41C: Multi-function Pin Control - enable ARM JTAG pins
        scu.scu41c().modify(|_, w| {
            w.enbl_armtmsfn_pin()
                .bit(true)
                .enbl_armtckfn_pin()
                .bit(true)
                .enbl_armtrstfn_pin()
                .bit(true)
                .enbl_armtdifn_pin()
                .bit(true)
                .enbl_armtdofn_pin()
                .bit(true)
        });

        // Halt here waiting for JTAG debugger
        // Break with JTAG and set HALT to 0 to continue
        static mut HALT: u32 = 1;
        loop {
            let val = unsafe { ptr::read_volatile(addr_of!(HALT)) };
            if val == 0 {
                break;
            }
        }
    }
}

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

// Default stub handlers for peripherals not yet implemented
default_handler!(
    fmc, gpio, hace,
    i2c, i2c1, i2c2, i2c3, i2c4, i2c5, i2c6, i2c7, i2c8, i2c9, i2c10, i2c11, i2c12, i2c13,
    i2cfilter,
    i3c, i3c1, i3c2, i3c3,
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

/// Initialize I2C subsystem
///
/// This must be called once before any I2C controller is used.
/// Sets up global I2C registers and pin muxing for I2C1 and I2C2.
fn i2c_init() {
    // 1. Initialize I2C global registers (reset, clock dividers)
    //    - Asserts/de-asserts I2C reset via SCU050/SCU054
    //    - Configures I2CG0C global control register
    //    - Sets I2CG10 base clock dividers for all speed modes
    aspeed_ddk::i2c_core::init_i2c_global();

    // 2. Configure I2C pin muxing
    aspeed_ddk::pinctrl::Pinctrl::apply_pinctrl_group(aspeed_ddk::pinctrl::PINCTRL_I2C1);
    aspeed_ddk::pinctrl::Pinctrl::apply_pinctrl_group(aspeed_ddk::pinctrl::PINCTRL_I2C2);
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

        // Initialize I2C1 for master mode operations
        i2c_init();
        let _ = console_backend::console_backend_write_all(b"I2C1 initialized\r\n");

        kernel::main(Arch, &mut INIT_STATE)
    };
}
