// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use earlgrey_pinmux::{EarlGreyPinmux, Pad, PadConfig, Pull};
use core::fmt::Debug;
use openprot_hal_blocking::gpio_port::{
    EdgeSensitivity, GpioError, GpioErrorKind, GpioErrorType, GpioInterrupt, GpioPort,
    InterruptOperation, PinMask,
};
use registers::gpio;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EarlGreyGpioError {
    HardwareFailure,
    InvalidConfiguration,
}

// TODO: figure out what we're doing with precise errors.
impl GpioError for EarlGreyGpioError {
    fn kind(&self) -> GpioErrorKind {
        match self {
            EarlGreyGpioError::HardwareFailure => GpioErrorKind::HardwareFailure,
            EarlGreyGpioError::InvalidConfiguration => GpioErrorKind::UnsupportedConfiguration,
        }
    }
}

pub struct EarlGreyGpio {
    registers: gpio::RegisterBlock<ureg::RealMmioMut<'static>>,
    pinmux: EarlGreyPinmux,
}

impl EarlGreyGpio {
    /// Create a new instance of the EarlGrey GPIO driver using real MMIO.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that they have exclusive access to the GPIO and Pinmux peripherals.
    pub unsafe fn new() -> Self {
        Self {
            registers: unsafe { gpio::RegisterBlock::new(gpio::Gpio::PTR) },
            pinmux: unsafe { EarlGreyPinmux::new() },
        }
    }

    /// Read current state of output pins.
    /// 
    /// This is a target-specific extension not yet in the core HAL.
    pub fn read_output(&self) -> Result<GpioMask, EarlGreyGpioError> {
        Ok(GpioMask(self.registers.direct_out().read()))
    }

    /// Read current output enable configuration.
    pub fn read_oe(&self) -> Result<GpioMask, EarlGreyGpioError> {
        Ok(GpioMask(self.registers.direct_oe().read()))
    }
}

impl GpioErrorType for EarlGreyGpio {
    type Error = EarlGreyGpioError;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct GpioMask(pub u32);

impl PinMask for GpioMask {
    fn empty() -> Self {
        Self(0)
    }

    fn all() -> Self {
        Self(0xFFFF_FFFF)
    }

    fn is_empty(&self) -> bool {
        self.0 == 0
    }

    fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    fn union(&self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn intersection(&self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    fn toggle(&self) -> Self {
        Self(!self.0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GpioPin {
    Pin0 = 0, Pin1 = 1, Pin2 = 2, Pin3 = 3, Pin4 = 4, Pin5 = 5, Pin6 = 6, Pin7 = 7,
    Pin8 = 8, Pin9 = 9, Pin10 = 10, Pin11 = 11, Pin12 = 12, Pin13 = 13, Pin14 = 14, Pin15 = 15,
    Pin16 = 16, Pin17 = 17, Pin18 = 18, Pin19 = 19, Pin20 = 20, Pin21 = 21, Pin22 = 22, Pin23 = 23,
    Pin24 = 24, Pin25 = 25, Pin26 = 26, Pin27 = 27, Pin28 = 28, Pin29 = 29, Pin30 = 30, Pin31 = 31,
}

impl From<GpioPin> for GpioMask {
    fn from(pin: GpioPin) -> Self {
        Self(1 << (pin as u32))
    }
}

pub struct EarlGreyPinConfig {
    /// Whether the pins should be configured as inputs.
    pub is_input: bool,
    /// Whether the pins should be configured as outputs.
    pub is_output: bool,
    /// Whether to enable the 16-cycle input filter.
    pub input_filter: bool,
    /// Optional pad to connect these pins to. 
    /// If multiple pins are specified in the mask, this should be None.
    pub pad: Option<Pad>,
    /// Pull-up/down configuration for the pad.
    pub pull: Pull,
}

impl Default for EarlGreyPinConfig {
    fn default() -> Self {
        Self {
            is_input: true,
            is_output: false,
            input_filter: false,
            pad: None,
            pull: Pull::None,
        }
    }
}

impl GpioPort for EarlGreyGpio {
    type Config = EarlGreyPinConfig;
    type Mask = GpioMask;

    fn configure(&mut self, pins: Self::Mask, config: Self::Config) -> Result<(), Self::Error> {
        // If a pad is provided, ensure only one pin is being configured
        if config.pad.is_some() && (pins.0.count_ones() != 1) {
            return Err(EarlGreyGpioError::InvalidConfiguration);
        }

        // Configure Output Enable
        let lower_mask = pins.0 & 0xFFFF;
        let upper_mask = (pins.0 >> 16) & 0xFFFF;

        if lower_mask != 0 {
            self.registers.masked_oe_lower().write(|w| {
                w.mask(lower_mask)
                    .data(if config.is_output { lower_mask } else { 0 })
            });
        }

        if upper_mask != 0 {
            self.registers.masked_oe_upper().write(|w| {
                w.mask(upper_mask)
                    .data(if config.is_output { upper_mask } else { 0 })
            });
        }

        // Configure Input Filter
        self.registers.ctrl_en_input_filter().modify(|w| {
            if config.input_filter {
                w | pins.0
            } else {
                w & !pins.0
            }
        });

        // Handle Pinmux and Pad attributes
        if let Some(pad) = config.pad {
            let pin_idx = pins.0.trailing_zeros() as usize;
            
            // DIO pads are dedicated and don't require routing, 
            // only attribute configuration.
            if !pad.is_dio() {
                if config.is_input {
                    self.pinmux.connect_input(pin_idx, pad);
                }
                if config.is_output {
                    self.pinmux.connect_output(pad, pin_idx);
                }
            }

            self.pinmux.configure_pad(pad, &PadConfig {
                pull: config.pull,
                ..Default::default()
            });
        }

        Ok(())
    }

    fn set_reset(
        &mut self,
        set_mask: Self::Mask,
        reset_mask: Self::Mask,
    ) -> Result<(), Self::Error> {
        // Process lower 16 bits
        let set_lower = set_mask.0 & 0xFFFF;
        let reset_lower = reset_mask.0 & 0xFFFF;
        let lower_mask = set_lower | reset_lower;

        if lower_mask != 0 {
            self.registers.masked_out_lower().write(|w| {
                w.mask(lower_mask).data(set_lower)
            });
        }

        // Process upper 16 bits
        let set_upper = (set_mask.0 >> 16) & 0xFFFF;
        let reset_upper = (reset_mask.0 >> 16) & 0xFFFF;
        let upper_mask = set_upper | reset_upper;

        if upper_mask != 0 {
            self.registers.masked_out_upper().write(|w| {
                w.mask(upper_mask).data(set_upper)
            });
        }

        Ok(())
    }

    fn read_input(&self) -> Result<Self::Mask, Self::Error> {
        Ok(GpioMask(self.registers.data_in().read()))
    }

    fn toggle(&mut self, pins: Self::Mask) -> Result<(), Self::Error> {
        let current = self.read_output()?;
        let set_mask = GpioMask(pins.0 & !current.0);
        let reset_mask = GpioMask(pins.0 & current.0);
        self.set_reset(set_mask, reset_mask)
    }
}

impl GpioInterrupt for EarlGreyGpio {
    type Mask = GpioMask;

    fn irq_configure(
        &mut self,
        mask: Self::Mask,
        sensitivity: EdgeSensitivity,
    ) -> Result<(), Self::Error> {
        // Clear all sensitivity settings for these pins first
        self.registers.intr_ctrl_en_rising().modify(|w| w & !mask.0);
        self.registers.intr_ctrl_en_falling().modify(|w| w & !mask.0);
        self.registers.intr_ctrl_en_lvlhigh().modify(|w| w & !mask.0);
        self.registers.intr_ctrl_en_lvllow().modify(|w| w & !mask.0);

        // Apply new sensitivity
        match sensitivity {
            EdgeSensitivity::RisingEdge => {
                self.registers.intr_ctrl_en_rising().modify(|w| w | mask.0);
            }
            EdgeSensitivity::FallingEdge => {
                self.registers.intr_ctrl_en_falling().modify(|w| w | mask.0);
            }
            EdgeSensitivity::BothEdges => {
                self.registers.intr_ctrl_en_rising().modify(|w| w | mask.0);
                self.registers.intr_ctrl_en_falling().modify(|w| w | mask.0);
            }
            EdgeSensitivity::HighLevel => {
                self.registers.intr_ctrl_en_lvlhigh().modify(|w| w | mask.0);
            }
            EdgeSensitivity::LowLevel => {
                self.registers.intr_ctrl_en_lvllow().modify(|w| w | mask.0);
            }
        }

        Ok(())
    }

    fn irq_control(
        &mut self,
        mask: Self::Mask,
        operation: InterruptOperation,
    ) -> Result<bool, Self::Error> {
        match operation {
            InterruptOperation::Enable => {
                // Clear state first to avoid spurious interrupts (ported from pie-rot)
                self.registers.intr_state().write(|_| mask.0);
                self.registers.intr_enable().modify(|w| w | mask.0);
                Ok(true)
            }
            InterruptOperation::Disable => {
                self.registers.intr_enable().modify(|w| w & !mask.0);
                Ok(true)
            }
            InterruptOperation::Clear => {
                // In EarlGrey, writing 1 to intr_state clears the interrupt
                self.registers.intr_state().write(|_| mask.0);
                Ok(true)
            }
            InterruptOperation::IsPending => {
                let state = self.registers.intr_state().read();
                Ok((state & mask.0) != 0)
            }
        }
    }

    fn register_interrupt_handler<F>(
        &mut self,
        _mask: Self::Mask,
        _handler: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Self::Mask) + Send + 'static,
    {
        // In the OpenPRoT microkernel architecture, interrupts are handled
        // via syscalls (wait on object) rather than registered callbacks.
        Err(EarlGreyGpioError::InvalidConfiguration)
    }
}
