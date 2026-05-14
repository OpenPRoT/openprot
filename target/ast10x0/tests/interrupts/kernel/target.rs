// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use arch_arm_cortex_m::Arch;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{TargetInterface, declare_target};

pub struct Target {}

// Must match the IRQ number in system.json5.
const TEST_IRQ: u32 = 42;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel Interrupts";

    fn main() -> ! {
        let sentinel: &[u8] = match test_interrupts::main::<Arch>(Arch, TEST_IRQ) {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(_e) => b"TEST_RESULT:FAIL\n",
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

codegen::declare_kernel_interrupt_handlers!();
declare_target!(Target);
