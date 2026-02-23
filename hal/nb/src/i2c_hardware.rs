// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Non-blocking I2C Hardware Abstraction Traits
//!
//! This module defines non-blocking traits for I2C hardware abstraction, specifically
//! for polling-based and interrupt-driven slave operations that don't block the caller.
//!
//! These traits complement the blocking traits in `openprot-hal-blocking` by providing
//! non-blocking alternatives suitable for async code, main loops, and interrupt handlers.
//!
//! # Examples
//!
//! ## Polling-based Event Handling
//!
//! ```rust,no_run
//! use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
//! use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
//!
//! fn poll_slave_events<T: I2cSlaveEventPolling>(slave: &mut T) -> Result<(), T::Error> {
//!     // Non-blocking check for events in main loop
//!     while let Some(event) = slave.poll_slave_events()? {
//!         match event {
//!             I2cSEvent::SlaveWrReq => {
//!                 println!("Master wants to write to us");
//!                 slave.handle_slave_event(event)?;
//!             },
//!             I2cSEvent::SlaveRdReq => {
//!                 println!("Master wants to read from us");
//!                 slave.handle_slave_event(event)?;
//!             },
//!             I2cSEvent::SlaveStop => {
//!                 println!("Transaction complete");
//!                 slave.handle_slave_event(event)?;
//!             },
//!             _ => {
//!                 slave.handle_slave_event(event)?;
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Interrupt-driven Event Handling
//!
//! ```rust,no_run
//! use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
//! use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
//!
//! // Called from interrupt service routine
//! fn i2c_slave_isr<T: I2cSlaveEventPolling>(slave: &mut T) {
//!     // Check specific events without blocking
//!     if slave.is_event_pending(I2cSEvent::SlaveWrReq).unwrap_or(false) {
//!         let _ = slave.handle_slave_event(I2cSEvent::SlaveWrReq);
//!     }
//!     
//!     if slave.is_event_pending(I2cSEvent::SlaveRdReq).unwrap_or(false) {
//!         let _ = slave.handle_slave_event(I2cSEvent::SlaveRdReq);
//!     }
//! }
//! ```
//!
//! ## Non-blocking Slave Implementation
//!
//! ```rust,no_run
//! use openprot_hal_nb::i2c_hardware::I2cSlaveNonBlocking;
//! use embedded_hal::i2c::SevenBitAddress;
//!
//! fn setup_nonblocking_slave<T: I2cSlaveNonBlocking<SevenBitAddress>>(
//!     mut slave: T
//! ) -> Result<(), T::Error> {
//!     // Configure slave address
//!     slave.configure_slave_address(0x42)?;
//!     slave.enable_slave_mode()?;
//!     
//!     // Enable interrupts for non-blocking operation
//!     slave.enable_slave_interrupts(0xFF); // Enable all slave interrupts
//!     
//!     // The slave is now ready for non-blocking operations
//!     // Events will be handled via polling or interrupts
//!     Ok(())
//! }
//! ```

use embedded_hal::i2c::{AddressMode, SevenBitAddress};
use openprot_hal_blocking::i2c_hardware::slave::{
    I2cSEvent, I2cSlaveBuffer, I2cSlaveCore, I2cSlaveInterrupts,
};

/// Non-blocking slave event handling (async/polling pattern)
///
/// This trait provides non-blocking event operations suitable for async code,
/// main loops, or interrupt-driven architectures. All operations return
/// immediately without blocking the caller.
///
/// # Examples
///
/// ## Basic Polling Loop
///
/// ```rust,no_run
/// use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
/// use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
///
/// fn handle_i2c_events<T: I2cSlaveEventPolling>(slave: &mut T) -> Result<(), T::Error> {
///     // Check for events without blocking
///     if let Some(event) = slave.poll_slave_events()? {
///         match event {
///             I2cSEvent::SlaveWrReq => {
///                 // Master wants to write - prepare to receive
///                 slave.handle_slave_event(event)?;
///             },
///             I2cSEvent::SlaveRdReq => {
///                 // Master wants to read - prepare data
///                 slave.handle_slave_event(event)?;
///             },
///             _ => slave.handle_slave_event(event)?,
///         }
///     }
///     Ok(())
/// }
/// ```
///
/// ## Interrupt Service Routine
///
/// ```rust,no_run
/// use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
/// use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
///
/// // Fast ISR that doesn't block
/// fn i2c_isr<T: I2cSlaveEventPolling>(slave: &mut T) {
///     // Quick event check
///     if slave.is_event_pending(I2cSEvent::SlaveWrReq).unwrap_or(false) {
///         let _ = slave.handle_slave_event(I2cSEvent::SlaveWrReq);
///     }
/// }
/// ```
pub trait I2cSlaveEventPolling<A: AddressMode = SevenBitAddress>: I2cSlaveInterrupts<A> {
    /// Check for pending slave events without blocking
    ///
    /// Returns the next available slave event if one is pending, or None
    /// if no events are waiting. This is useful for polling-based event
    /// handling or in main loops that need to be non-blocking.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(event))` - An event is pending
    /// - `Ok(None)` - No events are currently pending
    /// - `Err(error)` - Hardware error occurred
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
    /// use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
    ///
    /// fn check_events<T: I2cSlaveEventPolling>(slave: &mut T) -> Result<(), T::Error> {
    ///     if let Some(event) = slave.poll_slave_events()? {
    ///         println!("Event received: {:?}", event);
    ///     } else {
    ///         println!("No events pending");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn poll_slave_events(&mut self) -> Result<Option<I2cSEvent>, Self::Error>;

    /// Handle a specific slave event (called from ISR or event loop)
    ///
    /// Processes a slave event and performs any necessary hardware actions.
    /// This method encapsulates the event-specific logic and can be called
    /// from interrupt handlers or main event loops. Always returns immediately.
    ///
    /// # Arguments
    ///
    /// * `event` - The slave event to handle
    ///
    /// # Errors
    ///
    /// Returns an error if the hardware operation fails or the event cannot be handled.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
    /// use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
    ///
    /// fn handle_event<T: I2cSlaveEventPolling>(slave: &mut T) -> Result<(), T::Error> {
    ///     slave.handle_slave_event(I2cSEvent::SlaveWrReq)?;
    ///     println!("Write request handled");
    ///     Ok(())
    /// }
    /// ```
    fn handle_slave_event(&mut self, event: I2cSEvent) -> Result<(), Self::Error>;

    /// Non-blocking check if a specific event is pending
    ///
    /// Returns true if the specified event is currently pending, false otherwise.
    /// Useful for checking specific conditions without consuming the event.
    ///
    /// # Arguments
    ///
    /// * `event` - The specific event type to check for
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - The specified event is pending
    /// - `Ok(false)` - The specified event is not pending
    /// - `Err(error)` - Hardware error occurred during check
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use openprot_hal_nb::i2c_hardware::I2cSlaveEventPolling;
    /// use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
    ///
    /// fn check_specific_event<T: I2cSlaveEventPolling>(slave: &T) -> Result<(), T::Error> {
    ///     if slave.is_event_pending(I2cSEvent::SlaveWrReq)? {
    ///         println!("Write request is pending");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn is_event_pending(&self, event: I2cSEvent) -> Result<bool, Self::Error>;
}

/// Complete non-blocking slave implementation
///
/// This trait represents a full non-blocking slave implementation that supports
/// all non-blocking slave operations. Perfect for interrupt-driven or
/// polling-based implementations that cannot afford to block.
///
/// This is a composite trait that automatically implements for any type that
/// provides all the necessary slave functionality with non-blocking event handling.
///
/// # Examples
///
/// ## Using a Non-blocking Slave
///
/// ```rust,no_run
/// use openprot_hal_nb::i2c_hardware::I2cSlaveNonBlocking;
/// use embedded_hal::i2c::SevenBitAddress;
///
/// fn configure_slave<T: I2cSlaveNonBlocking<SevenBitAddress>>(
///     mut slave: T
/// ) -> Result<(), T::Error> {
///     // Configure slave address
///     slave.configure_slave_address(0x42)?;
///     
///     // Enable slave mode
///     slave.enable_slave_mode()?;
///     
///     // Set up buffers
///     let tx_data = [0x01, 0x02, 0x03];
///     slave.write_slave_response(&tx_data)?;
///     
///     // Enable interrupts for non-blocking operation
///     slave.enable_slave_interrupts(0xFF);
///     
///     Ok(())
/// }
/// ```
///
/// ## Main Loop with Non-blocking Slave
///
/// ```rust,no_run
/// use openprot_hal_nb::i2c_hardware::I2cSlaveNonBlocking;
/// use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
///
/// fn main_loop<T: I2cSlaveNonBlocking>(mut slave: T) -> Result<(), T::Error> {
///     loop {
///         // Handle I2C events without blocking
///         while let Some(event) = slave.poll_slave_events()? {
///             slave.handle_slave_event(event)?;
///         }
///         
///         // Do other work...
///         
///         // Check if we received data
///         if slave.rx_buffer_count()? > 0 {
///             let mut buffer = [0u8; 32];
///             let count = slave.read_slave_buffer(&mut buffer)?;
///             println!("Received {} bytes", count);
///         }
///     }
/// }
/// ```
pub trait I2cSlaveNonBlocking<A: AddressMode = SevenBitAddress>:
    I2cSlaveCore<A> + I2cSlaveBuffer<A> + I2cSlaveEventPolling<A>
{
}

/// Blanket implementation: any type implementing core + buffer + polling events gets non-blocking slave
impl<T, A: AddressMode> I2cSlaveNonBlocking<A> for T where
    T: I2cSlaveCore<A> + I2cSlaveBuffer<A> + I2cSlaveEventPolling<A>
{
}
