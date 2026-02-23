// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

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
/// ECDSA digital signature operations
pub mod ecdsa;
/// Gpio port module
pub mod gpio_port;
/// I2C device implementation traits (application layer)
pub mod i2c_device;
/// I2C hardware controller traits (hardware abstraction layer)
pub mod i2c_hardware;
/// Message Authentication Code (MAC) traits and implementations
pub mod mac;
/// Reset and clocking traits for OpenPRoT HAL
pub mod system_control;

/// Key management traits
pub mod key_vault;

/// Symmetric cipher abstractions with zero-copy support
pub mod cipher;

// Re-export embedded-hal 1.0 traits
pub use embedded_hal::delay::DelayNs;
pub use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
pub use embedded_hal::i2c::{I2c, SevenBitAddress, TenBitAddress};
pub use embedded_hal::spi::{SpiBus, SpiDevice};

// Re-export system control traits
pub use system_control::{ClockControl, ResetControl, SystemControl};
