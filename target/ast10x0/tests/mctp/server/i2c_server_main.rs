// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C server app: board init + open Bus 2 + run the server-runtime loop.

#![no_main]
#![no_std]

use app_i2c_server::{handle, signals};
use ast10x0_peripherals::i2c::{ClockConfig, I2cConfig, I2cSpeed, I2cXferMode};
use i2c_server_runtime::{run, Bus};
use userspace::entry;

const SLAVE_CFG: I2cConfig = I2cConfig {
    speed: I2cSpeed::Standard,
    xfer_mode: I2cXferMode::DmaMode,
    multi_master: false,
    smbus_timeout: false,
    smbus_alert: false,
    clock_config: ClockConfig::ast1060_default(),
};

#[unsafe(link_section = ".ram_nc")]
static mut MASTER_DMA_BUF: [u8; 4096] = [0u8; 4096];
#[unsafe(link_section = ".ram_nc")]
static mut SLAVE_DMA_BUF: [u8; 256] = [0u8; 256];

#[entry]
fn entry() {
    // SAFETY: server process exclusively owns Bus 2; kernel init is complete.
    if unsafe { i2c_backend::init_bus(2, &SLAVE_CFG) }.is_err() {
        pw_log::error!("init_bus(2) failed");
        loop {}
    }

    // SAFETY: init_bus(2) done above; both buffers are non-cached and owned by this process.
    let master_dma_buf: &'static mut [u8] =
        unsafe { &mut *core::ptr::addr_of_mut!(MASTER_DMA_BUF) };
    let slave_dma_buf: &'static mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(SLAVE_DMA_BUF) };
    let driver =
        match unsafe { i2c_backend::open_bus_dma(2, &SLAVE_CFG, master_dma_buf, slave_dma_buf) } {
            Ok(d) => d,
            Err(_) => {
                pw_log::error!("open_bus_dma(2) failed");
                loop {}
            }
        };

    pw_log::info!("I2C server ready on Bus 2");

    let mut buses = [Bus::new(handle::I2C, handle::I2C2_IRQ, driver)];
    run(handle::WG, signals::I2C2, &mut buses);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
