// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC FMC CS1 SFDP config-discovery test target.
//!
//! Drives `FmcUninit::new(config).init()`: `init()` discovers both driven chip
//! selects from SFDP and finalizes a controller whose per-CS geometry is
//! *derived* from each chip — no capacity is fed in, only `spi_clock_mhz`.
//! Asserts the discovered CS1 capacity (EVB's W25Q512) is 64 MiB. Read-only:
//! issues only READ_SFDP / READ_ID; nothing is erased or programmed.

#![no_std]
#![no_main]

#[allow(unused_imports)]
use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use ast10x0_peripherals::smc::{
    FlashConfig, FmcUninit, SmcConfig, SmcController, SmcError, SmcInstance, SmcTopology,
    SpiNorFlash, SpiNorFlashDevice,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

/// The one geometry field SFDP does not encode; everything else is discovered.
const SPI_CLOCK_MHZ: u32 = 50;

/// Expected capacity of the EVB's W25Q512 on CS1.
const CS1_EXPECTED_CAPACITY_MB: u32 = 64;

/// Compile-time FMC descriptor: both CS driven on the EVB at 50 MHz. Capacity
/// for each is discovered by `init()`, not fed in here.
struct FmcInstance;

impl SmcInstance for FmcInstance {
    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig {
            spi_clock_mhz: SPI_CLOCK_MHZ,
        }),
        cs1: Some(FlashConfig {
            spi_clock_mhz: SPI_CLOCK_MHZ,
        }),
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

pub struct Target {}

fn run_smc_fmc_cs1_sfdp_test() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    pw_log::info!("=== AST10x0 SMC FMC CS1 SFDP config test ===");
    // Both CS are driven on the EVB; capacity for each is an OUTPUT of init().
    let mut fmc = unsafe { FmcUninit::<FmcInstance>::new()? }.init()?;

    if !fmc.is_ready() {
        return Err(SmcError::HardwareError);
    }

    // CS1 geometry was discovered over SFDP during `init()`; read it back.
    let g = {
        let cs1 = fmc.cs1()?;
        cs1.geometry()
    };
    pw_log::info!(
        "derived CS1 geom: cap=0x{:08x} page={} sector={} block={}",
        g.capacity_bytes as u32,
        g.page_size as u32,
        g.sector_size as u32,
        g.block_size as u32
    );

    // JEDEC ID for the record (W25Q512 = EF 40 20). Read-only.
    let jedec = {
        let flash = SpiNorFlash::new(fmc.cs1()?)?;
        flash.jedec_id()?
    };
    pw_log::info!(
        "CS1 JEDEC ID: {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );

    let cap_mb = (g.capacity_bytes / (1024 * 1024)) as u32;
    if cap_mb == CS1_EXPECTED_CAPACITY_MB {
        pw_log::info!("SFDP-derived CS1 capacity MATCHES 64 MiB");
        Ok(())
    } else {
        pw_log::info!("SFDP-derived CS1 capacity DIFFERS from 64 MiB");
        Err(SmcError::DeviceNotSupported)
    }
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SMC FMC CS1 SFDP Test";

    fn main() -> ! {
        let sentinel = if run_smc_fmc_cs1_sfdp_test().is_ok() {
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
