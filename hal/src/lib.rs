// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mode-agnostic OpenPRoT HAL vocabulary: capability + role traits, and the pin-token generators.
//! Unlike the blocking/nb/async crates, nothing here is flavored by an execution mode.

#![no_std]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod field_mux;
pub mod gpio;
pub mod i2c;
pub mod i2c_cmd_words;
pub mod resource;
