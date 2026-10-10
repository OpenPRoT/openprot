// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SGPIOM single-board loopback test.
//!
//! Requires a physical jumper wiring SGPIO PORT A OUT -> PORT A IN (fixture
//! J26/U22 output bits 0..7 -> J34-35/U25 input bits 0..7). With the loop closed
//! the controller can drive its own bank-A outputs and observe them back on the
//! bank-A inputs after a serial scan, so this exercises real edge *delivery*
//! (not just IRQ configuration).
//!
//! Sequence: arm both-edge IRQ on bits 0..7, drive high, confirm the inputs
//! mirror the outputs and rising edges latched, then drive low and confirm the
//! inputs clear and falling edges latched. Finally drive asymmetric patterns
//! (single bit, nibble) to pin down bit order through the shift-register chain,
//! since the uniform 0x00/0xff levels above are palindromes and cannot reveal a
//! reversal on their own.
//!
//! Config matches Zephyr's live SGPIOM state (ngpios=128 -> numbers=16, clock
//! divider 24) so the full serial daisy chain is clocked.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{pinctrl::PINCTRL_SGPIOM, ScuRegisters};
use ast10x0_peripherals::sgpiom::{
    Bank, BankDevice, InterruptMode, InterruptTrigger, Sgpiom, SgpiomBankPort, SgpiomMask,
};
use console_backend::console_backend_write_all;
use openprot_hal_blocking::gpio_port::{ActivePolarity, GpioPort, PinConfig, PinDirection};
use target_common::{declare_target, TargetInterface};
use {codegen as _, entry as _};

/// Controller total SGPIO count (Zephyr DTS uses 128 -> numbers = 16 bytes).
const NGPIOS: u16 = 128;
/// Per-bank pin count for the descriptor (bank max 32).
const BANK_NGPIOS: u8 = 32;
/// Serial clock divider (matches Zephyr live config 0x554 division = 24).
const CLOCK_DIV: u16 = 24;
/// PORT A pins 0..7 — driven outputs, jumpered back to the watched inputs.
const MASK: u32 = 0x0000_00ff;
/// Busy-wait iterations between output changes. Sized to comfortably exceed one
/// 128-bit serial scan at divider 24 so a level change reaches the far end of
/// the shift-register chain and is sampled back before we read.
const SETTLE_SPINS: u32 = 8_000_000;

pub struct Target {}

/// Busy-wait long enough for one full serial scan to propagate out -> jumper ->
/// in. Kernel-mode targets have no `debug_clock_now` syscall, so spin.
fn settle() {
    for _ in 0..SETTLE_SPINS {
        core::hint::spin_loop();
    }
}

/// Reverse the low 8 bits — the expected input if the shift-register chain
/// mirrors bit order (out bit N arrives as in bit 7-N).
fn reverse8(v: u32) -> u32 {
    ((v & 0xff) as u8).reverse_bits() as u32
}

/// Drive `pattern` on the bank-A output bits within `MASK` (bits in `pattern`
/// high, the rest of `MASK` low), wait one full serial scan, and return the
/// sampled input bits (masked to bits 0..7).
fn drive_and_sample(port: &mut SgpiomBankPort, sgpiom: &Sgpiom, pattern: u32) -> u32 {
    // set_reset never errors for an in-range mask; the mask is a compile-time
    // constant subset of one bank.
    let _ = port.set_reset(SgpiomMask(pattern), SgpiomMask(MASK & !pattern));
    settle();
    sgpiom.port_get_raw(Bank::Ad) & MASK
}

fn run_loopback_test() -> bool {
    pw_log::info!("=== AST10x0 SGPIOM loopback test ===");

    // Route the four SGPIO-master pins (CK/LD/DO/DI) to the package. Without
    // this the serial engine shifts internally but never clocks the external
    // 594/165 chain, so inputs read a constant 0.
    // SAFETY: The test owns SCU access during early setup.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SGPIOM);

    // SAFETY: The test owns SGPIOM access for its runtime.
    let sgpiom = unsafe { Sgpiom::new_global() };
    if sgpiom.configure_global(NGPIOS, CLOCK_DIV).is_err() {
        pw_log::error!("configure_global failed");
        return false;
    }

    // BankDevice derives its bank from the pin offset; offset 0 => Bank::Ad.
    let Some(dev) = BankDevice::from_pin_offset(0, BANK_NGPIOS) else {
        pw_log::error!("invalid bank descriptor");
        return false;
    };

    // Second handle drives outputs through the HAL bank port; the first handle
    // above is used for interrupt config and status/input reads.
    // SAFETY: The test owns SGPIOM access for its runtime.
    let hal_sgpiom = unsafe { Sgpiom::new_global() };
    // SAFETY: Same ownership applies to this bank wrapper.
    let mut port = unsafe { SgpiomBankPort::new(hal_sgpiom, dev) };

    // Mark bits 0..7 as outputs, driven low to start (direction is HW-managed;
    // this validates the mask and sets a known initial level).
    if port
        .configure(
            SgpiomMask(MASK),
            PinConfig {
                direction: PinDirection::Output,
                polarity: ActivePolarity::ActiveHigh,
                initial_output: Some(false),
            },
        )
        .is_err()
    {
        pw_log::error!("configure (output) failed");
        return false;
    }

    // Arm both-edge edge interrupts on each watched pin.
    for pin in 0..8u8 {
        if sgpiom
            .configure_interrupt(&dev, pin, InterruptMode::Edge, InterruptTrigger::Both)
            .is_err()
        {
            pw_log::error!("configure_interrupt failed on pin {}", pin as u32);
            return false;
        }
    }

    // Settle the initial low level, then clear any startup edge so the status
    // we check below reflects only the transitions we drive.
    settle();
    sgpiom.clear_interrupt_status(Bank::Ad, MASK);

    // Drive high and let the level loop back through the jumper.
    if port.set_reset(SgpiomMask(MASK), SgpiomMask(0)).is_err() {
        pw_log::error!("set_reset (high) failed");
        return false;
    }
    settle();
    let din_hi = sgpiom.port_get_raw(Bank::Ad);
    let ist_hi = sgpiom.interrupt_status(Bank::Ad);
    pw_log::info!(
        "drive high: din=0x{:08x} int_status=0x{:08x}",
        din_hi as u32,
        ist_hi as u32
    );
    if (din_hi & MASK) != MASK {
        pw_log::error!(
            "loopback not closed on high: din&mask=0x{:08x}, expected 0x{:08x}",
            (din_hi & MASK) as u32,
            MASK as u32
        );
        return false;
    }
    if (ist_hi & MASK) != MASK {
        pw_log::error!(
            "rising edge missing: int_status&mask=0x{:08x}, expected 0x{:08x}",
            (ist_hi & MASK) as u32,
            MASK as u32
        );
        return false;
    }
    // The output latch must reflect the driven level; port_get_raw is sampled
    // input, so a broken output RMW would only surface here (identity, no reversal).
    let lat_hi = sgpiom.read_output_latch(Bank::Ad);
    if (lat_hi & MASK) != MASK {
        pw_log::error!(
            "output latch not high: latch&mask=0x{:08x}, expected 0x{:08x}",
            (lat_hi & MASK) as u32,
            MASK as u32
        );
        return false;
    }

    // Clear, then drive low and confirm the falling edge loops back.
    sgpiom.clear_interrupt_status(Bank::Ad, MASK);
    if port.set_reset(SgpiomMask(0), SgpiomMask(MASK)).is_err() {
        pw_log::error!("set_reset (low) failed");
        return false;
    }
    settle();
    let din_lo = sgpiom.port_get_raw(Bank::Ad);
    let ist_lo = sgpiom.interrupt_status(Bank::Ad);
    pw_log::info!(
        "drive low: din=0x{:08x} int_status=0x{:08x}",
        din_lo as u32,
        ist_lo as u32
    );
    if (din_lo & MASK) != 0 {
        pw_log::error!(
            "loopback not closed on low: din&mask=0x{:08x}, expected 0x0",
            (din_lo & MASK) as u32
        );
        return false;
    }
    if (ist_lo & MASK) != MASK {
        pw_log::error!(
            "falling edge missing: int_status&mask=0x{:08x}, expected 0x{:08x}",
            (ist_lo & MASK) as u32,
            MASK as u32
        );
        return false;
    }
    // Output latch must now read all-low for the driven bits (driven output, not
    // sampled input).
    let lat_lo = sgpiom.read_output_latch(Bank::Ad);
    if (lat_lo & MASK) != 0 {
        pw_log::error!(
            "output latch not low: latch&mask=0x{:08x}, expected 0x0",
            (lat_lo & MASK) as u32
        );
        return false;
    }

    // --- Bit-order integrity ---
    // The 0x00/0xff levels above are palindromes and pass regardless of bit
    // order. Drive asymmetric patterns to determine the actual mapping. A
    // reversed chain is acceptable hardware behavior, but must be reported and
    // internally consistent (a single fixed orientation, not scrambled).
    sgpiom.clear_interrupt_status(Bank::Ad, MASK);

    const SINGLE: u32 = 0x01;
    let in_single = drive_and_sample(&mut port, &sgpiom, SINGLE);
    pw_log::info!(
        "single-bit: out=0x{:02x} in=0x{:02x}",
        SINGLE as u32,
        in_single as u32
    );
    // Output latch reads the driven byte in identity order; an RMW that read
    // sampled (reversed) input instead would leave the wrong bit latched here.
    let lat_single = sgpiom.read_output_latch(Bank::Ad) & MASK;
    if lat_single != SINGLE {
        pw_log::error!(
            "output latch RMW wrong after single-bit: latch=0x{:02x} expected=0x{:02x}",
            lat_single as u32,
            SINGLE as u32
        );
        return false;
    }
    let reversed = if in_single == SINGLE {
        pw_log::info!("bit order: identity (out bit N -> in bit N)");
        false
    } else if in_single == reverse8(SINGLE) {
        pw_log::info!("bit order: reversed (out bit N -> in bit 7-N)");
        true
    } else {
        pw_log::error!(
            "single-bit scrambled: out=0x{:02x} in=0x{:02x} (neither identity 0x{:02x} nor reversed 0x{:02x})",
            SINGLE as u32,
            in_single as u32,
            SINGLE as u32,
            reverse8(SINGLE) as u32
        );
        return false;
    };

    const NIBBLE: u32 = 0x0f;
    let expect_nibble = if reversed { reverse8(NIBBLE) } else { NIBBLE };
    let in_nibble = drive_and_sample(&mut port, &sgpiom, NIBBLE);
    pw_log::info!(
        "nibble: out=0x{:02x} in=0x{:02x} expected=0x{:02x}",
        NIBBLE as u32,
        in_nibble as u32,
        expect_nibble as u32
    );
    if in_nibble != expect_nibble {
        pw_log::error!(
            "nibble mapping inconsistent with single-bit orientation: out=0x{:02x} in=0x{:02x} expected=0x{:02x}",
            NIBBLE as u32,
            in_nibble as u32,
            expect_nibble as u32
        );
        return false;
    }
    // Latch again reads the driven byte in identity order regardless of the input
    // chain orientation, pinning down the output RMW path.
    let lat_nibble = sgpiom.read_output_latch(Bank::Ad) & MASK;
    if lat_nibble != NIBBLE {
        pw_log::error!(
            "output latch RMW wrong after nibble: latch=0x{:02x} expected=0x{:02x}",
            lat_nibble as u32,
            NIBBLE as u32
        );
        return false;
    }

    // Leave the hardware benign: drive outputs low, disable the armed IRQs, and
    // clear any latched status so a later test starts from a known state.
    let _ = port.set_reset(SgpiomMask(0), SgpiomMask(MASK));
    sgpiom.clear_interrupt_enable(Bank::Ad, MASK);
    sgpiom.clear_interrupt_status(Bank::Ad, MASK);

    pw_log::info!("=== AST10x0 SGPIOM loopback test complete ===");
    true
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SGPIOM loopback test";

    fn main() -> ! {
        let sentinel = if run_loopback_test() {
            b"TEST_RESULT:PASS\n"
        } else {
            // Most failure modes here mean the loopback path is open. Flag the
            // required fixture wiring as the likely cause after the specific error.
            pw_log::warn!(
                "loopback requires a jumper: SGPIO PORT A OUT bits 0..7 (fixture J26/U22) -> PORT A IN bits 0..7 (J34-35/U25); without it inputs never mirror outputs and this test cannot pass"
            );
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
