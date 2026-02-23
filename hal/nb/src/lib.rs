// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Non-blocking HAL traits for OpenPRoT
//!
//! This crate re-exports embedded-hal-nb 1.0 traits for non-blocking, polling-based
//! hardware abstraction layer operations using the `nb` crate for error handling.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Non-blocking I2C hardware controller traits
pub mod i2c_hardware;

// Re-export nb and embedded-hal-nb 1.0 traits
pub use embedded_hal_nb::*;
pub use nb;
