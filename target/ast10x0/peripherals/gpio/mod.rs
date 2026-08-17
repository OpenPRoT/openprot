// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO peripheral: pin-token-driven, `unsafe`-free access; `.into_gpio()` binds a pin token to
//! a handle whose route `scu::route` applies, `bind_gpio` binds a pin privileged init already muxed.

mod map;

pub(crate) use map::{ABCD, EFGH, IJKL};
pub use openprot_hal::gpio::{bind_gpio, GpioRole, IntoGpio};
