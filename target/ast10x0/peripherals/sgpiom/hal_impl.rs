// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use openprot_hal_blocking::gpio_port::{
    ActivePolarity, EdgeSensitivity, GpioBankPassthrough, GpioError, GpioErrorKind, GpioErrorType,
    GpioInterrupt, GpioPort, InterruptOperation, PinConfig, PinDirection, PinMask,
};

use super::registers::Sgpiom;
use super::types::{BankDevice, Error, InterruptMode, InterruptTrigger};

/// 32-bit pin mask for a single SGPIOM bank.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SgpiomMask(pub u32);

impl PinMask for SgpiomMask {
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

impl GpioError for Error {
    fn kind(&self) -> GpioErrorKind {
        match self {
            Error::InvalidPin => GpioErrorKind::InvalidPin,
            Error::InvalidNgpios => GpioErrorKind::UnsupportedConfiguration,
            Error::UnsupportedFlags => GpioErrorKind::UnsupportedConfiguration,
        }
    }
}

/// A single SGPIOM bank exposed as a HAL [`GpioPort`].
///
/// Combines the shared [`Sgpiom`] register block with a [`BankDevice`] descriptor.
/// Use [`SgpiomBankPort::new`] to construct; the same safety contract as [`Sgpiom::new`]
/// applies.
pub struct SgpiomBankPort {
    pub(super) sgpiom: Sgpiom,
    pub(super) dev: BankDevice,
}

impl SgpiomBankPort {
    /// Create a bank port from an existing `Sgpiom` instance and a `BankDevice` descriptor.
    ///
    /// # Safety
    ///
    /// Same contract as [`Sgpiom::new`]: the register block pointer must be valid, non-null,
    /// and access must be externally coordinated for the lifetime of this value.
    pub const unsafe fn new(sgpiom: Sgpiom, dev: BankDevice) -> Self {
        Self { sgpiom, dev }
    }
}

impl GpioErrorType for SgpiomBankPort {
    type Error = Error;
}

impl GpioPort for SgpiomBankPort {
    type Config = PinConfig;
    type Mask = SgpiomMask;

    fn configure(&mut self, pins: Self::Mask, config: Self::Config) -> Result<(), Self::Error> {
        // Reject pins outside this bank's ngpios window.
        self.dev.validate_mask(pins.0)?;

        if config.direction == PinDirection::Output {
            if let Some(logical_high) = config.initial_output {
                // Map logical level through active polarity to physical drive level.
                let drive_high = match config.polarity {
                    ActivePolarity::ActiveHigh => logical_high,
                    ActivePolarity::ActiveLow => !logical_high,
                };
                if drive_high {
                    self.sgpiom.port_set_bits_raw(self.dev.bank, pins.0);
                } else {
                    self.sgpiom.port_clear_bits_raw(self.dev.bank, pins.0);
                }
            }
        }

        // SGPIOM direction is hardware-managed; no direction register write is required.
        Ok(())
    }

    fn set_reset(
        &mut self,
        set_mask: Self::Mask,
        reset_mask: Self::Mask,
    ) -> Result<(), Self::Error> {
        self.dev.validate_mask(set_mask.0 | reset_mask.0)?;
        // Atomically apply both masks: set wins over reset for overlapping bits.
        self.sgpiom
            .port_set_masked_raw(self.dev.bank, set_mask.0 | reset_mask.0, set_mask.0);
        Ok(())
    }

    fn read_input(&self) -> Result<Self::Mask, Self::Error> {
        Ok(SgpiomMask(self.sgpiom.port_get_raw(self.dev.bank)))
    }

    fn toggle(&mut self, pins: Self::Mask) -> Result<(), Self::Error> {
        self.dev.validate_mask(pins.0)?;
        self.sgpiom.port_toggle_bits(self.dev.bank, pins.0);
        Ok(())
    }
}

impl GpioInterrupt for SgpiomBankPort {
    type Mask = SgpiomMask;

    /// Program interrupt sensitivity for the masked pins.
    ///
    /// Only sensitivity is written here; enabling/disabling is done via
    /// [`GpioInterrupt::irq_control`] — matching the EarlGrey driver split.
    fn irq_configure(
        &mut self,
        mask: Self::Mask,
        sensitivity: EdgeSensitivity,
    ) -> Result<(), Self::Error> {
        let (mode, trig) = match sensitivity {
            EdgeSensitivity::RisingEdge => (InterruptMode::Edge, InterruptTrigger::High),
            EdgeSensitivity::FallingEdge => (InterruptMode::Edge, InterruptTrigger::Low),
            EdgeSensitivity::BothEdges => (InterruptMode::Edge, InterruptTrigger::Both),
            EdgeSensitivity::HighLevel => (InterruptMode::Level, InterruptTrigger::High),
            EdgeSensitivity::LowLevel => (InterruptMode::Level, InterruptTrigger::Low),
        };

        self.sgpiom
            .configure_interrupt_sensitivity_masked(&self.dev, mask.0, mode, trig)
    }

    /// Enable/disable/clear/query interrupts for the masked pins.
    fn irq_control(
        &mut self,
        mask: Self::Mask,
        operation: InterruptOperation,
    ) -> Result<bool, Self::Error> {
        self.dev.validate_mask(mask.0)?;
        match operation {
            InterruptOperation::Enable => {
                // Clear stale status first to avoid a spurious immediate fire.
                self.sgpiom.clear_interrupt_status(self.dev.bank, mask.0);
                self.sgpiom.set_interrupt_enable(self.dev.bank, mask.0);
                Ok(true)
            }
            InterruptOperation::Disable => {
                self.sgpiom.clear_interrupt_enable(self.dev.bank, mask.0);
                Ok(true)
            }
            InterruptOperation::Clear => {
                self.sgpiom.clear_interrupt_status(self.dev.bank, mask.0);
                Ok(true)
            }
            InterruptOperation::IsPending => {
                let status = self.sgpiom.interrupt_status(self.dev.bank);
                Ok((status & mask.0) != 0)
            }
        }
    }

    /// Callback registration is unsupported: in the OpenPRoT microkernel
    /// architecture interrupts are delivered to userspace via wait-on-object
    /// syscalls rather than in-driver callbacks (same as the EarlGrey driver).
    fn register_interrupt_handler<F>(
        &mut self,
        _mask: Self::Mask,
        _handler: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Self::Mask) + Send + 'static,
    {
        Err(Error::UnsupportedFlags)
    }
}

impl GpioBankPassthrough for SgpiomBankPort {
    /// Sample current inputs for `mask` pins and write them to the output latch (one-shot).
    fn set_passthrough_mask(&mut self, mask: Self::Mask) -> Result<(), Self::Error> {
        self.dev.validate_mask(mask.0)?;
        self.sgpiom.passthrough_masked(self.dev.bank, mask.0);
        Ok(())
    }

    /// No persistent passthrough hardware state exists; this is a no-op.
    fn clear_passthrough(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
