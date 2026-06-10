// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SMC SPI2 read smoke test target.

#![no_std]
#![no_main]
use ast1060_pac as device;
#[allow(unused_imports)]
use ast10x0_peripherals::scu::{
    pinctrl::{
        PINCTRL_GPIOL2, PINCTRL_GPIOL3, PINCTRL_SPI2_QUAD, PINCTRL_SPIM3_DEFAULT,
        PINCTRL_SPIM4_DEFAULT,
    },
    ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance, SpiMonitorPassthrough, SpiMonitorSource,
};
use ast10x0_peripherals::smc::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, SmcTopology, SpiTransaction,
    SpiUninit, TransferMode,
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
    spi_clock_mhz: 25,
};

pub struct Target {}
/*
SCU418  = 0x7E6E_2418, clear bits 26, 27
GPIO070 = 0x7E78_0070, set bits 26, 27
GPIO074 = 0x7E78_0074, set bits 26, 27
*/
fn gpio_flash_power() {
    const GPIOL2_L3_MASK: u32 = (1 << 26) | (1 << 27);

    // SAFETY: Board initialization has exclusive access to the PAC singleton.
    let gpio = unsafe { &*device::Gpio::ptr() };
    gpio.gpio070()
        .modify(|r, w| unsafe { w.bits(r.bits() | GPIOL2_L3_MASK) });
    gpio.gpio074()
        .modify(|r, w| unsafe { w.bits(r.bits() | GPIOL2_L3_MASK) });

    for _ in 0..1_000_000 {
        core::hint::spin_loop();
    }
}

#[allow(dead_code)]
fn configure_spi2_external_mux(select_mux1: bool) {
    const GPIO_E8: u32 = 1 << 8;
    const SGPIOM_A_D_2: u32 = 1 << 2;

    // SAFETY: Board initialization has exclusive access to the PAC singletons.
    let gpio = unsafe { &*device::Gpio::ptr() };
    let sgpio = unsafe { &*device::Sgpiom::ptr() };
    let _scu_unlocked = unsafe { ScuRegisters::new_global_unlocked() };
    let scu = unsafe { &*device::Scu::ptr() };

    scu.scu41c().modify(|_, w| {
        w.enbl_sgpiomaster_ckfn_pin()
            .set_bit()
            .enbl_sgpiomaster_ldfn_pin()
            .set_bit()
            .enbl_sgpiomaster_dofn_pin()
            .set_bit()
            .enbl_sgpiomaster_difn_pin()
            .set_bit()
    });
    sgpio.gpio554().modify(|_, w| unsafe {
        w.enbl_of_serial_gpio()
            .set_bit()
            .numbers_of_serial_gpiopins()
            .bits(16)
            .serial_gpioclk_division()
            .bits(24)
    });

    gpio.gpio020().modify(|r, w| unsafe {
        let bits = if select_mux1 {
            r.bits() | GPIO_E8
        } else {
            r.bits() & !GPIO_E8
        };
        w.bits(bits)
    });
    gpio.gpio024()
        .modify(|r, w| unsafe { w.bits(r.bits() | GPIO_E8) });

    let sgpio_latch = sgpio.gpio570().read().bits();
    let sgpio_data = if select_mux1 {
        sgpio_latch | SGPIOM_A_D_2
    } else {
        sgpio_latch & !SGPIOM_A_D_2
    };
    sgpio.gpio500().write(|w| unsafe { w.bits(sgpio_data) });

    // Match the overlay's ext-mux-sel-delay-us = <1000>.
    for _ in 0..100_000 {
        core::hint::spin_loop();
    }
}

fn config_spi2_master_controller() -> Result<(), SmcError> {
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_SPIM3_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPIM4_DEFAULT);
    scu.apply_pinctrl_group(PINCTRL_SPI2_QUAD);
    scu.apply_pinctrl_group(PINCTRL_GPIOL2);
    scu.apply_pinctrl_group(PINCTRL_GPIOL3);
    gpio_flash_power();
    //configure spi2 external mux through gpio pins
    configure_spi2_external_mux(true);

    // Configure the mux for the SPI master controller path.
    scu.set_spim_internal_master_route(SpiMonitorInstance::Spim2, SpiMonitorSource::Spi2);
    scu.set_spim_passthrough(SpiMonitorInstance::Spim2, SpiMonitorPassthrough::Enabled);
    scu.set_spim_ext_mux(SpiMonitorInstance::Spim2, ScuExtMuxSelect::Mux1);
    pw_log::info!("SCU pinmux and SPIM routing configured for SPI2 monitoring");

    for _ in 0..1_000_000 {
        core::hint::spin_loop();
    }
    Ok(())
}

fn run_spi2_read_test() -> Result<(), SmcError> {
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

    pw_log::info!("=== SPI2 controller register ===");
    dump_smc_register(0x7E64_0000, 16);
    dump_smc_register(0x7E64_0080, 16);
    pw_log::info!("=== SCU QSPI Mux routing register ===");
    dump_smc_register(0x7E6E_20F0, 2);
    dump_smc_register(0x7E6E_2418, 2);
    dump_smc_register(0x7e6e_2694, 2);
    dump_smc_register(0x7E78_0020, 2);
    dump_smc_register(0x7E78_0070, 2);
    let mut jedec = [0u8; 3];
    SpiTransaction::transceive_user_with_spim(
        &mut spi,
        SpiMonitorInstance::Spim2,
        ChipSelect::Cs0,
        &[0x9f],
        &[],
        &mut jedec,
        TransferMode::Mode111,
    )?;
    pw_log::info!(
        "SPI2 CS0 JEDEC ID: {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );

    if jedec[0] == 0xff {
        pw_log::info!("SPI2 CS0 JEDEC manufacturer is 0xff; skipping read test");
        return Ok(());
    }

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
