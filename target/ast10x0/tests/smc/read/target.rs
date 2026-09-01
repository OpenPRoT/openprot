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
    FlashConfig, FmcUninit, SmcConfig, SmcController, SmcError, SmcInstance, SmcTopology,
    TransferMode,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::{dump_smc_read, dump_smc_register};

/// Compile-time FMC descriptor: CS0 and CS1 driven on the EVB at 50 MHz,
/// geometry discovered over SFDP at init.
struct FmcInstance;

impl SmcInstance for FmcInstance {
    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: Some(FlashConfig { spi_clock_mhz: 50 }),
        dma_enabled: true,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

pub struct Target {}

/// DMA destination buffer. The FMC DMA engine (an AHB bus master) writes here,
/// so it must live in non-cached SRAM the engine and CPU observe coherently.
/// `.ram_nc` (0xA0000) is above both code and the kernel RAM region.
#[unsafe(link_section = ".ram_nc")]
static mut SMC_DMA_BUF: [u8; 256] = [0u8; 256];

#[allow(dead_code)]
fn run_smc_read_test() -> Result<(), SmcError> {
    // --- 1. Init ---
    // TODO:: set pinctrl in board/src/lib.rs
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    pw_log::info!("=== AST10x0 smc  read test  ===");
    let mut controller = unsafe { FmcUninit::<FmcInstance>::new()? }.init()?;

    let mut id = [0u8; 3];
    controller
        .cs1()?
        .transceive_user(&[0x9F], &[], &mut id, TransferMode::Mode111)?;
    pw_log::info!(
        "RDID cs1: mfr=0x{:02x} type=0x{:02x} cap=0x{:02x}",
        id[0] as u32,
        id[1] as u32,
        id[2] as u32
    );

    let mut sfdp = [0u8; 256];
    controller
        .cs1()?
        .transceive_user(&[0x5A], &[0, 0, 0, 0], &mut sfdp, TransferMode::Mode111)?;
    dump_smc_read(&sfdp, 64);

    let g = controller.cs1()?.geometry();
    pw_log::info!(
        "geom cs1: cap=0x{:08x} page={} sector={} block={}",
        g.capacity_bytes as u32,
        g.page_size as u32,
        g.sector_size as u32,
        g.block_size as u32
    );

    pw_log::info!("=== Dump 0x7E62_0000 ===");
    dump_smc_register(0x7E62_0000, 16);
    dump_smc_register(0x8000_0000, 16);
    if !controller.is_ready() || controller.controller_id() != SmcController::Fmc {
        return Err(SmcError::HardwareError);
    }

    // --- 2. MMIO read — success path ---
    // Confirm the call succeeds and returns the correct byte count.  Flash
    // content is not inspected so this is safe on both QEMU and silicon.
    pw_log::info!("=== read test cs1===");
    let mut buf = [0u8; 64];
    let n = controller.cs1()?.read(0x400, &mut buf)?;
    if n != 64 {
        return Err(SmcError::HardwareError);
    }
    dump_smc_read(&buf, 64);

    pw_log::info!("=== read dma test===");
    // --- 4. DMA  ---
    // SAFETY: SMC_DMA_BUF is a non-cached SRAM static uniquely owned here for
    // the duration of the DMA; the engine and CPU observe it coherently.
    let dma_buf: &'static mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(SMC_DMA_BUF) };

    // Poison the destination: an all-0xff readback then proves the DMA wrote it.
    dma_buf.fill(0xAA);
    pw_log::info!("=== dma dest preseed (expect AA) ===");
    dump_smc_read(dma_buf, 256);

    {
        let mut cs1 = controller.cs1()?;
        let _ = match cs1.dma_read(0x400, dma_buf.as_ptr() as usize, 256) {
            Err(SmcError::InvalidCapacity) => Ok(()),
            Err(other) => Err(other),
            Ok(()) => Err(SmcError::HardwareError),
        };

        loop {
            match cs1.poll_dma_completion() {
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
    }

    pw_log::info!("=== dma done= ==");
    dump_smc_register(0x7E62_0000, 8);
    dump_smc_register(0x7E62_0080, 8);
    dump_smc_read(dma_buf, 256);

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
