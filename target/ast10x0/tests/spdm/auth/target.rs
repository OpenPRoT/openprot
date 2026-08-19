// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::scu::pinctrl;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

static PINCTRL_GROUPS: [&[ast10x0_peripherals::scu::PinctrlPin]; 1] = [pinctrl::PINCTRL_I2C2];

pub struct Target;

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SPDM Auth Stress Test";

    fn main() -> ! {
        // SAFETY: kernel main() runs once with exclusive hardware ownership.
        if unsafe {
            Ast10x0Board::new(Ast10x0BoardDescriptor {
                pinctrl_groups: &PINCTRL_GROUPS,
            })
            .init()
        }
        .is_err()
        {
            loop {}
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
