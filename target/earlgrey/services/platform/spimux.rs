// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::{EarlGreyGpio, GpioMask, GpioPin};
use openprot_hal_blocking::gpio_port::{GpioPort, PinMask};
use userspace::time::{sleep_until, Clock, Duration, SystemClock};
use util_error::ErrorCode;

const RESET_HOLD_DELAY: Duration = Duration::from_micros(10);
const MUX_SWITCH_DELAY: Duration = Duration::from_micros(10);
const RESET_RECOVERY_DELAY: Duration = Duration::from_micros(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMuxRoute {
    /// Host CPU connects to Flash 0 (SPI_MUX_CTRL = LOW).
    /// Earlgrey accesses Flash 1 via SPI_HOST1.
    HostCpu0Earlgrey1,
    /// Host CPU connects to Flash 1 (SPI_MUX_CTRL = HIGH).
    /// Earlgrey accesses Flash 0 via SPI_HOST0.
    HostCpu1Earlgrey0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMuxEvent {
    ColdBoot,
    Route(SpiMuxRoute),
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
            SpiMuxEvent::ColdBoot => self.switch_mux_to(SpiMuxRoute::HostCpu0Earlgrey1, gpio),
            SpiMuxEvent::Route(route) => self.switch_mux_to(route, gpio),
        }
    }

    pub fn switch_mux_to(
        &mut self,
        route: SpiMuxRoute,
        gpio: &mut EarlGreyGpio,
    ) -> Result<(), ErrorCode> {
        // Step 1: Assert Reset to external Flash EEPROMs (drive SPI_RESET_N to LOW / 0V).
        // Note: GpioPort::set_reset(set_mask, reset_mask) drives set_mask pins HIGH (1)
        // and reset_mask pins LOW (0).
        gpio.set_reset(GpioMask::empty(), GpioMask::from(self.spi_reset_n))
            .map_err(ErrorCode::from)?;
        let _ = sleep_until(SystemClock::now() + RESET_HOLD_DELAY);

        // Step 2: Switch MUX selection channel.
        // HostCpu0Earlgrey1 -> SPI_MUX_CTRL = LOW  (Host CPU to Flash 0, Earlgrey to Flash 1).
        // HostCpu1Earlgrey0 -> SPI_MUX_CTRL = HIGH (Host CPU to Flash 1, Earlgrey to Flash 0).
        let ctrl_mask = GpioMask::from(self.spi_mux_ctrl);
        match route {
            SpiMuxRoute::HostCpu0Earlgrey1 => {
                gpio.set_reset(GpioMask::empty(), ctrl_mask)
                    .map_err(ErrorCode::from)?;
            }
            SpiMuxRoute::HostCpu1Earlgrey0 => {
                gpio.set_reset(ctrl_mask, GpioMask::empty())
                    .map_err(ErrorCode::from)?;
            }
        }
        let _ = sleep_until(SystemClock::now() + MUX_SWITCH_DELAY);

        // Step 3: Release Reset (SPI_RESET_N = HIGH), Enable Mux (SPI_MUX_EN_N = LOW),
        // and ensure both Write-Protect pins are unasserted / unprotected (WP0_N = HIGH, WP1_N = HIGH).
        let high_pins = GpioMask::from(self.spi_reset_n)
            .union(GpioMask::from(self.spi_host0_wp_n))
            .union(GpioMask::from(self.spi_host1_wp_n));
        let low_pins = GpioMask::from(self.spi_mux_en_n);

        gpio.set_reset(high_pins, low_pins)
            .map_err(ErrorCode::from)?;
        let _ = sleep_until(SystemClock::now() + RESET_RECOVERY_DELAY);

        Ok(())
    }
}
