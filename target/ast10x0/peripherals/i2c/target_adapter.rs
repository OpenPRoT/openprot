// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C Target (Slave) Adapter
//!
//! This module provides an adapter that bridges the low-level [`Ast1060I2c`]
//! slave API with a user-supplied `TargetCallbacks` implementation. The
//! adapter polls hardware events from the controller and dispatches them
//! to the callback methods, keeping the hardware layer (slave.rs)
//! decoupled from any particular target abstraction.
//!
//! # Example
//!
//! ```rust,ignore
//! use ast10x0_peripherals::i2c::target_adapter::{TargetAdapter, TargetCallbacks};
//!
//! struct MyTarget { /* ... */ }
//! impl TargetCallbacks for MyTarget { /* ... */ }
//!
//! let mut adapter = TargetAdapter::new(i2c_controller, MyTarget { /* ... */ });
//! adapter.configure(0x42)?;
//!
//! // In an interrupt handler or poll loop:
//! adapter.process_events()?;
//! ```

use super::controller::Ast1060I2c;
use super::error::I2cError;
use super::slave::{SlaveConfig, SlaveEvent};

/// Internal hardware buffer for adapter operations
const ADAPTER_BUFFER_SIZE: usize = 32;

/// Target adapter that bridges hardware events to [`TargetCallbacks`].
///
/// Wraps an [`Ast1060I2c`] controller and a user-provided callback
/// implementation, mapping hardware slave events to the appropriate
/// callback methods.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the I2C register references
/// * `T` - User's [`TargetCallbacks`] implementation
/// * `Y` - Yield closure threaded through the underlying [`Ast1060I2c`]
pub struct TargetAdapter<'a, T, Y: FnMut(u32)> {
    /// The underlying hardware controller
    i2c: Ast1060I2c<'a, Y>,
    /// User's target implementation
    target: T,
    /// Internal receive buffer
    rx_buffer: [u8; ADAPTER_BUFFER_SIZE],
    /// Internal transmit buffer
    tx_buffer: [u8; ADAPTER_BUFFER_SIZE],
    /// Flag indicating we're in a transaction
    in_transaction: bool,
}

impl<'a, T, Y: FnMut(u32)> TargetAdapter<'a, T, Y> {
    /// Create a new target adapter.
    ///
    /// # Arguments
    ///
    /// * `i2c` - The I2C controller configured for the desired bus
    /// * `target` - User's target implementation
    ///
    /// # Returns
    ///
    /// A new `TargetAdapter` instance ready to be configured.
    pub fn new(i2c: Ast1060I2c<'a, Y>, target: T) -> Self {
        Self {
            i2c,
            target,
            rx_buffer: [0u8; ADAPTER_BUFFER_SIZE],
            tx_buffer: [0u8; ADAPTER_BUFFER_SIZE],
            in_transaction: false,
        }
    }

    /// Get a reference to the underlying target implementation.
    pub fn target(&self) -> &T {
        &self.target
    }

    /// Get a mutable reference to the underlying target implementation.
    pub fn target_mut(&mut self) -> &mut T {
        &mut self.target
    }

    /// Get a reference to the underlying I2C controller.
    pub fn controller(&self) -> &Ast1060I2c<'a, Y> {
        &self.i2c
    }

    /// Get a mutable reference to the underlying I2C controller.
    pub fn controller_mut(&mut self) -> &mut Ast1060I2c<'a, Y> {
        &mut self.i2c
    }

    /// Decompose the adapter into its parts.
    ///
    /// Returns the I2C controller and target implementation.
    pub fn into_parts(self) -> (Ast1060I2c<'a, Y>, T) {
        (self.i2c, self.target)
    }
}

impl<'a, T, Y: FnMut(u32)> TargetAdapter<'a, T, Y>
where
    T: TargetCallbacks,
{
    /// Configure the controller for target (slave) mode.
    ///
    /// # Arguments
    ///
    /// * `address` - The 7-bit I2C address to respond to
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if configuration fails.
    pub fn configure(&mut self, address: u8) -> Result<(), I2cError> {
        let config = SlaveConfig::new(address)?;
        self.i2c.configure_slave(&config)?;

        // Initialize the target with the address
        self.target
            .on_init(address)
            .map_err(|_| I2cError::SlaveError)?;

        self.in_transaction = false;
        Ok(())
    }

    /// Disable target mode.
    pub fn disable(&mut self) {
        self.i2c.disable_slave();
        self.in_transaction = false;
    }

    /// Process pending hardware events.
    ///
    /// This method should be called from an interrupt handler or poll loop.
    /// It reads hardware status, maps events to trait callbacks, and handles
    /// data transfer between hardware and the target implementation.
    ///
    /// # Returns
    ///
    /// `Ok(Some(event))` if an event was processed, `Ok(None)` if no events pending,
    /// or an error if processing failed.
    pub fn process_events(&mut self) -> Result<Option<SlaveEvent>, I2cError> {
        let Some(event) = self.i2c.handle_slave_interrupt() else {
            return Ok(None);
        };

        match event {
            SlaveEvent::AddressMatch => {
                if !self.in_transaction {
                    self.target.on_transaction_start(false);
                    self.in_transaction = true;
                }
                self.target.on_address_match();
            }

            SlaveEvent::ReadRequest => {
                // Master wants to read from us - get data from target
                let len = self
                    .target
                    .on_read(&mut self.tx_buffer)
                    .map_err(|_| I2cError::SlaveError)?;

                // Send data to master
                if len > 0 {
                    let to_send = len.min(ADAPTER_BUFFER_SIZE);
                    self.i2c.slave_write(&self.tx_buffer[..to_send])?;
                }
            }

            SlaveEvent::WriteRequest => {
                // Master wants to write to us - nothing to do here,
                // data will arrive in DataReceived
            }

            SlaveEvent::DataReceived { len } => {
                // Read data from hardware
                let read_len = self.i2c.slave_read(&mut self.rx_buffer)?;
                let actual_len = read_len.min(len);

                if actual_len > 0 {
                    // Pass data to target
                    self.target
                        .on_write(&self.rx_buffer[..actual_len])
                        .map_err(|_| I2cError::SlaveError)?;
                }
            }

            SlaveEvent::DataSent { len: _ } => {
                // Data was sent successfully - target can track if needed
                self.target.on_data_sent();
            }

            SlaveEvent::Stop => {
                self.target.on_stop();
                self.in_transaction = false;
            }
        }

        Ok(Some(event))
    }

    /// Poll for and process all pending events.
    ///
    /// Continues processing until no more events are available.
    ///
    /// # Returns
    ///
    /// The count of events processed, or an error if processing failed.
    pub fn process_all_events(&mut self) -> Result<usize, I2cError> {
        let mut count: usize = 0;
        while self.process_events()?.is_some() {
            count = count.saturating_add(1);
        }
        Ok(count)
    }
}

/// Target callback trait for adapter integration.
///
/// This trait defines the callbacks that the adapter invokes in response to
/// hardware events from the I2C controller in slave mode. Implementors plug
/// in their own logic for address-match, read, write, and stop events.
pub trait TargetCallbacks {
    /// Error type for callback operations
    type Error;

    /// Called when the target is initialized with an address.
    ///
    /// # Arguments
    ///
    /// * `address` - The 7-bit I2C address assigned to this target
    fn on_init(&mut self, address: u8) -> Result<(), Self::Error>;

    /// Called when a transaction starts.
    ///
    /// # Arguments
    ///
    /// * `repeated` - True if this is a repeated start condition
    fn on_transaction_start(&mut self, repeated: bool);

    /// Called when our address is matched.
    fn on_address_match(&mut self);

    /// Called when the master requests data (read operation).
    ///
    /// # Arguments
    ///
    /// * `buffer` - Buffer to fill with data to send to master
    ///
    /// # Returns
    ///
    /// Number of bytes written to buffer, or an error.
    fn on_read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Called when the master sends data (write operation).
    ///
    /// # Arguments
    ///
    /// * `data` - Data received from the master
    fn on_write(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Called when data has been sent to the master.
    fn on_data_sent(&mut self) {}

    /// Called when a stop condition is received.
    fn on_stop(&mut self);
}

