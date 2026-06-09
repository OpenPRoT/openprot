// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SPI2 erase, write, verify, and restore test.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{
    pinctrl::{
        PINCTRL_GPIOL2, PINCTRL_GPIOL3, PINCTRL_SPI2_QUAD, PINCTRL_SPIM3_DEFAULT,
        PINCTRL_SPIM4_DEFAULT,
    },
    ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{SmcController, SmcError, SmcTopology};
use console_backend::console_backend_write_all;
use core::ptr::{read_volatile, write_volatile};
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

mod spi_write_common;
use spi_write_common::{new_spi, run_flash_test};

pub struct Target {}

fn gpio_flash_power() {
    const GPIO_DATA: *mut u32 = 0x7E78_0070 as *mut u32;
    const GPIO_DIR: *mut u32 = 0x7E78_0074 as *mut u32;
    const MASK: u32 = (1 << 26) | (1 << 27);

    unsafe {
        write_volatile(GPIO_DIR, read_volatile(GPIO_DIR) | MASK);
        write_volatile(GPIO_DATA, read_volatile(GPIO_DATA) | MASK);
    }
}

fn configure_spi2_external_mux(select_mux1: bool) {
    const SCU41C: *mut u32 = 0x7E6E_241C as *mut u32;
    const SGPIOM_PIN_MASK: u32 = 0xF << 8;
    const GPIO_DATA: *mut u32 = 0x7E78_0020 as *mut u32;
    const GPIO_DIR: *mut u32 = 0x7E78_0024 as *mut u32;
    const GPIO_E8: u32 = 1 << 8;
    const SGPIOM_DATA: *mut u32 = 0x7E78_0500 as *mut u32;
    const SGPIOM_CONFIG: *mut u32 = 0x7E78_0554 as *mut u32;
    const SGPIOM_LATCH: *const u32 = 0x7E78_0570 as *const u32;
    const SGPIOM_BIT: u32 = 1 << 2;
    const CONFIG_MASK: u32 = 1 | (0x1F << 6) | (0xFFFF << 16);
    const CONFIG: u32 = 1 | (16 << 6) | (24 << 16);

    unsafe {
        write_volatile(SCU41C, read_volatile(SCU41C) | SGPIOM_PIN_MASK);
        write_volatile(
            SGPIOM_CONFIG,
            (read_volatile(SGPIOM_CONFIG) & !CONFIG_MASK) | CONFIG,
        );

        let mut gpio = read_volatile(GPIO_DATA);
        let mut sgpio = read_volatile(SGPIOM_LATCH);
        if select_mux1 {
            gpio |= GPIO_E8;
            sgpio |= SGPIOM_BIT;
        } else {
            gpio &= !GPIO_E8;
            sgpio &= !SGPIOM_BIT;
        }
        write_volatile(GPIO_DATA, gpio);
        write_volatile(GPIO_DIR, read_volatile(GPIO_DIR) | GPIO_E8);
        write_volatile(SGPIOM_DATA, sgpio);
    }
}

fn configure_spi2() {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SPIM3_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPIM4_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPI2_QUAD);
    scu.apply_pinctrl_group(PINCTRL_GPIOL2);
    scu.apply_pinctrl_group(PINCTRL_GPIOL3);
    gpio_flash_power();
    configure_spi2_external_mux(true);
    scu.set_spim_internal_master_route(SpiMonitorInstance::Spim2, SpiMonitorSource::Spi2);
    scu.set_spim_passthrough(SpiMonitorInstance::Spim2, SpiMonitorPassthrough::Enabled);
    scu.set_spim_ext_mux(SpiMonitorInstance::Spim2, ScuExtMuxSelect::Mux1);
}

fn run_spi2_write_test() -> Result<(), SmcError> {
    pw_log::info!("=== AST10x0 SPI2 write and restore test ===");
    configure_spi2();
    let mut spi = new_spi(
        SmcController::Spi2,
        SmcTopology::NormalSpi { master_idx: 2 },
    )?;
    run_flash_test(&mut spi, SpiMonitorInstance::Spim2, 0xa5)
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SPI2 write and restore Test";

    fn main() -> ! {
        let sentinel = if run_spi2_write_test().is_ok() {
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
