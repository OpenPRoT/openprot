// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Kernel target for the PLDM update QEMU test.
//!
//! The firmware device reports the result with
//! `syscall::debug_shutdown(Ok(()) | Err(..))`, which lands here and writes
//! the UART sentinel qemu_runner.py greps for.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 PLDM update test";

    fn main() -> ! {
        // The firmware device stages into SPI NOR on CS1, so the pins have
        // to be muxed before any process runs. Doing it here keeps SCU
        // access out of userspace, where two tasks could race the shared
        // pinctrl registers.
        //
        // SAFETY: kernel main() runs once, single-threaded, with exclusive
        // hardware ownership.
        let scu = unsafe { ScuRegisters::new_global_unlocked() };
        scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

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
