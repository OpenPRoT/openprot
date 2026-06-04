// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_peripherals::i2c;
use ast10x0_peripherals::scu::pinctrl;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 MCTP Server";

    fn main() -> ! {
        // SAFETY: kernel main() runs once with exclusive hardware ownership.
        unsafe {
            let scu = ast10x0_peripherals::scu::ScuRegisters::new_global_unlocked();
            scu.ungate_clock_mask(ast10x0_peripherals::scu::ClockRegisterHalf::Lower, 1 << 2);
            scu.assert_reset_mask(ast10x0_peripherals::scu::ScuRegisterHalf::Upper, 1 << 2);
            for _ in 0..10_000u32 {
                core::hint::spin_loop();
            }
            scu.deassert_reset_mask(ast10x0_peripherals::scu::ScuRegisterHalf::Upper, 1 << 2);
            for _ in 0..10_000u32 {
                core::hint::spin_loop();
            }
            scu.apply_pinctrl_group(pinctrl::PINCTRL_I2C2);
            i2c::init_i2c_global();
        }

        codegen::start();
        loop {}
    }

    fn shutdown(code: u32) -> ! {
        let sentinel: &[u8] = if code == 0 {
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
