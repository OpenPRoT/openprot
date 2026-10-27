// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO interrupt bring-up - kernel side.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{pinctrl, ScuRegisters};
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 GPIO IRQ Bringup";

    fn main() -> ! {
        // SAFETY: single call at boot with exclusive access to SCU global registers.
        unsafe {
            let scu = ScuRegisters::new_global_unlocked();
            scu.apply_pinctrl_group(pinctrl::PINCTRL_GPIOA0);
        }

        codegen::start();
        #[expect(clippy::empty_loop)]
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
