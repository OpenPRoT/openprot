// Licensed under the Apache-2.0 license

//! # I2C Client API
//!
//! This crate provides the client-side API for interacting with the I2C service.
//! It defines the types, traits, and error handling for I2C operations.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────┐
//! │   Application       │
//! │  (sensor drivers,   │
//! │   etc.)             │
//! └─────────┬───────────┘
//!           │ uses I2cClient trait
//!           ▼
//! ┌─────────────────────┐
//! │   i2c-api           │◄── This crate
//! │  (types & traits)   │
//! └─────────┬───────────┘
//!           │ IPC (implementation specific)
//!           ▼
//! ┌─────────────────────┐
//! │   I2C Server        │
//! │  (actual hardware)  │
//! └─────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Controller mode**: Standard I2C controller operations via [`I2cClient`]
//! - **Target mode**: Respond to incoming transactions via [`I2cTargetClient`]
//! - **embedded-hal compatible**: Error types implement `embedded_hal::i2c::Error`
//!
//! ## Usage
//!
//! ### Controller Mode (Reading a Sensor)
//!
//! ```rust,ignore
//! use i2c_api::{I2cClient, I2cAddress, BusIndex};
//!
//! fn read_sensor<C: I2cClient>(client: &mut C) -> Result<[u8; 2], C::Error> {
//!     let address = I2cAddress::new(0x48)?;
//!     let bus = BusIndex::BUS_0;
//!     
//!     // Write register address, then read data
//!     let mut buffer = [0u8; 2];
//!     client.write_read(bus, address, &[0x00], &mut buffer)?;
//!     Ok(buffer)
//! }
//! ```
//!
//! ### Target Mode (MCTP Endpoint)
//!
//! ```rust,ignore
//! use i2c_api::{I2cTargetClient, I2cAddress, BusIndex, TargetMessage};
//! use core::time::Duration;
//!
//! fn mctp_handler<C: I2cTargetClient>(
//!     client: &mut C,
//!     bus: BusIndex,
//!     address: I2cAddress,
//! ) -> Result<(), C::Error> {
//!     client.configure_target_address(bus, address)?;
//!     client.enable_receive(bus)?;
//!     
//!     let mut messages = [TargetMessage::default(); 4];
//!     let count = client.wait_for_messages(bus, &mut messages, None)?;
//!     
//!     for msg in &messages[..count] {
//!         // Process MCTP message
//!     }
//!     Ok(())
//! }
//! ```

#![no_std]
#![warn(missing_docs)]

mod address;
mod client;
mod error;
mod operation;
mod target;
pub mod wire;

// Re-export address types
pub use address::{AddressError, I2cAddress};

// Re-export client traits and types
pub use client::{BusIndex, I2cClient, I2cClientBlocking};

// Re-export error types
pub use error::{I2cError, I2cErrorKind, NoAcknowledgeSource, ResponseCode};

// Re-export operation types
pub use operation::Operation;

// Re-export target mode types
pub use target::{I2cTargetClient, TargetMessage, TARGET_MESSAGE_MAX_LEN};
