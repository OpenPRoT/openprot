// Licensed under the Apache-2.0 license

//! Blocking (synchronous) HAL traits for OpenPRoT
//!
//! This crate provides a blocking (synchronous) hardware abstraction layer (HAL) for
//! OpenPRoT-compliant platforms. It includes platform-specific modules such as reset
//! control and re-exports traits from `embedded-hal` 1.0 for common hardware interfaces
//! like SPI, I2C, GPIO, and delays.
//!
//! The goal is to offer a unified, safe, and no_std-compatible interface for embedded
//! development across multiple chip families.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Cryptographic digest operations (hashing)
pub mod digest;
/// Gpio port module
pub mod gpio_port;
/// Message Authentication Code (MAC) traits and implementations
pub mod mac;
/// Reset and clocking traits for OpenPRoT HAL
pub mod system_control;

// Re-export embedded-hal 1.0 traits
pub use embedded_hal::delay::DelayNs;
pub use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
pub use embedded_hal::i2c::{I2c, SevenBitAddress, TenBitAddress};
pub use embedded_hal::spi::{SpiBus, SpiDevice};
