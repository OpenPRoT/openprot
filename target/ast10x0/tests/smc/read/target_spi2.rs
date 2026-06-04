// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC SPI2 read smoke test target.

#![no_std]
#![no_main]

use ast10x0_peripherals::scu::{
    pinctrl::{PINCTRL_SPI2_QUAD, PINCTRL_SPIM3_DEFAULT},
    ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, SmcTopology, SpiNorFlash,
    SpiNorFlashDevice, SpiTransaction, SpiUninit,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::{dump_smc_read, dump_smc_register};

const SPI_FLASH_CONFIG: FlashConfig = FlashConfig {
    capacity_mb: 32,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
    spi_clock_mhz: 50,
};

pub struct Target {}

//TODO: need to port this in GPIO peripherals.
//GPIOL2 and L3 should must be pull high before SPI2 Controller accessing flash.
//GPIOL2/L3 is controlling the power for level shifter, SPI2 data path may going
//  through level shifter, so the pmic enabling is required.
fn gpio_flash_power() {
    /*
    let peripherals = unsafe { Peripherals::steal() };
    let gpio = peripherals.gpio;
    let gpiol = gpiol::GPIOL::new(gpio).split();

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOL2);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOL3);
    let mut pl2 = gpiol.pl2.into_push_pull_output();
    pl2.set_high().unwrap();

    let mut pl3 = gpiol.pl3.into_push_pull_output();
    pl3.set_high().unwrap();*/
}

fn config_spi2_master_controller() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SPIM3_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPI2_QUAD);
    // Configure the mux for the SPI master controller path.
    scu.set_spim_internal_mux(SpiMonitorSource::Spi2, 3)
        .map_err(|_| SmcError::HardwareError)?;
    scu.set_spim_internal_master_route(SpiMonitorInstance::Spim2, SpiMonitorSource::Spi2);
    scu.set_spim_passthrough(SpiMonitorInstance::Spim2, SpiMonitorPassthrough::Enabled);
    scu.set_spim_ext_mux(SpiMonitorInstance::Spim2, ScuExtMuxSelect::Mux1);
    pw_log::info!("SCU pinmux and SPIM routing configured for SPI2 monitoring");
    Ok(())
}

fn run_spi2_read_test() -> Result<(), SmcError> {
    gpio_flash_power();
    config_spi2_master_controller()?;

    let config = SmcConfig {
        controller_id: SmcController::Spi2,
        cs0: Some(SPI_FLASH_CONFIG),
        cs1: None,
        dma_enabled: true,
        enable_interrupts: false,
        topology: SmcTopology::NormalSpi { master_idx: 2 },
    };

    pw_log::info!("=== AST10x0 SMC SPI2 read test ===");
    let spi = unsafe { SpiUninit::new(SmcController::Spi2, config)? };
    let mut spi = spi.init()?;
    spi.spi_nor_read_init(ChipSelect::Cs0)?;

    if !spi.is_ready() {
        return Err(SmcError::HardwareError);
    }

    let jedec = {
        let flash = SpiNorFlash::from_spi_cs(&mut spi, SPI_FLASH_CONFIG, ChipSelect::Cs0)?;
        flash.jedec_id()?
    };
    pw_log::info!(
        "SPI2 CS0 JEDEC ID: {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );

    pw_log::info!("=== SPI2 controller register ===");
    dump_smc_register(0x7E64_0000, 16);
    dump_smc_register(0x7E64_0080, 16);
    pw_log::info!("=== SCU QSPI Mux routing register ===");
    dump_smc_register(0x7E6E_20F0, 4);
    pw_log::info!("=== SPI2 controller/window ===");
    dump_smc_register(0xB000_0000, 16);

    pw_log::info!("=== SPI2 read ===");
    let mut buf = [0u8; 64];
    let n = SpiTransaction::read_with_spim(
        &mut spi,
        SpiMonitorInstance::Spim2,
        ChipSelect::Cs0,
        0x0,
        &mut buf,
    )?;
    if n != buf.len() {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&buf, buf.len() as u32);

    pw_log::info!("=== SPI2 DMA read @ 0x00000000 ===");
    let dma_buf = unsafe { core::slice::from_raw_parts_mut(0x41500 as *mut u8, 256) };
    let mut dma_txn = SpiTransaction::dma_read_with_spim(
        &mut spi,
        SpiMonitorInstance::Spim2,
        ChipSelect::Cs0,
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
    const NAME: &'static str = "AST10x0 SMC SPI2 read Test";

    fn main() -> ! {
        let sentinel = if run_spi2_read_test().is_ok() {
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
