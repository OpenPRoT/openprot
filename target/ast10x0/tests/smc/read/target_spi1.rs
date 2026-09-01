// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC SPI1 read smoke test target.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{
    pinctrl::{PINCTRL_SPI1_QUAD, PINCTRL_SPIM1_DEFAULT},
    ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{
    FlashConfig, SmcConfig, SmcController, SmcError, SmcInstance, SmcTopology, SpiNorFlash,
    SpiNorFlashDevice, SpiTransaction, SpiUninit,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::{dump_smc_read, dump_smc_register};

/// Compile-time SPI1 descriptor: single CS0 device at 50 MHz on the host-SPI
/// interface, geometry discovered over SFDP at init.
struct Spi1Instance;

impl SmcInstance for Spi1Instance {
    const CONTROLLER: SmcController = SmcController::Spi1;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: None,
        dma_enabled: true,
        enable_interrupts: false,
        topology: SmcTopology::HostSpi { master_idx: 0 },
    };
}

pub struct Target {}

fn config_spi1_master_controller() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SPIM1_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPI1_QUAD);
    // Configure the mux for the SPI master controller path.
    scu.set_spim_internal_mux(SpiMonitorSource::Spi1, 1)
        .map_err(|_| SmcError::HardwareError)?;
    scu.set_spim_internal_master_route(SpiMonitorInstance::Spim0, SpiMonitorSource::Spi1);
    scu.set_spim_passthrough(SpiMonitorInstance::Spim0, SpiMonitorPassthrough::Enabled);
    scu.set_spim_ext_mux(SpiMonitorInstance::Spim0, ScuExtMuxSelect::Mux1);
    pw_log::info!("SCU pinmux and SPIM routing configured for SPI1 monitoring");
    Ok(())
}

fn run_spi1_read_test() -> Result<(), SmcError> {
    config_spi1_master_controller()?;

    pw_log::info!("=== AST10x0 SMC SPI1 read test ===");
    let mut spi = unsafe { SpiUninit::<Spi1Instance>::new()? }.init()?;

    if !spi.is_ready() {
        return Err(SmcError::HardwareError);
    }

    let jedec = {
        let flash = SpiNorFlash::new(spi.cs0()?)?;
        flash.jedec_id()?
    };
    pw_log::info!(
        "SPI1 CS0 JEDEC ID: {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );

    pw_log::info!("=== SPI1 controller register ===");
    dump_smc_register(0x7E63_0000, 16);
    dump_smc_register(0x7E63_0080, 16);
    pw_log::info!("=== SCU QSPI Mux routing register ===");
    dump_smc_register(0x7E6E_20F0, 4);
    pw_log::info!("=== SPI1 controller/window ===");
    dump_smc_register(0x9000_0000, 16);

    pw_log::info!("=== SPI1 read ===");
    let mut buf = [0u8; 64];
    let n = SpiTransaction::read_with_spim(spi.cs0()?, SpiMonitorInstance::Spim0, 0x0, &mut buf)?;
    if n != buf.len() {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&buf, buf.len() as u32);

    pw_log::info!("=== SPI1 DMA read @ 0x00000000 ===");
    let dma_buf = unsafe { core::slice::from_raw_parts_mut(0x41500 as *mut u8, 256) };
    let mut dma_txn = SpiTransaction::dma_read_with_spim(
        spi.cs0()?,
        SpiMonitorInstance::Spim0,
        0x0,
        0x41500usize,
        dma_buf.len() as u32,
    )?;

    loop {
        match dma_txn.poll_dma_completion() {
            core::task::Poll::Pending => {}
            core::task::Poll::Ready(result) => {
                result?;
                break;
            }
        }
    }
    dump_smc_read(dma_buf, dma_buf.len() as u32);

    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SMC SPI1 read Test";

    fn main() -> ! {
        let sentinel = if run_spi1_read_test().is_ok() {
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
