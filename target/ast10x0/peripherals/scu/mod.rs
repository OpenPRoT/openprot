// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 System Control Unit (SCU) module.

pub mod clock;
pub mod pinctrl;
pub mod registers;
pub mod reset;
pub mod routing;
pub mod status;
pub mod types;

pub use pinctrl::PinctrlPin;
pub use registers::ScuRegisters;
pub use types::{
    ClockRegisterHalf, ScuError, ScuExtMuxSelect, ScuRegisterHalf, SpiMonitorInstance,
    SpiMonitorPassthrough, SpiMonitorSource,
};
