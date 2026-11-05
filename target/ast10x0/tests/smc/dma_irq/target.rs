// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC DMA-read interrupt completion test target.
//!
//! Tests (in order):
//!
//! 1. **Init with DMA interrupts enabled** -- construct the FMC controller,
//!    apply FMC pinmux, initialize hardware, and assert the controller is ready.
//! 2. **DMA read kick-off** -- start a valid CS0 DMA read and assert the
//!    controller moves out of Ready while the transfer is in flight.
//! 3. **IRQ completion path** -- the FMC IRQ handler records that the interrupt
//!    fired, then main context completes the transfer through `handle_dma_irq()`
//!    rather than polling the DMA status register.
//! 4. **Ready after IRQ** -- assert the IRQ completion path clears state and
//!    returns the controller to Ready.

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use core::task::Poll;

use arch_arm_cortex_m::Arch;
use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use ast10x0_peripherals::smc::{
    Cs, FlashConfig, FmcUninit, SmcConfig, SmcController, SmcError, SmcInstance, SmcInterrupt,
    SmcTopology,
};
use codegen as _;
use console_backend::console_backend_write_all;
use kernel::Arch as KernelArch;
use kernel::Kernel;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

#[path = "../target_debug.rs"]
mod target_debug;
use target_debug::{dump_smc_read, dump_smc_register};

pub struct Target {}

/// Compile-time FMC descriptor: single CS0 device at 50 MHz on the boot-SPI
/// interface, DMA-completion interrupts enabled, geometry discovered over SFDP
/// at init.
struct FmcInstance;

impl SmcInstance for FmcInstance {
    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: None,
        dma_enabled: true,
        enable_interrupts: true,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

const DMA_FLASH_OFFSET: u32 = 0x500;
const DMA_DRAM_ADDR: usize = 0x0004_1000;
const DMA_LEN: u32 = 256;
const FMC_IRQ: u32 = 39;
const DMA_IRQ_TIMEOUT: u32 = 0x00010_0000;

static FMC_DMA_IRQ_FIRED: AtomicBool = AtomicBool::new(false);
static FMC_DMA_IRQ_COUNT: AtomicU32 = AtomicU32::new(0);

fn read_u32(addr: usize) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

fn dump_nvic_irq_state(irq: u32) {
    let word = (irq / 32) as usize;
    let bit = irq % 32;
    let mask = 1u32 << bit;
    let iser = read_u32(0xE000_E100 + (word * 4));
    let ispr = read_u32(0xE000_E200 + (word * 4));
    let iabr = read_u32(0xE000_E300 + (word * 4));

    pw_log::info!(
        "NVIC irq={} word={} bit={} mask=0x{:08x} ISER=0x{:08x} ISPR=0x{:08x} IABR=0x{:08x}",
        irq as u32,
        word as u32,
        bit as u32,
        mask as u32,
        iser as u32,
        ispr as u32,
        iabr as u32,
    );
}

pub fn fmc_dma_irq_handler<K: Kernel>(_kernel: K) {
    FMC_DMA_IRQ_COUNT.fetch_add(1, Ordering::AcqRel);
    FMC_DMA_IRQ_FIRED.store(true, Ordering::Release);
    <<Arch as KernelArch>::InterruptController as kernel::interrupt_controller::InterruptController>::disable_interrupt(
        FMC_IRQ,
    );
}

fn poll_dma_irq_completion(cs: &mut Cs<'_>) -> Poll<Result<(), SmcError>> {
    if !FMC_DMA_IRQ_FIRED.swap(false, Ordering::AcqRel) {
        return Poll::Pending;
    }

    match cs.handle_dma_irq() {
        Ok(SmcInterrupt::DmaComplete) => Poll::Ready(Ok(())),
        Ok(_) => Poll::Ready(Err(SmcError::HardwareError)),
        Err(err) => Poll::Ready(Err(err)),
    }
}

fn run_dma_read_irq_test() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    let mut controller = unsafe { FmcUninit::<FmcInstance>::new()? }.init()?;

    if !controller.is_ready() || controller.controller_id() != SmcController::Fmc {
        return Err(SmcError::HardwareError);
    }

    FMC_DMA_IRQ_FIRED.store(false, Ordering::Release);
    FMC_DMA_IRQ_COUNT.store(0, Ordering::Release);
    <<Arch as KernelArch>::InterruptController as kernel::interrupt_controller::InterruptController>::enable_interrupt(
        FMC_IRQ,
    );
    dump_nvic_irq_state(FMC_IRQ);

    pw_log::info!("SMC DMA IRQ: starting DMA read");
    dump_smc_register(0x7E62_0000, 8);
    dump_smc_register(0x7E62_0080, 8);
    {
        let mut cs0 = controller.cs0()?;
        cs0.dma_read(DMA_FLASH_OFFSET, DMA_DRAM_ADDR, DMA_LEN)?;
    }
    pw_log::info!("after calling dma_read()");
    dump_smc_register(0x7E62_0000, 8);
    dump_smc_register(0x7E62_0080, 8);
    if controller.is_ready() {
        return Err(SmcError::HardwareError);
    }

    let mut completed = false;
    {
        let mut cs0 = controller.cs0()?;
        for _ in 0..DMA_IRQ_TIMEOUT {
            match poll_dma_irq_completion(&mut cs0) {
                Poll::Ready(result) => {
                    result?;
                    completed = true;
                    break;
                }
                Poll::Pending => core::hint::spin_loop(),
            }
        }
        if !completed {
            pw_log::info!("dma Timeout");
            pw_log::info!(
                "FMC IRQ count={}",
                FMC_DMA_IRQ_COUNT.load(Ordering::Acquire) as u32
            );
            pw_log::info!(
                "FMC DMA status at timeout=0x{:08x}",
                cs0.dma_status() as u32
            );
        }
    }

    if !completed {
        dump_nvic_irq_state(FMC_IRQ);
        return Err(SmcError::Timeout);
    }

    if !controller.is_ready() {
        return Err(SmcError::HardwareError);
    }
    pw_log::info!("SMC DMA IRQ: DMA read completed via IRQ");
    let dma_buf =
        unsafe { core::slice::from_raw_parts(DMA_DRAM_ADDR as *const u8, DMA_LEN as usize) };
    dump_smc_register(0x7E62_0000, 8);
    dump_smc_register(0x7E62_0080, 8);
    dump_smc_read(dma_buf, DMA_LEN);
    Ok(())
}

codegen::declare_kernel_interrupt_handlers!();
declare_target!(Target);

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SMC DMA Read IRQ Test";

    fn main() -> ! {
        let sentinel: &[u8] = match run_dma_read_irq_test() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(_) => b"TEST_RESULT:FAIL\n",
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}
