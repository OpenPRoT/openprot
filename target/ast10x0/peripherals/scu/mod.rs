// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 System Control Unit (SCU) module.

pub mod registers;
pub mod types;
pub mod reset;
pub mod clock;
pub mod status;
pub mod pinctrl;

pub use registers::ScuRegisters;
pub use pinctrl::PinctrlPin;
pub use types::{ClockRegisterHalf, ScuRegisterHalf};