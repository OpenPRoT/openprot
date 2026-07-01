// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SGPIOM output + interrupt configuration test (userspace).
//!
//! - Drives a static pattern on SGPIO_A 0..7 output pins.
//! - Arms both-edge interrupts on the same pins via the HAL `GpioInterrupt` trait.
//! - Verifies the `int_en` register matches the watch mask after arming.
//! - Calls `debug_shutdown(Ok(()))` → `TEST_RESULT:PASS` on success.
//!
//! Config matches Zephyr's live SGPIOM state (ngpios=128 -> numbers=16, clock
//! divider 24) so the full serial daisy chain is clocked.

#![no_main]
#![no_std]

use app_sgpiom_irq_server::{handle, signals};
use ast10x0_peripherals::sgpiom::{Bank, BankDevice, Sgpiom, SgpiomBankPort, SgpiomMask};
use openprot_hal_blocking::gpio_port::{
    ActivePolarity, EdgeSensitivity, GpioInterrupt, GpioPort, InterruptOperation, PinConfig,
    PinDirection,
};
use pw_status::Error;
use userspace::entry;
use userspace::syscall;

/// Controller total SGPIO count (Zephyr DTS uses 128 -> numbers = 16 bytes).
const NGPIOS: u16 = 128;
/// Per-bank pin count for the descriptor (bank max 32).
const BANK_NGPIOS: u8 = 32;
/// Serial clock divider (matches Zephyr live config 0x554 division = 24).
const CLOCK_DIV: u16 = 24;
/// Output pins: SGPIO_A 0..7 (the LEDs).
const OUT_MASK: u32 = 0x0000_00ff;
/// Static output pattern, pins 0..7 = 1,0,1,0,... (A0/A2/A4/A6 high).
const OUT_PATTERN: u32 = 0x0000_0055;
/// Watched input pins for interrupts (SGPIO_A 0..7).
const WATCH_MASK: u32 = 0x0000_00ff;

macro_rules! fail {
    ($($arg:tt)*) => {{
        pw_log::error!($($arg)*);
        let _ = syscall::debug_shutdown(Err(Error::Unknown));
        #[expect(clippy::empty_loop)]
        loop {}
    }};
}

#[entry]
fn entry() {
    // SAFETY: this process exclusively owns the SGPIOM device region mapped in
    // system.json5 (`sgpiom_regs`); `new_global` points at 0x7e780500.
    let sgpiom = unsafe { Sgpiom::new_global() };
    if sgpiom.configure_global(NGPIOS, CLOCK_DIV).is_err() {
        fail!("configure_global failed");
    }

    // Second handle (read-only register dumps / status reads).
    // SAFETY: same device region; reads are side-effect free.
    let monitor = unsafe { Sgpiom::new_global() };

    let Some(dev) = BankDevice::from_pin_offset(0, BANK_NGPIOS) else {
        fail!("invalid BankDevice descriptor");
    };
    // SAFETY: same ownership contract as `Sgpiom::new`.
    let mut port = unsafe { SgpiomBankPort::new(sgpiom, dev) };

    // OUTPUT: mark the blink pins as outputs (direction is HW-managed; validates mask).
    if port
        .configure(
            SgpiomMask(OUT_MASK),
            PinConfig {
                direction: PinDirection::Output,
                polarity: ActivePolarity::ActiveHigh,
                initial_output: None,
            },
        )
        .is_err()
    {
        fail!("configure (output) failed");
    }

    // OUTPUT: drive the static 1,0,1,0... pattern (0x55) once — set even pins,
    // reset odd pins within the output mask. Held for the rest of the run.
    if port
        .set_reset(SgpiomMask(OUT_PATTERN), SgpiomMask(OUT_MASK & !OUT_PATTERN))
        .is_err()
    {
        fail!("set_reset (output) failed");
    }

    // INPUT: read back the input state once output is driven, before arming IRQ.
    // SGPIO samples inputs serially over the daisy chain (clock divider 24), so
    // the data register is only valid after a full scan completes; the first
    // read after `configure_global` lands mid-scan and returns the reset value
    // (0). The HW serial engine runs independently of this thread, so a short
    // busy-wait on the monotonic clock lets it settle. (`sleep_until` returns
    // early here, so it does not actually delay.)
    busy_wait_ms(50);
    // Decode each watched pin so it's human-readable, not just a hex word.
    let data = monitor.port_get_raw(Bank::Ad);
    pw_log::info!("SGPIO input: data=0x{:08x}", data as u32);
    let mut watch = WATCH_MASK;
    while watch != 0 {
        let pin = watch.trailing_zeros();
        watch &= watch - 1;
        pw_log::info!(
            "  SGPIO_{}{} (pin {}) level={}",
            group_letter(pin) as &str,
            (pin % 8) as u32,
            pin as u32,
            ((data >> pin) & 1) as u32
        );
    }
    let state = monitor.dump_state(Bank::Ad);
    pw_log::info!(
        "SGPIO state: config=0x{:08x} data=0x{:08x} latch=0x{:08x} int_en=0x{:08x} int_status=0x{:08x}",
        state.config as u32,
        state.data as u32,
        state.latch as u32,
        state.int_en as u32,
        state.int_status as u32,
    );

    // INPUT/IRQ: both-edge sensitivity on the watched pins.
    if port
        .irq_configure(SgpiomMask(WATCH_MASK), EdgeSensitivity::BothEdges)
        .is_err()
    {
        fail!("irq_configure failed");
    }

    // Register the IRQ object with the wait group BEFORE enabling delivery.
    if syscall::wait_group_add(
        handle::WG,
        handle::SGPIOM_IRQ,
        signals::SGPIOM,
        handle::SGPIOM_IRQ as usize,
    )
    .is_err()
    {
        fail!("wait_group_add failed");
    }

    if port.irq_control(SgpiomMask(WATCH_MASK), InterruptOperation::Enable) != Ok(true) {
        fail!("irq enable failed");
    }
    if syscall::interrupt_ack(handle::SGPIOM_IRQ, signals::SGPIOM).is_err() {
        fail!("initial interrupt_ack failed");
    }

    pw_log::info!(
        "SGPIOM monitoring IRQ: out pins0..7=0x{:02x}, watch in=0x{:08x}",
        OUT_PATTERN as u32,
        WATCH_MASK as u32
    );

    // Verify IRQ registers were correctly programmed by reading back state.
    // int_en must have every watched pin enabled; int_status should be clear
    // (no spurious edge before we've driven anything). This validates the full
    // IRQ configuration path without requiring an external signal source.
    let armed = monitor.dump_state(Bank::Ad);
    pw_log::info!(
        "Armed state: int_en=0x{:08x} int_status=0x{:08x}",
        armed.int_en as u32,
        armed.int_status as u32,
    );
    if armed.int_en & WATCH_MASK != WATCH_MASK {
        fail!(
            "int_en mismatch: got 0x{:08x}, expected 0x{:08x}",
            (armed.int_en & WATCH_MASK) as u32,
            WATCH_MASK as u32
        );
    }
    pw_log::info!("PASS: IRQ configuration verified");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

/// Busy-wait `ms` milliseconds on the monotonic clock.
fn busy_wait_ms(ms: i64) {
    use userspace::time::Duration;
    let until = syscall::debug_clock_now() + Duration::from_millis(ms);
    while syscall::debug_clock_now() < until {}
}

/// SGPIO group letter for a bank-Ad pin index (A=0..7, B=8..15, C=16..23, D=24..31).
fn group_letter(pin: u32) -> &'static str {
    match pin / 8 {
        0 => "A",
        1 => "B",
        2 => "C",
        _ => "D",
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
