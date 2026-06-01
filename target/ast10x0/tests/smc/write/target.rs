// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC FMC CS1 erase/write/read verify test target.

#![no_std]
#![no_main]

#[allow(unused_imports)]
use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use ast10x0_peripherals::smc::{
    ChipSelect, FlashConfig, FmcUninit, SmcConfig, SmcController, SmcError, SmcTopology,
    SpiNorFlash, SpiNorFlashDevice,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::{dump_smc_read, dump_smc_register};

const CS0_CONFIG: FlashConfig = FlashConfig {
    capacity_mb: 8,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
    spi_clock_mhz: 50,
};

const CS1_CONFIG: FlashConfig = FlashConfig {
    capacity_mb: 64,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
    spi_clock_mhz: 50,
};

const TEST_OFFSET: u32 = 0x10_0000;
const TEST_LEN: usize = 256;

pub struct Target {}

fn fill_test_pattern(out: &mut [u8; TEST_LEN]) {
    let mut i = 0usize;
    while i < out.len() {
        out[i] = (i as u8).wrapping_mul(17).wrapping_add(0x5a);
        i += 1;
    }
}

fn expect_erased(buf: &[u8]) -> Result<(), SmcError> {
    for &byte in buf {
        if byte != 0xff {
            return Err(SmcError::HardwareError);
        }
    }
    Ok(())
}

fn run_smc_fmc_cs1_write_test() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    let config = SmcConfig {
        controller_id: SmcController::Fmc,
        cs0: Some(CS0_CONFIG),
        cs1: Some(CS1_CONFIG),
        dma_enabled: true,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };

    pw_log::info!("=== AST10x0 SMC FMC CS1 write test ===");
    let fmc = unsafe { FmcUninit::new(config)? };
    let mut fmc = fmc.init()?;
    fmc.spi_nor_read_init(ChipSelect::Cs1)?;

    if !fmc.is_ready() {
        return Err(SmcError::HardwareError);
    }

    let mut flash = SpiNorFlash::from_fmc_cs(&mut fmc, CS1_CONFIG, ChipSelect::Cs1)?;
    let jedec = flash.jedec_id()?;
    pw_log::info!(
        "CS1 JEDEC ID: {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );

    pw_log::info!("=== erase CS1 sector ===");
    flash.erase_sector(TEST_OFFSET)?;

    let mut read_buf = [0u8; TEST_LEN];
    let n = flash.read(TEST_OFFSET, &mut read_buf)?;
    if n != TEST_LEN {
        return Err(SmcError::HardwareError);
    }
    expect_erased(&read_buf)?;
    dump_smc_read(&read_buf, TEST_LEN as u32);

    pw_log::info!("=== program CS1 page ===");
    let mut pattern = [0u8; TEST_LEN];
    fill_test_pattern(&mut pattern);
    let written = flash.program_page(TEST_OFFSET, &pattern)?;
    if written != TEST_LEN {
        return Err(SmcError::HardwareError);
    }

    pw_log::info!("=== read CS1 page ===");
    read_buf.fill(0);
    let n = flash.read(TEST_OFFSET, &mut read_buf)?;
    if n != TEST_LEN || read_buf != pattern {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&read_buf, TEST_LEN as u32);

    pw_log::info!("=== verify CS1 page ===");
    if !flash.verify(TEST_OFFSET, &pattern)? {
        return Err(SmcError::HardwareError);
    }

    dump_smc_register(0x7E62_0000, 16);
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SMC FMC CS1 write Test";

    fn main() -> ! {
        let sentinel = if run_smc_fmc_cs1_write_test().is_ok() {
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
