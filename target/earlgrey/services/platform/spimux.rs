// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::{EarlGreyGpio, GpioMask, GpioPin};
use openprot_hal_blocking::gpio_port::{GpioPort, PinMask};
use util_error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMuxEvent {
    ColdBoot,
}

pub struct SpiMuxHandler {
    pub spi_mux_en_n: GpioPin,
    pub spi_mux_ctrl: GpioPin,
    pub spi_reset_n: GpioPin,
    pub spi_host0_wp_n: GpioPin,
    pub spi_host1_wp_n: GpioPin,
}

impl SpiMuxHandler {
    pub const fn new(
        spi_mux_en_n: GpioPin,
        spi_mux_ctrl: GpioPin,
        spi_reset_n: GpioPin,
        spi_host0_wp_n: GpioPin,
        spi_host1_wp_n: GpioPin,
    ) -> Self {
        Self {
            spi_mux_en_n,
            spi_mux_ctrl,
            spi_reset_n,
            spi_host0_wp_n,
            spi_host1_wp_n,
        }
    }

    pub fn handle_event(
        &mut self,
        event: SpiMuxEvent,
        gpio: &mut EarlGreyGpio,
    ) -> Result<(), ErrorCode> {
        match event {
            SpiMuxEvent::ColdBoot => {
                let low_pins =
                    GpioMask::from(self.spi_mux_ctrl).union(GpioMask::from(self.spi_mux_en_n));
                let high_pins = GpioMask::from(self.spi_reset_n)
                    .union(GpioMask::from(self.spi_host0_wp_n))
                    .union(GpioMask::from(self.spi_host1_wp_n));

                gpio.set_reset(high_pins, low_pins)
                    .map_err(ErrorCode::from)?;
            }
        }
        Ok(())
    }
}
