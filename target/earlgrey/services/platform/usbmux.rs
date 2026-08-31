// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::{EarlGreyGpio, GpioMask, GpioPin};
use openprot_hal_blocking::gpio_port::{
    EdgeSensitivity, GpioInterrupt, GpioPort, InterruptOperation, PinMask,
};
use util_error::ErrorCode;
use zfmt::Zfmt;

#[derive(Zfmt, Clone)]
#[zfmt(format = "USB Presence: {present}")]
pub struct UsbPresence {
    pub present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbMuxEvent {
    PinChanged,
}

pub struct UsbMuxHandler {
    pub usb_presence_n: GpioPin,
    pub usb_mux_ctrl: GpioPin,
}

impl UsbMuxHandler {
    pub const fn new(usb_presence_n: GpioPin, usb_mux_ctrl: GpioPin) -> Self {
        Self {
            usb_presence_n,
            usb_mux_ctrl,
        }
    }

    pub fn setup_interrupts(&self, gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        let usb_pres = GpioMask::from(self.usb_presence_n);
        gpio.irq_configure(usb_pres, EdgeSensitivity::BothEdges)
            .map_err(ErrorCode::from)?;
        gpio.irq_control(usb_pres, InterruptOperation::Enable)
            .map_err(ErrorCode::from)?;
        Ok(())
    }

    pub fn is_present(&self, gpio: &EarlGreyGpio) -> Result<bool, ErrorCode> {
        let pin_mask = GpioMask::from(self.usb_presence_n);
        let is_high = gpio
            .read_input()
            .map_err(ErrorCode::from)?
            .contains(pin_mask);
        Ok(!is_high)
    }

    pub fn is_host_routed(&self, gpio: &EarlGreyGpio) -> Result<bool, ErrorCode> {
        let pin_mask = GpioMask::from(self.usb_mux_ctrl);
        let is_high = gpio
            .read_output()
            .map_err(ErrorCode::from)?
            .contains(pin_mask);
        Ok(is_high)
    }

    pub fn set_host_route(&self, gpio: &mut EarlGreyGpio, host: bool) -> Result<(), ErrorCode> {
        let usb_mux = GpioMask::from(self.usb_mux_ctrl);
        if host {
            gpio.set_reset(usb_mux, GpioMask::empty())
                .map_err(ErrorCode::from)?;
        } else {
            gpio.set_reset(GpioMask::empty(), usb_mux)
                .map_err(ErrorCode::from)?;
        }
        Ok(())
    }

    pub fn handle_event(
        &mut self,
        event: UsbMuxEvent,
        gpio: &mut EarlGreyGpio,
    ) -> Result<(), ErrorCode> {
        match event {
            UsbMuxEvent::PinChanged => {
                let pin_mask = GpioMask::from(self.usb_presence_n);
                gpio.irq_control(pin_mask, InterruptOperation::Clear)
                    .map_err(ErrorCode::from)?;

                let is_high = gpio
                    .read_input()
                    .map_err(ErrorCode::from)?
                    .contains(pin_mask);
                let usb_mux = GpioMask::from(self.usb_mux_ctrl);
                if is_high {
                    gpio.set_reset(usb_mux, GpioMask::empty())
                        .map_err(ErrorCode::from)?;
                    util_zfmt::info!(UsbPresence { present: false });
                } else {
                    gpio.set_reset(GpioMask::empty(), usb_mux)
                        .map_err(ErrorCode::from)?;
                    util_zfmt::info!(UsbPresence { present: true });
                }
            }
        }
        Ok(())
    }
}
