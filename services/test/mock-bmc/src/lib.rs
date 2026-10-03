// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A BMC that boots, or does not, on demand.
//!
//! The RoT drives a reset line and watches a ready line. This models what
//! sits on the other end of those two wires: hold the ready line low while
//! reset is asserted, and once reset is released, raise it after a delay,
//! or never. That is the whole vocabulary a boot-sequence test needs from
//! a managed device.
//!
//! Nothing here knows what the lines are made of. On hardware they are
//! GPIO; in a QEMU test both ends live in one image and the pins are
//! whatever the board wiring supplies. The model is generic over
//! `InputPin` and `OutputPin` so neither case is special.
//!
//! Time arrives as an argument. [`MockBmc::poll`] is given the current
//! millisecond count and never waits, so a caller can drive it from a
//! kernel event loop, or from a test that moves the clock by hand.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

mod boot;
mod error;
mod model;

pub use boot::{BootBehaviour, Phase};
pub use error::Error;
pub use model::MockBmc;

#[cfg(test)]
mod tests;
