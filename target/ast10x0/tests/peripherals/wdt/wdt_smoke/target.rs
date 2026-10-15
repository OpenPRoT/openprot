// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_peripherals::wdt::{
    ResetMode, Watchdog, WdtConfig, WdtError, WdtRegisters, WDT_CLOCK_HZ,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {codegen as _, entry as _};

pub struct Target {}

fn run_smoke_test() -> bool {
    pw_log::info!("=== AST10x0 watchdog smoke test ===");

    // SAFETY: The test owns WDT0 access for its runtime.
    let mut wdt = Watchdog::new(unsafe { WdtRegisters::new_wdt0() });

    // A zero window cannot be programmed into the counter.
    if wdt.start(WdtConfig {
        timeout_ms: 0,
        reset_on_timeout: false,
        reset_mode: ResetMode::SocSystem,
    }) != Err(WdtError::InvalidTimeout)
    {
        pw_log::error!("zero timeout was not rejected");
        return false;
    }

    // Arm a 1s status-only watchdog (no hardware reset in QEMU).
    if wdt
        .start(WdtConfig {
            timeout_ms: 1000,
            reset_on_timeout: false,
            reset_mode: ResetMode::SocSystem,
        })
        .is_err()
    {
        pw_log::error!("start failed");
        return false;
    }

    // SAFETY: The test owns WDT0 access for its runtime.
    let regs = unsafe { &*ast1060_pac::Wdt::ptr() };

    let expected_ticks = WDT_CLOCK_HZ / 1000 * 1000;
    if regs.wdt004().read().bits() != expected_ticks {
        pw_log::error!("reload value not programmed");
        return false;
    }

    let ctrl = regs.wdt00c().read();
    if !ctrl.wdtenbl_sig().is_enable() {
        pw_log::error!("enable bit not set after start");
        return false;
    }
    if !ctrl.rst_sys_after_timeout().is_disable() {
        pw_log::error!("reset-on-timeout should be disabled");
        return false;
    }
    if !ctrl
        .rst_sys_mode()
        .is_soc_system_ewvergated_by_reset_mask_registers()
    {
        pw_log::error!("reset mode not SoC-system");
        return false;
    }

    if wdt.is_timeout() {
        pw_log::error!("timeout latched immediately after start");
        return false;
    }

    // Feeding must not fault and must leave the watchdog running.
    wdt.feed();
    if !regs.wdt00c().read().wdtenbl_sig().is_enable() {
        pw_log::error!("feed disturbed the enable bit");
        return false;
    }

    // Re-arm requesting a full-chip reset and confirm the mode bits.
    if wdt
        .start(WdtConfig {
            timeout_ms: 500,
            reset_on_timeout: true,
            reset_mode: ResetMode::FullChip,
        })
        .is_err()
    {
        pw_log::error!("re-arm failed");
        return false;
    }
    let ctrl = regs.wdt00c().read();
    if !ctrl.rst_sys_after_timeout().is_enable() || !ctrl.rst_sys_mode().is_full_chip() {
        pw_log::error!("reset configuration mismatch");
        return false;
    }

    // Disabling must clear the enable bit so no reset can fire.
    wdt.disable();
    if regs.wdt00c().read().wdtenbl_sig().is_enable() {
        pw_log::error!("disable did not clear the enable bit");
        return false;
    }

    pw_log::info!("=== AST10x0 watchdog smoke test complete ===");
    true
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 watchdog smoke test";

    fn main() -> ! {
        let sentinel = if run_smoke_test() {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
