// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use openprot_hal_blocking::gpio_port::GpioErrorKind;

use super::hal_impl::SgpiomBankPort;

/// Concrete SGPIOM controller owning up to `N` child bank ports (US-01, US-05).
///
/// Each slot is `Option<SgpiomBankPort>` so that absent banks (disabled in
/// Devicetree) are represented safely without crashing callers (US-07).
/// Operations on one bank do not affect any other bank.
pub struct SgpiomController<const N: usize> {
    banks: [Option<SgpiomBankPort>; N],
}

impl<const N: usize> SgpiomController<N> {
    /// Construct from an array of optional bank instances.
    pub fn new(banks: [Option<SgpiomBankPort>; N]) -> Self {
        Self { banks }
    }

    /// Return the total number of bank slots (present or absent).
    pub fn num_banks(&self) -> usize {
        N
    }

    /// Obtain a mutable reference to the bank at `index`.
    ///
    /// Returns `Err(GpioErrorKind::InvalidPort)` when the index is out of range
    /// or the bank slot is absent.
    pub fn bank(&mut self, index: usize) -> Result<&mut SgpiomBankPort, GpioErrorKind> {
        self.banks
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or(GpioErrorKind::InvalidPort)
    }

    /// Return `None` when the bank is absent; never errors.
    /// Prefer this for optional-bank patterns (US-07).
    pub fn bank_opt(&mut self, index: usize) -> Option<&mut SgpiomBankPort> {
        self.banks.get_mut(index)?.as_mut()
    }
}
