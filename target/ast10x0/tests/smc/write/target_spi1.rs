// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SPI1 erase, write, verify, and restore test.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{
    pinctrl::{PINCTRL_SPI1_QUAD, PINCTRL_SPIM1_DEFAULT},
    ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{SmcController, SmcError, SmcTopology};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

mod spi_write_common;
use spi_write_common::{new_spi, run_flash_test};

pub struct Target {}

fn configure_spi1() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SPIM1_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPI1_QUAD);
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, 1)
        .map_err(|_| SmcError::HardwareError)?;
    scu.set_spim_internal_master_route(SpiMonitorInstance::Spim0, SpiMonitorSource::Spi1);
    scu.set_spim_passthrough(SpiMonitorInstance::Spim0, SpiMonitorPassthrough::Enabled);
    scu.set_spim_ext_mux(SpiMonitorInstance::Spim0, ScuExtMuxSelect::Mux1);
    Ok(())
}

fn run_spi1_write_test() -> Result<(), SmcError> {
    pw_log::info!("=== AST10x0 SPI1 write and restore test ===");
    configure_spi1()?;
    let mut spi = new_spi(SmcController::Spi1, SmcTopology::HostSpi { master_idx: 0 })?;
    run_flash_test(&mut spi, SpiMonitorInstance::Spim0, 0x5a)
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SPI1 write and restore Test";

    fn main() -> ! {
        let sentinel = if run_spi1_write_test().is_ok() {
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
