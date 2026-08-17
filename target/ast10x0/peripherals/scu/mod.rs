// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 System Control Unit (SCU) module.

pub mod cache;
pub mod clock;
pub mod pinctrl;
pub mod pinmux;
pub mod pins;
pub mod registers;
pub mod reset;
pub mod routing;
pub mod status;
pub mod types;

pub use openprot_hal::resource::FieldWrite;
pub use pinmux::{apply_mux, route, PinctrlPin};
pub use pins::{create_pins, scu414_30, scu414_31, scu418_0, scu418_1, PinTokens, I2C_CTRL};
pub use registers::ScuRegisters;
pub use routing::SpimGpioOriVal;
pub use types::{
    ClockRegisterHalf, ScuError, ScuExtMuxSelect, ScuRegisterHalf, SpiMonitorInstance,
    SpiMonitorPassthrough, SpiMonitorSource,
};
