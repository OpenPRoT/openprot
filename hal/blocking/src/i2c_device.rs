// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # I2C Device Implementation Traits
//!
//! This module provides application-level traits for implementing I2C devices such as sensors,
//! EEPROMs, display controllers, ADCs, DACs, and other peripheral devices that respond
//! to I2C master/controller requests.
//!
//! ## Purpose
//!
//! These traits focus on the **application logic** of how devices should respond to I2C
//! transactions initiated by a master/controller. They are **not** for controlling I2C
//! hardware controllers - that's handled by hardware abstraction layer (HAL) traits.
//!
//! ## Layer Distinction
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Application Layer                            │
//! │  i2c_device.rs - Implement device behavior (this module)       │
//! │  Examples: Sensor readings, EEPROM storage, register access    │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                Hardware Abstraction Layer                      │
//! │  i2c_hardware.rs - Control I2C controller peripherals          │
//! │  Examples: Buffer management, interrupt handling, bus control   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Design Philosophy
//!
//! The traits in this module are designed around **device behavior patterns**:
//!
//! - **Event-driven**: Devices respond to master-initiated transactions
//! - **Callback-based**: Use `on_*` methods to handle different transaction types
//! - **Stateful**: Devices can maintain internal state between transactions
//! - **Protocol-aware**: Support common I2C device patterns (register access, etc.)
//!
//! ## Common Use Cases
//!
//! ### Temperature Sensor
//! ```rust,ignore
//! struct TemperatureSensor {
//!     temperature: f32,
//!     address: u8,
//! }
//!
//! impl I2CCoreTarget for TemperatureSensor {
//!     fn on_read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
//!         let temp_bytes = self.temperature.to_le_bytes();
//!         buffer[..4].copy_from_slice(&temp_bytes);
//!         Ok(4)
//!     }
//! }
//! ```
//!
//! ### EEPROM Device
//! ```rust,ignore
//! struct EepromDevice {
//!     memory: [u8; 256],
//!     address_pointer: u8,
//! }
//!
//! impl WriteTarget for EepromDevice {
//!     fn on_write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
//!         if let Some(&addr) = data.first() {
//!             self.address_pointer = addr;
//!             // Write remaining data to memory starting at addr
//!             for (i, &byte) in data[1..].iter().enumerate() {
//!                 if let Some(mem_slot) = self.memory.get_mut(addr as usize + i) {
//!                     *mem_slot = byte;
//!                 }
//!             }
//!         }
//!         Ok(())
//!     }
//! }
//!
//! impl ReadTarget for EepromDevice {
//!     fn on_read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
//!         let start = self.address_pointer as usize;
//!         let end = (start + buffer.len()).min(self.memory.len());
//!         let bytes_to_read = end - start;
//!         buffer[..bytes_to_read].copy_from_slice(&self.memory[start..end]);
//!         Ok(bytes_to_read)
//!     }
//! }
//! ```
//!
//! ## Trait Overview
//!
//! The traits are organized in a composable hierarchy:
//!
//! - `I2CCoreTarget`: Core transaction lifecycle (required for all devices)
//! - `WriteTarget`: Handle write operations from master
//! - `ReadTarget`: Handle read operations from master  
//! - `WriteReadTarget`: Handle combined write-read transactions
//! - `RegisterAccess`: Higher-level register-based access patterns
//! - `I2CTarget`: Convenience trait combining all capabilities
//!
//! ## Transaction Flow
//!
//! A typical I2C transaction from the device perspective:
//!
//! 1. **Address Match**: `on_address_match(address)` - Should this device respond?
//! 2. **Transaction Start**: `on_transaction_start(direction, repeated)` - Initialize for transaction
//! 3. **Data Phase**: `on_write(data)` or `on_read(buffer)` - Handle the actual data
//! 4. **Transaction End**: `on_stop()` - Clean up after transaction
//!
//! ## Error Handling
//!
//! All traits use associated `Error` types that must implement `embedded_hal::i2c::Error`,
//! providing standard I2C error conditions while allowing device-specific error extensions.

#![allow(clippy::doc_overindented_list_items)]
use embedded_hal::i2c::ErrorType as I2CErrorType;

/// Transaction response
pub enum TransactionResponse {
    /// For read transactions with immediate data
    ByteToSend(u8),
    /// For transactions without immediate data
    NoImmediateData,
}

/// Direction of an I2C transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionDirection {
    /// Write transaction (controller writing to target).
    Write,
    /// Read transaction (controller reading from target).
    Read,
}

/// A convenience trait alias that represents a fully-featured I2C device implementation.
///
/// This trait combines all the core and extended I2C device traits into a single interface:
///
/// - [`I2CCoreTarget`]: Handles transaction lifecycle and address matching.
/// - [`ReadTarget`]: Supports reading data from the device.
/// - [`WriteTarget`]: Supports writing data to the device.
/// - [`WriteReadTarget`]: Supports combined write-read transactions.
/// - [`RegisterAccess`]: Supports register-level read/write operations.
///
/// Implementing this trait means the device is capable of handling all standard I2C device behaviors,
/// making it suitable for use in generic drivers or frameworks that require full I2C functionality.
///
/// ## When to Use
///
/// Use this trait when you need a device that supports all I2C interaction patterns:
/// - Simple read/write operations
/// - Register-based access (common for sensors, configuration devices)
/// - Complex transaction sequences
/// - Generic device drivers that work with multiple device types
///
/// ## Implementation Pattern
///
/// Most implementations will implement the individual traits and get `I2CTarget` automatically:
///
/// ```rust,ignore
/// struct MyComplexDevice {
///     registers: [u8; 256],
///     current_register: u8,
/// }
///
/// impl I2CCoreTarget for MyComplexDevice { /* ... */ }
/// impl ReadTarget for MyComplexDevice { /* ... */ }
/// impl WriteTarget for MyComplexDevice { /* ... */ }
/// impl WriteReadTarget for MyComplexDevice { /* ... */ }
/// impl RegisterAccess for MyComplexDevice { /* ... */ }
///
/// // Now MyComplexDevice automatically implements I2CTarget
/// fn use_any_device<T: I2CTarget>(device: &mut T) {
///     // Can use all I2C device capabilities
/// }
/// ```
pub trait I2CTarget:
    I2CCoreTarget + ReadTarget + WriteTarget + WriteReadTarget + RegisterAccess
{
}

impl<T> I2CTarget for T where
    T: I2CCoreTarget + ReadTarget + WriteTarget + WriteReadTarget + RegisterAccess
{
}

/// Trait representing core I2C device behavior.
///
/// This trait defines the fundamental methods that an I2C device must implement to handle
/// transactions initiated by an I2C master/controller. It covers the essential lifecycle
/// of I2C transactions from the device's perspective.
///
/// ## Core Responsibilities
///
/// - **Address matching**: Decide whether to respond to a specific I2C address
/// - **Transaction initialization**: Set up device state for incoming transactions  
/// - **Transaction lifecycle**: Handle start conditions, repeated starts, and stop conditions
///
/// ## Implementation Notes
///
/// This trait focuses on **device logic**, not hardware control. Implementations should:
/// - Maintain device state (registers, memory, sensor readings, etc.)
/// - Implement device-specific address matching logic
/// - Handle transaction setup and teardown
/// - Prepare for data exchange (actual data handling is in `ReadTarget`/`WriteTarget`)
///
/// For hardware I2C controller management, use hardware abstraction layer traits instead.
pub trait I2CCoreTarget: I2CErrorType {
    /// Initialize the target with a specific address.
    fn init(&mut self, address: u8) -> Result<(), Self::Error>;

    /// Called when a new I2C transaction begins.
    ///
    /// This method is invoked at the start of a transaction initiated by the controller (master).
    /// It provides a flag indicating whether the transaction is a repeated start, which allows
    /// the target (slave) device to preserve or reset internal state accordingly.
    ///
    /// # Arguments
    ///
    /// * `direction` - Indicates whether the upcoming transaction is a Read or Write.
    /// * `repeated`  - A boolean flag indicating whether this transaction is a repeated start (`true`)
    ///                 or a fresh start (`false`). A repeated start means the controller has not
    ///                 released the bus between transactions.
    ///
    /// # Return
    ///
    /// * `Ok(TransactionResponse::ByteToSend(byte))` - For read transactions, the first byte to send to the controller
    /// * `Ok(TransactionResponse::NoImmediateData)` - No immediate data to send or this is a write transaction
    /// * `Err(e)` - An error occurred during transaction setup
    /// # Usage Model
    ///
    /// This method is distinct from `on_address_match(address: u8) -> bool`:
    ///
    /// - `on_address_match` is called during the address phase to decide whether the target should respond.
    /// - `on_transaction_start` is called after the address is matched and before the data phase begins.
    ///
    /// ## Typical Use Cases:
    /// - Reset internal state if `repeated == false`.
    /// - Preserve context (e.g., register pointer) if `repeated == true`.
    /// - Prepare buffers or state machines for read/write.
    ///
    /// ## When it's called:
    /// - After `on_address_match` returns `true`.
    /// - Before `on_read` or `on_write` is invoked.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn on_transaction_start(&mut self, direction: TransactionDirection, repeated: bool) {
    ///     if repeated {
    ///         // Continue using previous state
    ///     } else {
    ///         // Reset internal state
    ///     }
    /// }
    /// ```
    fn on_transaction_start(
        &mut self,
        direction: TransactionDirection,
        repeated: bool,
    ) -> Result<TransactionResponse, Self::Error>;

    /// Optional: handle stop condition or reset.
    ///
    /// This method is called when the master sends a stop condition, indicating the end of a transaction.
    fn on_stop(&mut self);

    /// Optional: handle address match event.
    ///
    /// # Arguments
    ///
    /// * `address` - The address sent by the master.
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if the address matches the target's address, `false` otherwise.
    fn on_address_match(&mut self, address: u8) -> bool;
}

/// Trait for I2C targets that support write operations.
pub trait WriteTarget: I2CCoreTarget {
    /// Called when the master initiates a write to this target.
    ///
    /// # Arguments
    ///
    /// * `data` - A slice containing the data to be written to the target.
    ///
    /// # Returns
    ///
    /// * `Result<(), I2CError>` - Returns `Ok(())` if the write is successful, or an `I2CError` if an error occurs.
    fn on_write(&mut self, data: &[u8]) -> Result<(), Self::Error>;
}

/// Trait for I2C targets that support read operations.
pub trait ReadTarget: I2CCoreTarget {
    /// Called when the master initiates a read from this target.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable slice where the read data will be stored.
    ///
    /// # Returns
    ///
    /// * `Result<usize, I2CError>` - Returns the number of bytes read if successful, or an `I2CError` if an error occurs.
    fn on_read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
}

/// Trait for I2C targets that support combined write-read transactions.
pub trait WriteReadTarget: WriteTarget + ReadTarget {
    /// Performs a combined write-read transaction on the device.
    ///
    /// This method writes data from `write_buffer` and then reads data into `read_buffer`
    /// in a single, atomic operation (if supported by the underlying hardware).
    ///
    /// # Parameters
    /// - `write_buffer`: The buffer containing data to write.
    /// - `read_buffer`: The buffer to store the data read from the device.
    ///
    /// # Returns
    /// - `Ok(usize)`: The number of bytes read into `read_buffer`.
    /// - `Err(Self::Error)`: If the transaction fails.
    ///
    /// # Errors
    /// This function returns an error if the write or read operation fails.
    ///
    /// # Example
    /// ```ignore
    /// device.on_write_read(&mut [0x01, 0x02], &mut [0; 4])?;
    /// ```
    fn on_write_read(
        &mut self,
        write_buffer: &mut [u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        self.on_write(write_buffer)?;
        self.on_read(read_buffer)
    }
}

/// Provides register-level access to a device that supports both reading and writing operations.
///
/// This trait combines the capabilities of `WriteTarget` and `ReadTarget` to provide a higher-level
/// interface for accessing device registers. It simplifies common operations like reading and writing
/// individual registers by address.
pub trait RegisterAccess: WriteTarget + ReadTarget {
    /// Writes a single byte to a specified register address.
    ///
    /// # Parameters
    ///
    /// * `address` - The register address to write to
    /// * `data` - The byte value to write to the register
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The write operation was successful
    /// * `Err(e)` - An error occurred during the write operation
    fn write_register(&mut self, address: u8, data: u8) -> Result<(), Self::Error>;

    /// Reads data from a specified register address into a buffer.
    ///
    /// # Parameters
    ///
    /// * `address` - The register address to read from
    /// * `buffer` - The buffer to store the read data
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes successfully read into the buffer
    /// * `Err(e)` - An error occurred during the read operation
    fn read_register(&mut self, address: u8, buffer: &mut [u8]) -> Result<usize, Self::Error>;
}
