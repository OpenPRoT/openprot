// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1060 I2C bare-metal driver core
//!
//! This module provides a portable, hardware-abstraction layer for the AST1060 I2C controller.
//! It is designed to be usable in both bare-metal and RTOS environments without requiring
//! OS-specific dependencies.
//!
//! # Features
//!
//! - Multi-master support
//! - Master and slave (target) mode
//! - **Buffer mode with 32-byte hardware FIFO**
//! - **DMA mode with up to 4096-byte transfers** (requires non-cached SRAM buffer)
//! - Clock stretching and bus recovery
//! - `SMBus` alert support
//! - Configurable speeds: Standard (100kHz), Fast (400kHz), Fast-plus (1MHz)
//!
//! # Architecture
//!
//! The driver is split into focused modules:
//!
//! - `controller`: Core hardware abstraction and initialization
//! - `master`: Master mode operations (read/write)
//! - `slave`: Slave (target) mode operations
//! - `transfer`: Low-level transfer state machine
//! - `timing`: Clock timing configuration
//! - `recovery`: Bus recovery mechanisms
//! - `types`: Core type definitions
//! - `error`: Error types
//! - `constants`: Hardware register constants
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use ast10x0_peripherals::i2c::*;
//! use ast1060_pac;
//!
//! // Initialize I2C global registers ONCE before any controller use.
//! init_i2c_global();
//!
//! // Build the I2C driver around UART1's register block. The yield
//! // closure is invoked between status polls inside wait_completion.
//! let config = I2cConfig::default();
//! let mut i2c = unsafe {
//!     Ast1060I2c::new(
//!         ast1060_pac::I2c1::ptr(),
//!         ast1060_pac::I2cbuff1::ptr(),
//!         config,
//!         |_ns| core::hint::spin_loop(),
//!     )?
//! };
//!
//! // Perform a master read.
//! let mut data = [0u8; 4];
//! i2c.read(0x50, &mut data)?;
//! ```

mod constants;
mod controller;
mod error;
mod global;
mod hal_impl;
mod hal_slave_impl;
mod master;
mod recovery;
mod registers;
mod slave;
mod timing;
mod transfer;
mod types;

// Re-export public API
pub use constants::*;
pub use controller::Ast1060I2c;
pub use error::I2cError;
pub use global::init_i2c_global;
pub use registers::Ast1060I2cRegisters;
pub use slave::{SlaveBuffer, SlaveConfig, SlaveEvent};
pub use types::*;

// Re-export HAL implementations for external use
#[allow(unused_imports)]
pub use hal_impl::*;
