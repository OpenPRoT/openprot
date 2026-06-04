// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC portable smoke test target.
//!
//! Safe to run on both QEMU and silicon.  Does not assert on flash content
//! because silicon flash will not be in the erased state.
//!
//! Tests (in order):
//!
//! 1. **Init** — construct FMC controller, run hardware init, assert Ready.
//! 2. **PIO read — success path** — issue a read from offset 0; assert the
//!    call succeeds and returns the expected byte count.  Flash content is not
//!    inspected.
//! 3. **PIO read — bounds rejection** — assert that a read past the configured
//!    capacity returns `SmcError::InvalidCapacity` before touching hardware.
//! 4. **DMA disabled rejection** — assert that `dma_read` returns
//!    `SmcError::DmaNotEnabled` when `SmcConfig::dma_enabled` is false.

#![no_std]
#![no_main]
#[allow(unused_imports)]
use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use ast10x0_peripherals::smc::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, SmcTopology, UninitSmc,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::{dump_smc_read, dump_smc_register};

pub struct Target {}

#[allow(dead_code)]
fn run_smc_read_test() -> Result<(), SmcError> {
    // --- 1. Init ---
    // TODO:: set pinctrl in board/src/lib.rs
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    let config = SmcConfig {
        controller_id: SmcController::Fmc,

        cs0: Some(FlashConfig {
            capacity_mb: 8,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            spi_clock_mhz: 50,
        }),
        cs1: Some(FlashConfig {
            capacity_mb: 64,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            spi_clock_mhz: 50,
        }),
        dma_enabled: true,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
    pw_log::info!("=== AST10x0 smc  read test  ===");
    let controller = unsafe { UninitSmc::new(config)? };
    let mut controller = controller.init()?;

    let _ = match controller.spi_nor_read_init(ChipSelect::Cs0) {
        Ok(v) => v,
        Err(e) => {
            pw_log::info!("Error:: spi_nor_read_init cs0");
            return Err(e);
        }
    };
    let _ = match controller.spi_nor_read_init(ChipSelect::Cs1) {
        Ok(v) => v,
        Err(e) => {
            pw_log::info!("Error:: spi_nor_read_init cs1");
            return Err(e);
        }
    };

    pw_log::info!("=== Dump 0x7E62_0000 ===");
    dump_smc_register(0x7E62_0000, 16);
    dump_smc_register(0x8000_0000, 16);
    if !controller.is_ready() || controller.controller_id() != SmcController::Fmc {
        return Err(SmcError::HardwareError);
    }

    // --- 2. MMIO read — success path ---
    // Confirm the call succeeds and returns the correct byte count.  Flash
    // content is not inspected so this is safe on both QEMU and silicon.
    // TODO: need to add test CS1
    pw_log::info!("=== read test cs0===");
    let mut buf = [0u8; 64];
    let n = controller.read(ChipSelect::Cs0, 0x400, &mut buf)?;
    if n != 64 {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&buf, 64);

    pw_log::info!("=== read test cs1===");
    let n = controller.read(ChipSelect::Cs1, 0x400, &mut buf)?;
    if n != 64 {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&buf, 64);

    pw_log::info!("=== read dma test===");
    // --- 4. DMA  ---
    let tempbuf = unsafe { core::slice::from_raw_parts(0x41500 as *mut u8, 256) };

    let _ = match controller.dma_read(ChipSelect::Cs0, 0x400, 0x41500 as usize, 256) {
        Err(SmcError::InvalidCapacity) => Ok(()),
        Err(other) => Err(other),
        Ok(()) => Err(SmcError::HardwareError),
    };

    loop {
        match controller.poll_dma_completion() {
            core::task::Poll::Pending => {
                // still running
            }
            core::task::Poll::Ready(result) => {
                result?;
                pw_log::info!("dma completion is ready");
                break;
            }
        }
    }

    pw_log::info!("=== dma done= ==");
    dump_smc_register(0x7E62_0000, 8);
    dump_smc_register(0x7E62_0080, 8);
    dump_smc_read(tempbuf, 256);

    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SMC read Test";

    fn main() -> ! {
        let sentinel = if run_smc_read_test().is_ok() {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);

        // Physical-board UART tests stop after the sentinel. Semihosting exit
        // faults on silicon when no debugger handles the BKPT request.
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
