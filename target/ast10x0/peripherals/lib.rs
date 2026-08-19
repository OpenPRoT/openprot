// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

/// The HAL's confined MMIO applier, re-exported so `crate::Mmio` resolves target-wide.
pub use openprot_hal::field_mux::Mmio;

pub mod gpio;
pub mod hace;
pub mod i2c;
pub mod i3c;
pub mod scu;
pub mod sgpiom;
pub mod smc;
pub mod spimonitor;
pub mod uart;

/// The chip's pin universe, created once at boot — re-exported so apps need no `scu` import for pins.
pub use scu::{create_pins, PinTokens};
