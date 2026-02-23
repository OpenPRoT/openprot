// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # I2C Hardware Abstraction Traits
//!
//! This module defines composable traits for I2C hardware abstraction following
//! a clean separation of concerns. Each trait has a specific responsibility
//! and can be composed to build complete I2C functionality.
//!
//! ## Design Philosophy
//!
//! The traits are designed to be:
//! - **Composable**: Small, focused traits that can be combined
//! - **Flexible**: Different implementations can pick the traits they need
//! - **Extensible**: New functionality can be added without breaking existing code
//! - **Clear**: Each trait has a single, well-defined responsibility
//!
//! ## Trait Hierarchy
//!
//! ```text
//! I2cHardwareCore (foundation)
//!     ├── I2cMaster (master operations)
//!     └── slave module (blocking operations only)
//!         ├── I2cSlaveCore (basic slave setup)
//!         ├── I2cSlaveBuffer (data transfer)
//!         ├── I2cSlaveInterrupts (interrupt & status management)
//!         │   └── I2cSlaveEventSync (sync/blocking events)
//!         ├── Composite Traits:
//!         │   ├── I2cSlaveBasic (core + buffer)
//!         │   └── I2cSlaveSync (basic + sync events)
//!         └── I2cMasterSlave (master + sync slave)
//! ```
//!
//! For non-blocking slave operations, see `openprot-hal-nb::i2c_hardware`.

use embedded_hal::i2c::{AddressMode, Operation, SevenBitAddress};

/// Core I2C hardware interface providing basic operations
///
/// This is the foundation trait that all I2C hardware implementations must provide.
/// It contains only the most basic operations needed for any I2C controller.
pub trait I2cHardwareCore {
    /// Hardware-specific error type that implements embedded-hal error traits
    type Error: embedded_hal::i2c::Error + core::fmt::Debug;

    /// Hardware-specific configuration type for I2C initialization and setup
    type Config;

    /// I2C speed configuration type
    type I2cSpeed;

    /// Timing configuration type
    type TimingConfig;

    /// Initialize the I2C hardware with the given configuration
    fn init(&mut self, config: &mut Self::Config) -> Result<(), Self::Error>;

    /// Configure timing parameters (clock speed, setup/hold times)
    ///
    /// Takes timing parameters as input and returns the calculated clock source frequency.
    /// This provides type safety by making clear what is read vs. what is computed/returned.
    ///
    /// # Arguments
    ///
    /// * `speed` - Target I2C bus speed (Standard, Fast, FastPlus, etc.)
    /// * `timing` - Timing configuration parameters for setup/hold times
    ///
    /// # Returns
    ///
    /// Returns the actual calculated clock source frequency in Hz.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested timing cannot be achieved with the
    /// available hardware clock sources or if parameters are invalid.
    fn configure_timing(
        &mut self,
        speed: Self::I2cSpeed,
        timing: &Self::TimingConfig,
    ) -> Result<u32, Self::Error>;

    /// Enable hardware interrupts with the specified mask
    fn enable_interrupts(&mut self, mask: u32);

    /// Clear hardware interrupts with the specified mask
    fn clear_interrupts(&mut self, mask: u32);

    /// Handle hardware interrupt events (called from ISR)
    fn handle_interrupt(&mut self);

    /// Attempt to recover the I2C bus from stuck conditions
    fn recover_bus(&mut self) -> Result<(), Self::Error>;
}

/// I2C Master mode operations
///
/// This trait extends the core interface with master-specific functionality.
/// Implementations provide the actual I2C master protocol operations.
pub trait I2cMaster<A: AddressMode = SevenBitAddress>: I2cHardwareCore {
    /// Write data to a slave device at the given address
    fn write(&mut self, addr: A, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Read data from a slave device at the given address
    fn read(&mut self, addr: A, buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// Combined write-then-read operation with restart condition
    fn write_read(&mut self, addr: A, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// Execute a sequence of I2C operations as a single atomic transaction
    fn transaction_slice(
        &mut self,
        addr: A,
        ops_slice: &mut [Operation<'_>],
    ) -> Result<(), Self::Error>;
}

/// I2C Slave/Target mode functionality
///
/// This module contains all slave-related traits decomposed into
/// focused responsibilities for better composability.
pub mod slave {
    use super::*;

    /// I2C slave events that can occur during slave operations
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum I2cSEvent {
        /// Master is requesting to read from slave
        SlaveRdReq,
        /// Master is requesting to write to slave
        SlaveWrReq,
        /// Slave read operation is in progress
        SlaveRdProc,
        /// Slave has received write data from master
        SlaveWrRecvd,
        /// Stop condition received
        SlaveStop,
    }

    /// Status information for I2C slave operations
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct SlaveStatus {
        /// Whether slave mode is currently enabled
        pub enabled: bool,
        /// Current slave address (if enabled)
        pub address: Option<u8>,
        /// Whether there's data available to read
        pub data_available: bool,
        /// Number of bytes in receive buffer
        pub rx_buffer_count: usize,
        /// Number of bytes in transmit buffer
        pub tx_buffer_count: usize,
        /// Last slave event that occurred
        pub last_event: Option<I2cSEvent>,
        /// Whether an error condition exists
        pub error: bool,
    }

    /// Core slave functionality - address configuration and mode control
    ///
    /// This trait provides the fundamental slave operations that all slave
    /// implementations need: setting slave address and enabling/disabling slave mode.
    /// This is the minimal trait for any I2C slave implementation.
    pub trait I2cSlaveCore<A: AddressMode = SevenBitAddress>: super::I2cHardwareCore {
        /// Configure the slave address for this I2C controller
        fn configure_slave_address(&mut self, addr: A) -> Result<(), Self::Error>;

        /// Enable slave mode operation
        fn enable_slave_mode(&mut self) -> Result<(), Self::Error>;

        /// Disable slave mode and return to master-only operation
        fn disable_slave_mode(&mut self) -> Result<(), Self::Error>;

        /// Check if slave mode is currently enabled
        fn is_slave_mode_enabled(&self) -> bool;

        /// Get the currently configured slave address
        fn slave_address(&self) -> Option<A>;
    }

    /// Slave buffer operations - data transfer with master
    ///
    /// This trait handles the actual data exchange between slave and master.
    /// Separate from core to allow different buffer management strategies.
    /// Implementations can choose different buffering approaches (ring buffer,
    /// simple array, DMA, etc.) while maintaining the same interface.
    pub trait I2cSlaveBuffer<A: AddressMode = SevenBitAddress>: I2cSlaveCore<A> {
        /// Read received data from the slave buffer
        ///
        /// Returns the number of bytes actually read. The buffer is filled
        /// with data received from the master during the last transaction.
        /// This is typically called after detecting a slave write event.
        fn read_slave_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

        /// Write response data to the slave transmit buffer
        ///
        /// Prepares data to be sent to the master during the next read transaction.
        /// The data will be transmitted when the master requests it.
        fn write_slave_response(&mut self, data: &[u8]) -> Result<(), Self::Error>;

        /// Non-blocking check for available slave data
        ///
        /// Returns Some(length) if data is available to read, None otherwise.
        /// This is useful for polling-based implementations or to check
        /// before calling read_slave_buffer.
        fn poll_slave_data(&mut self) -> Result<Option<usize>, Self::Error>;

        /// Clear the slave receive buffer and reset state
        ///
        /// Clears any pending received data and resets the buffer to
        /// a clean state. Useful for error recovery or initialization.
        fn clear_slave_buffer(&mut self) -> Result<(), Self::Error>;

        /// Get available space in transmit buffer
        ///
        /// Returns the number of bytes that can be written to the transmit
        /// buffer without overflowing. Useful for flow control.
        fn tx_buffer_space(&self) -> Result<usize, Self::Error>;

        /// Get number of bytes available in receive buffer
        ///
        /// Returns the current count of bytes waiting to be read from
        /// the receive buffer.
        fn rx_buffer_count(&self) -> Result<usize, Self::Error>;
    }

    /// Slave interrupt and status management
    ///
    /// Common interrupt and status operations shared by both async and sync event patterns.
    /// This provides the foundation for event-driven slave operations.
    pub trait I2cSlaveInterrupts<A: AddressMode = SevenBitAddress>: I2cSlaveCore<A> {
        /// Enable slave-specific hardware interrupts
        ///
        /// Configures the hardware to generate interrupts for slave events.
        /// The mask parameter specifies which interrupt sources to enable.
        /// Common interrupts include: address match, data received, stop condition, etc.
        fn enable_slave_interrupts(&mut self, mask: u32);

        /// Clear slave-specific hardware interrupts  
        ///
        /// Clears pending interrupt flags for the specified interrupt sources.
        /// This is typically called in interrupt service routines to acknowledge
        /// that the interrupt has been handled.
        fn clear_slave_interrupts(&mut self, mask: u32);

        /// Current slave hardware status
        ///
        /// Returns comprehensive status information about the slave controller
        /// including enabled state, address, buffer counts, and error conditions.
        fn slave_status(&self) -> Result<SlaveStatus, Self::Error>;

        /// Last slave event that occurred
        ///
        /// Returns the most recent slave event, useful for debugging
        /// and state tracking. May return None if no events have occurred
        /// since reset or if the hardware doesn't track this information.
        fn last_slave_event(&self) -> Option<I2cSEvent>;
    }

    /// Blocking slave event handling (sync pattern)
    ///
    /// This trait provides blocking operations suitable for synchronous code
    /// that can afford to wait for events. Operations may block the calling
    /// thread until the requested condition is met or timeout occurs.
    pub trait I2cSlaveEventSync<A: AddressMode = SevenBitAddress>: I2cSlaveInterrupts<A> {
        /// Wait for a specific slave event with timeout
        ///
        /// Blocks until the specified event occurs or the timeout expires.
        /// Returns true if the event occurred, false if timeout expired.
        /// Useful for synchronous slave operations that need to coordinate
        /// with master transactions.
        fn wait_for_slave_event(
            &mut self,
            expected_event: I2cSEvent,
            timeout_ms: u32,
        ) -> Result<bool, Self::Error>;

        /// Wait for any slave event with timeout
        ///
        /// Blocks until any slave event occurs or timeout expires.
        /// Returns the event that occurred, or None if timeout expired.
        /// Useful when any event needs to be processed synchronously.
        fn wait_for_any_event(&mut self, timeout_ms: u32)
            -> Result<Option<I2cSEvent>, Self::Error>;

        /// Handle a specific slave event with blocking semantics
        ///
        /// Processes a slave event and may block if the event handling
        /// requires waiting for hardware completion. This is different
        /// from the polling version which always returns immediately.
        fn handle_slave_event_blocking(&mut self, event: I2cSEvent) -> Result<(), Self::Error>;
    }

    /// Complete slave implementation combining core functionality
    ///
    /// This trait represents a basic slave implementation that combines
    /// core setup and buffer operations. It's suitable for most simple
    /// slave use cases without requiring event handling.
    pub trait I2cSlaveBasic<A: AddressMode = SevenBitAddress>:
        I2cSlaveCore<A> + I2cSlaveBuffer<A>
    {
    }

    /// Blanket implementation: any type implementing core + buffer gets basic slave
    impl<T, A: AddressMode> I2cSlaveBasic<A> for T where T: I2cSlaveCore<A> + I2cSlaveBuffer<A> {}

    /// Complete sync slave implementation
    ///
    /// This trait represents a full sync slave implementation that supports
    /// all blocking slave operations. Perfect for traditional blocking
    /// implementations that can afford to wait.
    pub trait I2cSlaveSync<A: AddressMode = SevenBitAddress>:
        I2cSlaveCore<A> + I2cSlaveBuffer<A> + I2cSlaveEventSync<A>
    {
    }

    /// Blanket implementation: any type implementing core + buffer + sync events gets sync slave
    impl<T, A: AddressMode> I2cSlaveSync<A> for T where
        T: I2cSlaveCore<A> + I2cSlaveBuffer<A> + I2cSlaveEventSync<A>
    {
    }

    /// Combined trait for controllers supporting both master and slave modes
    ///
    /// This is a convenience trait for hardware that supports both modes.
    /// Implementations get this automatically via blanket implementation.
    pub trait I2cMasterSlave<A: AddressMode = SevenBitAddress>:
        super::I2cMaster<A> + I2cSlaveSync<A>
    {
    }

    /// Blanket implementation: any type implementing both master and sync slave gets this trait
    impl<T, A: AddressMode> I2cMasterSlave<A> for T where T: super::I2cMaster<A> + I2cSlaveSync<A> {}
}

/// Re-export slave traits for convenience
pub use slave::{
    I2cMasterSlave, I2cSlaveBasic, I2cSlaveBuffer, I2cSlaveCore, I2cSlaveEventSync,
    I2cSlaveInterrupts, I2cSlaveSync,
};
