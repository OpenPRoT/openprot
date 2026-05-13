// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Read-only SCU status helpers.

use super::registers::ScuRegisters;

impl ScuRegisters {
    /// Read the raw hardware-revision register value.
    #[must_use]
    pub fn hardware_revision_raw(&self) -> u32 {
        self.regs().scu004().read().bits()
    }

    /// Read the raw route-control register value.
    #[must_use]
    pub fn route_control_raw(&self) -> u32 {
        self.regs().scu0f0().read().bits()
    }

    /// Read the raw SCU690 multi-function control register value.
    #[must_use]
    pub fn multi_func_690_raw(&self) -> u32 {
        self.regs().scu690().read().bits()
    }

    /// Read the raw SCU694 multi-function control register value.
    #[must_use]
    pub fn multi_func_694_raw(&self) -> u32 {
        self.regs().scu694().read().bits()
    }
}