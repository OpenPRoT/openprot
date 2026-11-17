// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO peripheral: pin-token-driven, `unsafe`-free access; kernel-side init routes a pin
//! via `openprot_hal::field_mux::Route`, `bind_gpio` binds it for userspace to drive.

mod map;

pub(crate) use map::{ABCD, EFGH, IJKL};
pub use openprot_hal::gpio::{
    bind_gpio, ActiveHigh, ActiveLow, Gpio, GpioBlock, GpioPin, Input, InputPin, IntTrigger,
    OpenDrain, OpenDrainMode, Output, OutputPin, StatefulOutputPin,
};
