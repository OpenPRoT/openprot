// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_peripherals::sgpiom::{
    Bank, BankDevice, Direction, InitialLevel, InterruptMode, InterruptTrigger, Sgpiom,
    SgpiomBankPort, SgpiomMask, SgpiomPinConfig,
};
use console_backend::console_backend_write_all;
use openprot_hal_blocking::gpio_port::{
    ActivePolarity, EdgeSensitivity, GpioBankPassthrough, GpioInterrupt, GpioPort,
    InterruptOperation, PinConfig, PinDirection,
};
use target_common::{declare_target, TargetInterface};
use {codegen as _, entry as _};

pub struct Target {}

fn run_smoke_test() -> bool {
    pw_log::info!("=== AST10x0 SGPIOM smoke test ===");

    // SAFETY: The test owns SGPIOM access for its runtime.
    let sgpiom = unsafe { Sgpiom::new_global() };

    if sgpiom.configure_global(64, 8).is_err() {
        pw_log::error!("configure_global failed");
        return false;
    }

    if sgpiom.configure_global(0, 8).is_ok() {
        pw_log::error!("configure_global accepted invalid ngpios");
        return false;
    }

    // SAFETY: The test owns SGPIOM access for its runtime.
    let regs = unsafe { &*ast1060_pac::Sgpiom::ptr() };
    let global_cfg = regs.gpio554().read().bits();
    if (global_cfg & 0x1) == 0 {
        pw_log::error!("global enable bit not set");
        return false;
    }

    let dev = match BankDevice::from_pin_offset(0, 16) {
        Some(d) => d,
        None => {
            pw_log::error!("invalid BankDevice descriptor");
            return false;
        }
    };

    if sgpiom
        .configure_pin(
            &dev,
            1,
            SgpiomPinConfig {
                direction: Direction::Output,
                initial: Some(InitialLevel::High),
                pull_up: false,
                pull_down: false,
            },
        )
        .is_err()
    {
        pw_log::error!("configure_pin failed");
        return false;
    }

    // Verify the driven output via the output-latch readback, not port_get_raw
    // (the Data Value register reads sampled *input*, not the last driven value).
    if (sgpiom.read_output_latch(Bank::Ad) & (1 << 1)) == 0 {
        pw_log::error!("configure_pin did not set pin high");
        return false;
    }

    if sgpiom
        .configure_interrupt(&dev, 2, InterruptMode::Edge, InterruptTrigger::Both)
        .is_err()
    {
        pw_log::error!("configure_interrupt failed");
        return false;
    }

    let int_en = regs.gpio504().read().bits();
    let sens0 = regs.gpio508().read().bits();
    let sens1 = regs.gpio50c().read().bits();
    let sens2 = regs.gpio510().read().bits();
    if (int_en & (1 << 2)) == 0
        || (sens2 & (1 << 2)) == 0
        || (sens0 & (1 << 2)) != 0
        || (sens1 & (1 << 2)) != 0
    {
        pw_log::error!("interrupt config register mismatch");
        return false;
    }

    if sgpiom
        .configure_pin(
            &dev,
            31,
            SgpiomPinConfig {
                direction: Direction::Input,
                initial: None,
                pull_up: false,
                pull_down: false,
            },
        )
        .is_ok()
    {
        pw_log::error!("configure_pin accepted out-of-range pin");
        return false;
    }

    // SAFETY: The test owns SGPIOM access for its runtime.
    let hal_sgpiom = unsafe { Sgpiom::new_global() };
    // SAFETY: Same ownership applies to this bank wrapper.
    let mut bank_port = unsafe { SgpiomBankPort::new(hal_sgpiom, dev) };

    let configure_result = bank_port.configure(
        SgpiomMask((1 << 3) | (1 << 4)),
        PinConfig {
            direction: PinDirection::Output,
            polarity: ActivePolarity::ActiveHigh,
            initial_output: Some(true),
        },
    );
    if configure_result.is_err() {
        pw_log::error!("HAL configure failed");
        return false;
    }

    if bank_port
        .set_reset(SgpiomMask(1 << 5), SgpiomMask(1 << 3))
        .is_err()
    {
        pw_log::error!("HAL set_reset failed");
        return false;
    }

    if bank_port.toggle(SgpiomMask(1 << 4)).is_err() {
        pw_log::error!("HAL toggle failed");
        return false;
    }

    // Verify the driven output via the output-latch readback (not port_get_raw,
    // which reads sampled input). set_reset drove bit5 high / bit3 low. Checked
    // here BEFORE passthrough, which would overwrite bit5 with sampled input.
    let latched = sgpiom.read_output_latch(Bank::Ad);
    if (latched & (1 << 5)) == 0 || (latched & (1 << 3)) != 0 {
        pw_log::error!("HAL operations produced unexpected output state");
        return false;
    }

    if bank_port.set_passthrough_mask(SgpiomMask(1 << 5)).is_err() {
        pw_log::error!("HAL passthrough failed");
        return false;
    }

    if bank_port.clear_passthrough().is_err() {
        pw_log::error!("HAL clear_passthrough failed");
        return false;
    }

    // HAL GpioInterrupt: configure sensitivity, then enable/query/clear/disable.
    let irq_mask = SgpiomMask(1 << 6);
    if bank_port
        .irq_configure(irq_mask, EdgeSensitivity::RisingEdge)
        .is_err()
    {
        pw_log::error!("HAL irq_configure failed");
        return false;
    }
    // RisingEdge => Edge+High => sens code 1 => sens0 set, sens1/sens2 clear.
    let s0 = regs.gpio508().read().bits();
    let s1 = regs.gpio50c().read().bits();
    let s2 = regs.gpio510().read().bits();
    if (s0 & (1 << 6)) == 0 || (s1 & (1 << 6)) != 0 || (s2 & (1 << 6)) != 0 {
        pw_log::error!("HAL irq_configure sensitivity mismatch");
        return false;
    }

    if bank_port.irq_control(irq_mask, InterruptOperation::Enable) != Ok(true) {
        pw_log::error!("HAL irq enable failed");
        return false;
    }
    if (regs.gpio504().read().bits() & (1 << 6)) == 0 {
        pw_log::error!("HAL irq enable did not set int_en");
        return false;
    }

    // IsPending must not error (status value is environment-dependent).
    if bank_port
        .irq_control(irq_mask, InterruptOperation::IsPending)
        .is_err()
    {
        pw_log::error!("HAL irq IsPending failed");
        return false;
    }
    if bank_port.irq_control(irq_mask, InterruptOperation::Clear) != Ok(true) {
        pw_log::error!("HAL irq clear failed");
        return false;
    }
    if bank_port.irq_control(irq_mask, InterruptOperation::Disable) != Ok(true) {
        pw_log::error!("HAL irq disable failed");
        return false;
    }
    if (regs.gpio504().read().bits() & (1 << 6)) != 0 {
        pw_log::error!("HAL irq disable did not clear int_en");
        return false;
    }

    // Callback registration is unsupported in the microkernel model.
    if bank_port
        .register_interrupt_handler(irq_mask, |_m: SgpiomMask| {})
        .is_ok()
    {
        pw_log::error!("HAL register_interrupt_handler unexpectedly succeeded");
        return false;
    }

    // Exercise the read_input HAL path (returns sampled input; value is
    // environment-dependent, so only check it does not error).
    if bank_port.read_input().is_err() {
        pw_log::error!("HAL read_input failed");
        return false;
    }

    pw_log::info!("=== AST10x0 SGPIOM smoke test complete ===");
    true
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SGPIOM smoke test";

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
