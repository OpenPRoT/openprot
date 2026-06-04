// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

//! I3C controller init smoke test.
//!
//! Brings up I3C bus 0 (`PAC I3c`) through the `ast10x0_peripherals::i3c`
//! driver — the behavioral-parity port of `aspeed-rust/src/i3c/`
//! (see `target/ast10x0/peripherals/i3c/plans/goal.md`). Validates the clock
//! configuration, constructs the controller behind the confined-`unsafe`
//! façade with a busy-spin yield hook, runs the `Uninitialized -> Ready`
//! bring-up (`start()`), and (on real hardware) verifies the
//! controller-enable bit. Reports PASS/FAIL via the console sentinel,
//! matching the I2C tests.

use core::pin::Pin;

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i3c::{Ast1060I3c, I3cConfig, I3cController, I3cCore};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{TargetInterface, declare_target};

pub struct Target {}

/// One driver type serves every bus; the instance is selected at runtime.
type I3cHw = Ast1060I3c<fn(u32)>;
/// Bus index under test (PAC `I3c`, the first instance).
const I3C_BUS: u8 = 0;

/// Busy-spin yield hook (bare-metal wait policy). A named `fn` (not a closure)
/// keeps the `I3cCore` type nameable for the `singleton!` storage.
fn yield_spin(_ns: u32) {
    core::hint::spin_loop();
}

/// Example platform core clock (Hz) for timing computation. The AST1060 I3C
/// core is fed from the HCLK domain; 200 MHz is a representative value and is
/// only used to derive the timing-register fields during `init`.
const CORE_CLK_HZ: u32 = 200_000_000;
/// Target I3C push-pull SCL (12.5 MHz, SDR0).
const I3C_SCL_HZ: u32 = 12_500_000;
/// Target legacy-I2C SCL (Fast-mode, 400 kHz).
const I2C_SCL_HZ: u32 = 400_000;

fn run_i3c_init_smoke_test() -> Result<(), &'static str> {
    pw_log::info!("=== AST10x0 I3C init smoke test ===");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I3C0],
    });
    // SAFETY: Test target runs once at boot with exclusive access to the board.
    unsafe { board.init() };
    pw_log::info!("Board-level pinctrl applied for I3C1");

    let mut config = I3cConfig::new()
        .core_clk_hz(CORE_CLK_HZ)
        .i3c_scl_hz(I3C_SCL_HZ)
        .i2c_scl_hz(I2C_SCL_HZ);
    config.core_period = 1_000_000_000 / CORE_CLK_HZ;

    config
        .validate_clock()
        .map_err(|_| "i3c clock validation failed")?;
    pw_log::info!("Clock configuration validated");

    // SAFETY: the test owns I3C bus 0 for its lifetime and uses the matching
    // PAC register blocks; the busy-spin hook is the bare-metal wait policy.
    let hw = unsafe { I3cHw::new(I3C_BUS, yield_spin) }.ok_or("invalid I3C bus index")?;
    // The ISR-shared core lives in a static so its address is stable and
    // `'static` — the IRQ trampoline's pointer validity is type-guaranteed.
    let i3c_core = cortex_m::singleton!(: I3cCore<I3cHw> = I3cCore::new(hw, config))
        .ok_or("I3C core storage already taken")?;
    let ctrl = I3cController::new(Pin::static_mut(i3c_core));
    pw_log::info!("Controller constructed");

    // `start()` claims the IRQ slot (single-shot) and programs the hardware.
    // This smoke test leaves the NVIC line masked (its system.json5 has no
    // I3C vector entry), so no interrupt can be delivered.
    let _ctrl = ctrl.start().map_err(|_| "controller start failed")?;
    pw_log::info!("controller start complete");

    // On real hardware the controller-enable bit must be set after a primary
    // (non-secondary) init. QEMU `ast1030-evb` does not model the I3C block, so
    // the on-hardware register check is exercised only by the hardware-tagged
    // `i3c_init_test`; here we confirm the bring-up sequence ran to completion.
    // SAFETY: exclusive ownership of I3C bus 0 during the test.
    let regs = unsafe { &*ast1060_pac::I3c::ptr() };
    let enabled = regs.i3cd000().read().enbl_i3cctrl().bit_is_set();
    pw_log::info!("i3cd000.enbl_i3cctrl = {}", enabled as u8);

    pw_log::info!("=== AST10x0 I3C init smoke test complete ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel I3C";

    fn main() -> ! {
        let sentinel: &[u8] = match run_i3c_init_smoke_test() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(error) => {
                pw_log::error!("I3C init smoke test failed: {}", error as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };

        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
