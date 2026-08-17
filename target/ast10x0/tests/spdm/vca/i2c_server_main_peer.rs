// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use app_i2c_server_peer::{handle, signals};
use ast10x0_peripherals::create_pins;
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

#[entry]
fn entry() {
    // The kernel routed Bus 2's pins at the SCU before starting us; userspace has no SCU grant, so
    // we bind the already-routed pins rather than re-muxing them.
    // SAFETY: sole pin creation site in this binary, at boot; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let (scl, sda) = (pins.scu418_0, pins.scu418_1);

    let (Some(master_dma_buf), Some(slave_dma_buf)) = (
        i2c_backend::non_cached_buf!(4096),
        i2c_backend::non_cached_buf!(256),
    ) else {
        pw_log::error!("i2c DMA buffers already taken");
        loop {}
    };
    // Bind Bus 2's already-muxed pins and bring the controller up (init + DMA wrap) in one step.
    let Ok(driver) = i2c_backend::open_bus_dma(scl, sda, &SLAVE_CFG, master_dma_buf, slave_dma_buf)
    else {
        pw_log::error!("i2c bus open failed");
        loop {}
    };

    pw_log::info!("I2C server peer ready on Bus 2");

    let mut buses = [Bus::new(handle::I2C, handle::I2C2_IRQ, driver)];
    run(handle::WG, signals::I2C2, &mut buses);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
