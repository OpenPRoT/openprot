// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SGPIOM interrupt bring-up — kernel side.
//!
//! Performs kernel-owned early platform init for the SGPIOM userspace app:
//! applies the SGPIOM pin mux (SCU41C[8:11]). SGPIOM is clocked by PCLK, which
//! is already running, so no clock ungate or controller reset is needed (the
//! Zephyr driver likewise only reads PCLK to compute the serial divider).
//!
//! All interrupt handling happens in userspace via the wait-on-object syscall
//! model (see `sgpiom_irq_server_main.rs`); the kernel only brings up pins and
//! starts the apps.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{pinctrl, ScuRegisters};
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SGPIOM IRQ Bringup";

    fn main() -> ! {
        // SAFETY: single call at boot with exclusive access to SCU global regs.
        unsafe {
            let scu = ScuRegisters::new_global_unlocked();
            scu.apply_pinctrl_group(pinctrl::PINCTRL_SGPIOM);
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
