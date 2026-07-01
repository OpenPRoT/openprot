// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 Serial GPIO Matrix (SGPIOM) peripheral driver.

mod controller;
mod hal_impl;
mod registers;
mod types;

pub use controller::SgpiomController;
pub use hal_impl::{SgpiomBankPort, SgpiomMask};
pub use registers::{Sgpiom, SgpiomBankState};
pub use types::{
    Bank, BankDevice, Direction, Error, InitialLevel, InterruptMode, InterruptTrigger,
    SgpiomPinConfig,
};

impl core::fmt::Display for SgpiomBankState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "bank={} cfg(554)=0x{:08x} data(500)=0x{:08x} latch=0x{:08x} int_en=0x{:08x} int_sts=0x{:08x}",
            self.bank as u32,
            self.config,
            self.data,
            self.latch,
            self.int_en,
            self.int_status
        )
    }
}
