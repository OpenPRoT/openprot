// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use codegen as _;
use console_backend as _;
use cortex_m_semihosting::debug::{EXIT_FAILURE, EXIT_SUCCESS, exit};
use entry as _;
use target_common::{TargetInterface, declare_target};

pub struct Target {}

// Must match the IRQ number in system.json5.
const TEST_IRQ: u32 = 42;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel Interrupts";

    fn main() -> ! {
        let exit_status = match test_interrupts::main::<Arch>(TEST_IRQ) {
            Ok(()) => EXIT_SUCCESS,
            Err(_e) => EXIT_FAILURE,
        };
        exit(exit_status);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

codegen::declare_kernel_interrupt_handlers!();
declare_target!(Target);
