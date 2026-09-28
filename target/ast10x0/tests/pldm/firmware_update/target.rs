// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::aperture::take_aperture;
use ast10x0_peripherals::gpio::{GpioBlock, IntoGpio, OutputPin};
use ast10x0_peripherals::scu::{self, create_pins, pinctrl};
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target;

static PINCTRL_GROUPS: [&[ast10x0_peripherals::scu::PinctrlPin]; 2] =
    [pinctrl::PINCTRL_I2C2, pinctrl::PINCTRL_FMC_QUAD];

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 PLDM firmware update";

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

        // GPIOK5: the fd image's reset-passthrough request line to the Pi harness.
        // SAFETY: sole pin creation site in this binary, at boot; the pins! table is this chip's true pin map.
        let pins = unsafe { create_pins() };
        // SAFETY: kernel-only binary, minted once; no process holds a conflicting grant.
        let gpio = GpioBlock::new(unsafe { take_aperture() });
        let gpio_k5 = pins.scu418_21.into_gpio(&gpio);
        scu::route(&gpio_k5);
        // Driven low (deasserted) immediately so the line has a defined level for
        // the whole test instead of floating until fd_main.rs binds it as output.
        let mut gpio_k5 = gpio_k5.into_output();
        let _ = gpio_k5.set_low();

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
