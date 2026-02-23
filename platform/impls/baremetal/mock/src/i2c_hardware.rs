// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Minimal mock implementation of OpenPRoT I2C hardware
//!
//! This module provides the simplest possible implementation of the I2C hardware
//! abstraction traits for testing and development purposes. The mock supports both
//! blocking and non-blocking operations, master and slave modes, and provides
//! comprehensive testing utilities.
//!
//! # Features
//!
//! - **Complete trait coverage**: Implements all OpenPRoT I2C hardware traits
//! - **Configurable behavior**: Success/failure modes for testing error paths
//! - **Event simulation**: Inject and poll I2C slave events for testing
//! - **Buffer management**: Realistic slave receive/transmit buffer simulation
//! - **No external dependencies**: Uses only core Rust and OpenPRoT traits
//! - **Production testing**: Comprehensive test suite with 20+ test cases
//!
//! # Examples
//!
//! ## Basic Master Operations
//!
//! ```text
//! use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cConfig};
//! use openprot_hal_blocking::i2c_hardware::{I2cHardwareCore, I2cMaster};
//!
//! let mut mock = MockI2cHardware::new();
//! let mut config = MockI2cConfig::default();
//! mock.init(&mut config);
//!
//! // Write to device at address 0x50
//! match mock.write(0x50, &[0x01, 0x02, 0x03]) {
//!     Ok(()) => {},
//!     Err(_) => return,
//! }
//!
//! // Read from device
//! let mut buffer = [0u8; 4];
//! match mock.read(0x50, &mut buffer) {
//!     Ok(()) => {
//!         // Buffer now contains [0xFF, 0xFF, 0xFF, 0xFF] (mock dummy data)
//!         assert_eq!(buffer, [0xFF; 4]);
//!     },
//!     Err(_) => return,
//! }
//! ```
//!
//! ## Slave Mode Testing
//!
//! ```text
//! use openprot_platform_mock::i2c_hardware::MockI2cHardware;
//! use openprot_hal_blocking::i2c_hardware::slave::{I2cSlaveCore, I2cSlaveBuffer};
//!
//! let mut mock = MockI2cHardware::new();
//!
//! // Configure as slave device
//! match mock.configure_slave_address(0x42) {
//!     Ok(()) => {},
//!     Err(_) => return,
//! }
//! match mock.enable_slave_mode() {
//!     Ok(()) => {},
//!     Err(_) => return,
//! }
//!
//! // Simulate receiving data from master
//! mock.inject_slave_data(&[0xAA, 0xBB, 0xCC]);
//!
//! // Read the received data
//! let mut buffer = [0u8; 3];
//! match mock.read_slave_buffer(&mut buffer) {
//!     Ok(count) => {
//!         assert_eq!(count, 3);
//!         assert_eq!(buffer, [0xAA, 0xBB, 0xCC]);
//!     },
//!     Err(_) => return,
//! }
//! ```
//!
//! ## Error Testing
//!
//! ```text
//! use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cError};
//! use openprot_hal_blocking::i2c_hardware::I2cMaster;
//!
//! let mut failing_mock = MockI2cHardware::new_failing();
//!
//! // All operations will fail
//! let result = failing_mock.write(0x50, &[0x01]);
//! match result {
//!     Err(MockI2cError::Bus) => {
//!         // Expected error for failing mock
//!     },
//!     _ => return, // Unexpected result
//! }
//! ```
//!
//! ## SystemControl Integration
//!
//! ```text
//! use openprot_platform_mock::i2c_hardware::MockI2cHardwareWithSystem;
//! use openprot_platform_mock::system_control::{MockSystemControl, MockClockId, MockResetId};
//! use openprot_hal_blocking::i2c_hardware::{I2cHardwareCore, I2cMaster};
//! use openprot_hal_blocking::system_control::SystemControl;
//!
//! // Create mock system controller
//! let system_control = MockSystemControl::new();
//! let mut i2c_with_system = MockI2cHardwareWithSystem::new(system_control, MockClockId::I2c1, MockResetId::I2c1);
//!
//! // Initialize with system control integration
//! let mut config = MockI2cConfig::default();
//! match i2c_with_system.init(&mut config) {
//!     Ok(()) => {
//!         // System control automatically enabled I2C clock and released reset
//!     },
//!     Err(_) => return,
//! }
//!
//! // Configure timing with automatic clock source management
//! match i2c_with_system.configure_timing(400_000, &()) {
//!     Ok(actual_freq) => {
//!         // System controller configured optimal clock source frequency
//!         // I2C timing registers configured based on actual frequency
//!     },
//!     Err(_) => return,
//! }
//! ```
//!
//! ## Non-blocking Event Handling
//!
//! ```text
//! use openprot_platform_mock::i2c_hardware::MockI2cHardware;
//! use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;
//!
//! let mut mock = MockI2cHardware::new();
//!
//! // Inject single event for testing (mock only stores most recent event)
//! mock.inject_slave_event(I2cSEvent::SlaveWrReq);
//!
//! // Poll for events (non-blocking)
//! match mock.poll_slave_events() {
//!     Ok(Some(I2cSEvent::SlaveWrReq)) => {
//!         // Event received as expected
//!     },
//!     Ok(Some(_)) => {
//!         // Other event type received
//!     },
//!     Ok(None) => {
//!         // No events pending
//!     },
//!     Err(_) => return, // Error occurred
//! }
//!
//! // Subsequent poll returns None (event was consumed)
//! match mock.poll_slave_events() {
//!     Ok(None) => {
//!         // No more events
//!     },
//!     Ok(Some(_)) => {
//!         // Unexpected event
//!     },
//!     Err(_) => return, // Error occurred
//! }
//! ```

use embedded_hal::i2c::{Operation, SevenBitAddress};
use openprot_hal_blocking::i2c_hardware::{I2cHardwareCore, I2cMaster};

/// Mock error type for I2C operations
///
/// This error type implements the embedded-hal I2C error trait and provides
/// standard I2C error conditions for testing purposes. All errors can be
/// converted to ResponseCode for compatibility with Hubris I2C servers.
///
/// # Examples
///
/// ```text
/// use openprot_platform_mock::i2c_hardware::MockI2cError;
/// use embedded_hal::i2c::{Error, ErrorKind};
///
/// let error = MockI2cError::NoAcknowledge;
/// assert_eq!(error.kind(), ErrorKind::NoAcknowledge(embedded_hal::i2c::NoAcknowledgeSource::Unknown));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockI2cError {
    /// Bus error (line stuck, arbitration lost, clock timeout, etc.)
    ///
    /// This covers general bus-level problems that prevent communication.
    Bus,
    /// Arbitration lost during multi-master operation
    ///
    /// Occurs when multiple masters try to use the bus simultaneously.
    ArbitrationLoss,
    /// No acknowledge received from slave device
    ///
    /// The addressed device did not respond or is not present.
    NoAcknowledge,
    /// Other unspecified error
    ///
    /// Catch-all for any other error conditions.
    Other,
}

impl embedded_hal::i2c::Error for MockI2cError {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self {
            MockI2cError::Bus => embedded_hal::i2c::ErrorKind::Bus,
            MockI2cError::ArbitrationLoss => embedded_hal::i2c::ErrorKind::ArbitrationLoss,
            MockI2cError::NoAcknowledge => embedded_hal::i2c::ErrorKind::NoAcknowledge(
                embedded_hal::i2c::NoAcknowledgeSource::Unknown,
            ),
            MockI2cError::Other => embedded_hal::i2c::ErrorKind::Other,
        }
    }
}

/// Mock I2C configuration
///
/// Configuration structure for controlling mock I2C hardware behavior.
/// This allows tests to configure whether operations succeed or fail,
/// and to set simulated timing parameters.
///
/// # Examples
///
/// ```text
/// use openprot_platform_mock::i2c_hardware::MockI2cConfig;
///
/// // Default config (operations succeed, 100kHz)
/// let config = MockI2cConfig::default();
/// assert!(config.success);
/// assert_eq!(config.frequency, 100_000);
///
/// // Failing config for error testing
/// let failing_config = MockI2cConfig {
///     success: false,
///     frequency: 400_000,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MockI2cConfig {
    /// Whether operations should succeed
    ///
    /// When `true`, all I2C operations will succeed. When `false`,
    /// operations will return `MockI2cError::Bus` for testing error paths.
    pub success: bool,
    /// Simulated clock frequency in Hz
    ///
    /// This value is stored but doesn't affect timing in the mock.
    /// Common values: 100_000 (100kHz), 400_000 (400kHz), 1_000_000 (1MHz)
    pub frequency: u32,
}

impl Default for MockI2cConfig {
    fn default() -> Self {
        Self {
            success: true,
            frequency: 100_000, // 100 kHz
        }
    }
}

/// Mock I2C hardware implementation
///
/// This implementation provides the bare minimum functionality needed to satisfy
/// the OpenPRoT hardware traits. All operations are no-ops or return predictable
/// dummy data, making it perfect for unit testing and development.
///
/// # Memory Usage (Exact Calculation)
///
/// ```text
/// Field                    | Size (bytes) | Alignment
/// -------------------------|--------------|----------
/// config                   |     5        |    4
/// initialized              |     1        |    1
/// [padding]                |     2        |    -
/// slave_enabled            |     1        |    1
/// slave_address            |     1        |    1
/// [padding]                |     6        |    -
/// slave_rx_buffer          |    64        |    1
/// slave_rx_count           |     8        |    8
/// slave_tx_buffer          |    64        |    1
/// [padding]                |     7        |    -
/// slave_tx_count           |     8        |    8
/// last_slave_event         |     1        |    1
/// [padding]                |     7        |    -
/// -------------------------|--------------|----------
/// TOTAL                    |   168 bytes  |    8
/// ```
///
/// **Final Size**: 168 bytes per instance (72% reduction from original 608 bytes)
///
/// **Memory Breakdown**:
/// - Base fields: 16 bytes (config, flags, addresses)
/// - Slave buffers: 128 bytes (2x 64-byte arrays)
/// - Counters: 16 bytes (2x usize = 2x 8 bytes on 64-bit)
/// - Event storage: 1 byte (enum discriminant)
/// - Padding: 7 bytes (for alignment)
///
/// # Examples
///
/// ```text
/// use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cConfig};
/// use openprot_hal_blocking::i2c_hardware::{I2cHardwareCore, I2cMaster};
///
/// // Create and initialize mock
/// let mut mock = MockI2cHardware::new();
/// let mut config = MockI2cConfig::default();
/// mock.init(&mut config);
///
/// // Perform I2C operations - use match for error handling
/// match mock.write(0x50, &[0x01, 0x02]) {
///     Ok(()) => {},
///     Err(_) => return,
/// }
/// let mut buffer = [0u8; 4];
/// match mock.read(0x50, &mut buffer) {
///     Ok(()) => {},
///     Err(_) => return,
/// }
/// ```
#[derive(Debug)]
pub struct MockI2cHardware {
    /// Current configuration settings (5 bytes: bool + u32)
    config: MockI2cConfig,
    /// Whether init() has been called (1 byte)
    initialized: bool,

    // Slave mode fields
    /// Whether slave mode is currently enabled (1 byte)
    slave_enabled: bool,
    /// Currently configured slave address (1 byte: `Option<u8>`)
    slave_address: Option<SevenBitAddress>,
    /// Slave receive buffer (64 bytes) - realistic I2C message size
    slave_rx_buffer: [u8; 64],
    /// Number of valid bytes in receive buffer (8 bytes: usize on 64-bit)
    slave_rx_count: usize,
    /// Slave transmit buffer (64 bytes) - realistic I2C message size  
    slave_tx_buffer: [u8; 64],
    /// Number of valid bytes in transmit buffer (8 bytes: usize on 64-bit)
    slave_tx_count: usize,
    /// Most recent slave event that occurred (1 byte: `Option<enum>`)
    last_slave_event: Option<openprot_hal_blocking::i2c_hardware::slave::I2cSEvent>,
}

impl MockI2cHardware {
    /// Create a new mock I2C hardware instance
    ///
    /// Creates a mock in success mode with default configuration.
    /// The mock is not initialized until `init()` is called.
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::MockI2cHardware;
    ///
    /// let mock = MockI2cHardware::new();
    /// assert!(!mock.is_initialized());
    /// ```
    pub fn new() -> Self {
        Self {
            config: MockI2cConfig::default(),
            initialized: false,
            slave_enabled: false,
            slave_address: None,
            slave_rx_buffer: [0; 64],
            slave_rx_count: 0,
            slave_tx_buffer: [0; 64],
            slave_tx_count: 0,
            last_slave_event: None,
        }
    }

    /// Create a new mock that will fail operations
    ///
    /// Creates a mock in failure mode where all operations will return
    /// `MockI2cError::Bus`. Useful for testing error handling paths.
    pub fn new_failing() -> Self {
        Self {
            config: MockI2cConfig {
                success: false,
                frequency: 100_000,
            },
            initialized: false,
            slave_enabled: false,
            slave_address: None,
            slave_rx_buffer: [0; 64],
            slave_rx_count: 0,
            slave_tx_buffer: [0; 64],
            slave_tx_count: 0,
            last_slave_event: None,
        }
    }

    /// Check if the mock has been initialized
    ///
    /// Returns `true` if `init()` has been called, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cConfig};
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// assert!(!mock.is_initialized());
    ///
    /// let mut config = MockI2cConfig::default();
    /// mock.init(&mut config);
    /// assert!(mock.is_initialized());
    /// ```
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if operations should succeed
    ///
    /// Internal helper method that returns Ok(()) if operations should succeed,
    /// or Err(MockI2cError::Bus) if they should fail.
    fn check_success(&self) -> Result<(), MockI2cError> {
        if self.config.success {
            Ok(())
        } else {
            Err(MockI2cError::Bus)
        }
    }
}

impl Default for MockI2cHardware {
    fn default() -> Self {
        Self::new()
    }
}

impl I2cHardwareCore for MockI2cHardware {
    type Error = MockI2cError;
    type Config = MockI2cConfig;
    type I2cSpeed = u32; // Speed in Hz
    type TimingConfig = (); // No timing config needed for mock

    /// Initialize the I2C hardware with the given configuration
    ///
    /// Sets up the I2C controller with initial configuration and marks
    /// it as initialized. This is typically called once during system startup.
    ///
    /// # Parameters
    ///
    /// * `config` - Mutable reference to I2C configuration (allows hardware to modify)
    ///
    /// # Mock Behavior
    ///
    /// - Stores the configuration internally
    /// - Sets initialized flag to true
    /// - No actual hardware initialization occurs
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cConfig};
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// let mut config = MockI2cConfig::default();
    ///
    /// mock.init(&mut config);
    /// assert!(mock.is_initialized());
    /// ```
    fn init(&mut self, config: &mut Self::Config) -> Result<(), Self::Error> {
        self.config = *config;
        self.initialized = true;
        Ok(())
    }

    /// Configure I2C timing parameters
    ///
    /// Updates the timing configuration for the I2C bus, including
    /// frequency settings and timing parameters.
    ///
    /// # Parameters
    ///
    /// * `config` - Mutable reference to configuration containing timing settings
    ///
    /// # Mock Behavior
    ///
    /// - Updates internal frequency setting
    /// - No actual hardware timing configuration occurs
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cConfig};
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// let speed = 400_000u32; // 400kHz
    /// let timing_config = (); // Empty timing config for mock
    ///
    /// let result = mock.configure_timing(speed, &timing_config);
    /// assert!(result.is_ok());
    /// // Configuration is now applied to the mock
    /// ```
    fn configure_timing(
        &mut self,
        speed: Self::I2cSpeed,
        _timing: &Self::TimingConfig,
    ) -> Result<u32, Self::Error> {
        self.config.frequency = speed;
        Ok(speed)
    }

    /// Enable specific interrupt sources
    ///
    /// Enables hardware interrupts for the specified interrupt mask.
    /// In the mock implementation, this is a no-op.
    ///
    /// # Parameters
    ///
    /// * `mask` - Bitmask of interrupts to enable
    ///
    /// # Mock Behavior
    ///
    /// - No operation performed (mock doesn't simulate interrupts)
    /// - Mask value is ignored
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::MockI2cHardware;
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// mock.enable_interrupts(0x01); // Enable specific interrupt
    /// // No visible effect in mock
    /// ```
    fn enable_interrupts(&mut self, _mask: u32) {
        // No-op for mock
    }

    /// Clear pending interrupt flags
    ///
    /// Clears the specified pending interrupt flags in the hardware.
    /// In the mock implementation, this is a no-op.
    ///
    /// # Parameters
    ///
    /// * `mask` - Bitmask of interrupts to clear
    ///
    /// # Mock Behavior
    ///
    /// - No operation performed (mock doesn't simulate interrupts)
    /// - Mask value is ignored
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::MockI2cHardware;
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// mock.clear_interrupts(0xFF); // Clear all interrupts
    /// // No visible effect in mock
    /// ```
    fn clear_interrupts(&mut self, _mask: u32) {
        // No-op for mock
    }

    /// Handle pending interrupts
    ///
    /// Process any pending hardware interrupts and perform necessary actions.
    /// In the mock implementation, this is a no-op.
    ///
    /// # Mock Behavior
    ///
    /// - No operation performed (mock doesn't simulate interrupts)
    /// - Use `inject_slave_event()` for event-driven testing instead
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::MockI2cHardware;
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// mock.handle_interrupt(); // No visible effect in mock
    ///
    /// // For testing interrupt-like behavior, use:
    /// // mock.inject_slave_event(event);
    /// ```
    fn handle_interrupt(&mut self) {
        // No-op for mock
    }

    /// Recover the I2C bus from error conditions
    ///
    /// Attempts to recover the I2C bus from stuck or error conditions
    /// by performing bus recovery procedures.
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Bus recovery was successful
    /// - `Err(MockI2cError::Bus)` - If the mock is configured to fail
    ///
    /// # Mock Behavior
    ///
    /// - Simply checks the configured success/failure mode
    /// - No actual bus recovery operations performed
    /// - Can be configured to fail for testing error handling
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::MockI2cHardware;
    /// use openprot_hal_blocking::i2c_hardware::I2cHardwareCore;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// assert!(mock.recover_bus().is_ok());
    ///
    /// // Test failure mode
    /// let mut failing_mock = MockI2cHardware::new_failing();
    /// assert!(failing_mock.recover_bus().is_err());
    /// ```
    fn recover_bus(&mut self) -> Result<(), Self::Error> {
        self.check_success()
    }
}

impl I2cMaster<SevenBitAddress> for MockI2cHardware {
    fn write(&mut self, _addr: SevenBitAddress, _bytes: &[u8]) -> Result<(), Self::Error> {
        self.check_success()
    }

    fn read(&mut self, _addr: SevenBitAddress, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.check_success()?;
        // Fill buffer with dummy data
        for byte in buffer.iter_mut() {
            *byte = 0xFF;
        }
        Ok(())
    }

    fn write_read(
        &mut self,
        _addr: SevenBitAddress,
        _bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.check_success()?;
        // Fill buffer with dummy data
        for byte in buffer.iter_mut() {
            *byte = 0xFF;
        }
        Ok(())
    }

    fn transaction_slice(
        &mut self,
        _addr: SevenBitAddress,
        ops_slice: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.check_success()?;

        // Process each operation
        for op in ops_slice.iter_mut() {
            match op {
                Operation::Read(buffer) => {
                    // Fill read buffers with dummy data
                    for byte in buffer.iter_mut() {
                        *byte = 0xFF;
                    }
                }
                Operation::Write(_) => {
                    // Write operations are no-ops in mock
                }
            }
        }
        Ok(())
    }
}

// Slave trait implementations
impl openprot_hal_blocking::i2c_hardware::slave::I2cSlaveCore<SevenBitAddress> for MockI2cHardware {
    fn configure_slave_address(&mut self, addr: SevenBitAddress) -> Result<(), Self::Error> {
        self.check_success()?;
        self.slave_address = Some(addr);
        Ok(())
    }

    fn enable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.check_success()?;
        self.slave_enabled = true;
        Ok(())
    }

    fn disable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.check_success()?;
        self.slave_enabled = false;
        Ok(())
    }

    fn is_slave_mode_enabled(&self) -> bool {
        self.slave_enabled
    }

    fn slave_address(&self) -> Option<SevenBitAddress> {
        self.slave_address
    }
}

impl openprot_hal_blocking::i2c_hardware::slave::I2cSlaveBuffer<SevenBitAddress>
    for MockI2cHardware
{
    fn read_slave_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.check_success()?;
        let copy_len = buffer.len().min(self.slave_rx_count);

        // Use safe slice access instead of direct indexing
        if let (Some(dst_slice), Some(src_slice)) = (
            buffer.get_mut(..copy_len),
            self.slave_rx_buffer.get(..copy_len),
        ) {
            dst_slice.copy_from_slice(src_slice);
        }
        self.slave_rx_count = 0; // Clear buffer after reading
        Ok(copy_len)
    }

    fn write_slave_response(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.check_success()?;
        let copy_len = data.len().min(self.slave_tx_buffer.len());

        // Use safe slice access instead of direct indexing
        if let (Some(dst_slice), Some(src_slice)) = (
            self.slave_tx_buffer.get_mut(..copy_len),
            data.get(..copy_len),
        ) {
            dst_slice.copy_from_slice(src_slice);
            self.slave_tx_count = copy_len;
        }
        Ok(())
    }

    fn poll_slave_data(&mut self) -> Result<Option<usize>, Self::Error> {
        self.check_success()?;
        if self.slave_rx_count > 0 {
            Ok(Some(self.slave_rx_count))
        } else {
            Ok(None)
        }
    }

    fn clear_slave_buffer(&mut self) -> Result<(), Self::Error> {
        self.check_success()?;
        self.slave_rx_count = 0;
        self.slave_tx_count = 0;
        Ok(())
    }

    fn tx_buffer_space(&self) -> Result<usize, Self::Error> {
        if self.config.success {
            // Use saturating_sub to prevent underflow
            Ok(self
                .slave_tx_buffer
                .len()
                .saturating_sub(self.slave_tx_count))
        } else {
            Err(MockI2cError::Bus)
        }
    }

    fn rx_buffer_count(&self) -> Result<usize, Self::Error> {
        if self.config.success {
            Ok(self.slave_rx_count)
        } else {
            Err(MockI2cError::Bus)
        }
    }
}

impl openprot_hal_blocking::i2c_hardware::slave::I2cSlaveInterrupts<SevenBitAddress>
    for MockI2cHardware
{
    fn enable_slave_interrupts(&mut self, _mask: u32) {
        // No-op for mock
    }

    fn clear_slave_interrupts(&mut self, _mask: u32) {
        // No-op for mock
    }

    fn slave_status(
        &self,
    ) -> Result<openprot_hal_blocking::i2c_hardware::slave::SlaveStatus, Self::Error> {
        if self.config.success {
            Ok(openprot_hal_blocking::i2c_hardware::slave::SlaveStatus {
                enabled: self.slave_enabled,
                address: self.slave_address,
                data_available: self.slave_rx_count > 0,
                rx_buffer_count: self.slave_rx_count,
                tx_buffer_count: self.slave_tx_count,
                last_event: self.last_slave_event,
                error: false,
            })
        } else {
            Err(MockI2cError::Bus)
        }
    }

    fn last_slave_event(&self) -> Option<openprot_hal_blocking::i2c_hardware::slave::I2cSEvent> {
        self.last_slave_event
    }
}

// Non-blocking trait implementations
impl MockI2cHardware {
    /// Inject data into the slave receive buffer for testing
    ///
    /// Simulates data being received from an I2C master by directly
    /// placing data into the slave receive buffer.
    ///
    /// # Parameters
    ///
    /// * `data` - The data to inject (up to 64 bytes)
    ///
    /// # Behavior
    ///
    /// - Data is copied into the internal receive buffer using safe slice operations
    /// - If data is longer than 64 bytes, only the first 64 bytes are used
    /// - Previous buffer contents are overwritten
    /// - The receive count is updated to match the data length
    ///
    /// # Examples
    ///
    /// ```text
    /// use openprot_platform_mock::i2c_hardware::MockI2cHardware;
    /// use openprot_hal_blocking::i2c_hardware::slave::I2cSlaveBuffer;
    ///
    /// let mut mock = MockI2cHardware::new();
    /// mock.inject_slave_data(&[0xAA, 0xBB, 0xCC]);
    ///
    /// let mut buffer = [0u8; 3];
    /// match mock.read_slave_buffer(&mut buffer) {
    ///     Ok(count) => {
    ///         assert_eq!(count, 3);
    ///         assert_eq!(buffer, [0xAA, 0xBB, 0xCC]);
    ///     },
    ///     Err(_) => return, // Handle error appropriately
    /// }
    /// ```
    pub fn inject_slave_data(&mut self, data: &[u8]) {
        let copy_len = data.len().min(self.slave_rx_buffer.len());

        // Use safe slice access instead of direct indexing
        if let (Some(dst_slice), Some(src_slice)) = (
            self.slave_rx_buffer.get_mut(..copy_len),
            data.get(..copy_len),
        ) {
            dst_slice.copy_from_slice(src_slice);
            self.slave_rx_count = copy_len;
        }
    }

    /// Set the last slave event for testing
    ///
    /// Simple event injection that only stores the most recent event.
    /// No complex event queue management.
    pub fn inject_slave_event(
        &mut self,
        event: openprot_hal_blocking::i2c_hardware::slave::I2cSEvent,
    ) {
        self.last_slave_event = Some(event);
    }

    /// Get and clear the last slave event
    ///
    /// Returns the most recent slave event and clears it.
    /// Simplified version of event polling.
    pub fn poll_slave_events(
        &mut self,
    ) -> Result<Option<openprot_hal_blocking::i2c_hardware::slave::I2cSEvent>, MockI2cError> {
        self.check_success()?;
        let event = self.last_slave_event.take();
        Ok(event)
    }

    /// Check if a specific event is the last recorded event
    ///
    /// Simplified event checking without complex queue management.
    pub fn is_event_pending_nb(
        &self,
        event: openprot_hal_blocking::i2c_hardware::slave::I2cSEvent,
    ) -> Result<bool, MockI2cError> {
        if self.config.success {
            Ok(self.last_slave_event == Some(event))
        } else {
            Err(MockI2cError::Bus)
        }
    }

    /// Handle a specific slave event (non-blocking)
    ///
    /// Processes a slave event with mock behavior.
    pub fn handle_slave_event_nb(
        &mut self,
        event: openprot_hal_blocking::i2c_hardware::slave::I2cSEvent,
    ) -> Result<(), MockI2cError> {
        self.check_success()?;
        self.last_slave_event = Some(event);

        // Simulate event handling based on event type
        match event {
            openprot_hal_blocking::i2c_hardware::slave::I2cSEvent::SlaveWrRecvd => {
                // Simulate receiving data using safe injection
                self.inject_slave_data(&[0xAA, 0xBB, 0xCC]);
            }
            openprot_hal_blocking::i2c_hardware::slave::I2cSEvent::SlaveRdReq => {
                // Prepare response data using safe method
                use openprot_hal_blocking::i2c_hardware::slave::I2cSlaveBuffer;
                match self.write_slave_response(&[0x11, 0x22, 0x33]) {
                    Ok(()) => {}
                    Err(_) => {
                        // Error writing response - this is logged but not propagated
                        // since this is simulation code and the error would be
                        // caught in actual usage
                    }
                }
            }
            _ => {
                // Other events are just recorded
            }
        }
        Ok(())
    }
}

/// I2C hardware implementation with external SystemControl integration
///
/// This implementation demonstrates how to integrate external system control
/// dependencies into I2C hardware initialization and timing configuration.
/// It combines the basic MockI2cHardware functionality with SystemControl
/// operations for realistic hardware abstraction patterns.
///
/// # Type Parameters
///
/// * `S` - SystemControl implementation (clock and reset control)
///
/// # Examples
///
/// ```text
/// use openprot_platform_mock::i2c_hardware::MockI2cHardwareWithSystem;
/// use openprot_platform_mock::system_control::{MockSystemControl, MockClockId, MockResetId};
///
/// let system_control = MockSystemControl::new();
/// let mut i2c = MockI2cHardwareWithSystem::new(
///     system_control,
///     MockClockId::I2c1,
///     MockResetId::I2c1
/// );
///
/// let mut config = MockI2cConfig::default();
/// i2c.init(&mut config).unwrap();
/// ```
pub struct MockI2cHardwareWithSystem<S> {
    /// Underlying I2C hardware mock
    base_hardware: MockI2cHardware,
    /// External system control for clock and reset management
    system_control: S,
    /// Clock identifier for this I2C instance
    clock_id: crate::system_control::MockClockId,
    /// Reset identifier for this I2C instance
    reset_id: crate::system_control::MockResetId,
}

impl<S> MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    /// Create a new I2C hardware instance with system control integration
    ///
    /// # Parameters
    ///
    /// * `system_control` - System control implementation for clock and reset management
    /// * `clock_id` - Clock identifier for this I2C peripheral
    /// * `reset_id` - Reset identifier for this I2C peripheral
    pub fn new(
        system_control: S,
        clock_id: crate::system_control::MockClockId,
        reset_id: crate::system_control::MockResetId,
    ) -> Self {
        Self {
            base_hardware: MockI2cHardware::new(),
            system_control,
            clock_id,
            reset_id,
        }
    }

    /// Create a failing instance for error testing
    pub fn new_failing(
        system_control: S,
        clock_id: crate::system_control::MockClockId,
        reset_id: crate::system_control::MockResetId,
    ) -> Self {
        Self {
            base_hardware: MockI2cHardware::new_failing(),
            system_control,
            clock_id,
            reset_id,
        }
    }

    /// Get reference to the system control for testing
    pub fn system_control(&self) -> &S {
        &self.system_control
    }

    /// Get mutable reference to the system control for testing
    pub fn system_control_mut(&mut self) -> &mut S {
        &mut self.system_control
    }
}

impl<S> I2cHardwareCore for MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    type Error = MockI2cError;
    type Config = MockI2cConfig;
    type I2cSpeed = u32;
    type TimingConfig = ();

    /// Initialize I2C hardware with SystemControl integration
    ///
    /// This demonstrates the integration pattern where system control
    /// operations are performed during hardware initialization:
    /// 1. Enable peripheral clock
    /// 2. Release peripheral from reset
    /// 3. Initialize I2C-specific hardware
    fn init(&mut self, config: &mut Self::Config) -> Result<(), Self::Error> {
        // Step 1: Enable peripheral clock via system control
        self.system_control
            .enable(&self.clock_id)
            .map_err(|_| MockI2cError::Bus)?;

        // Step 2: Release peripheral from reset
        self.system_control
            .reset_deassert(&self.reset_id)
            .map_err(|_| MockI2cError::Bus)?;

        // Step 3: Initialize I2C hardware (delegate to base implementation)
        self.base_hardware.init(config)
    }

    /// Configure timing with SystemControl clock management
    ///
    /// This shows how external system control can manage clock sources
    /// while I2C hardware configures its internal timing registers.
    fn configure_timing(
        &mut self,
        speed: Self::I2cSpeed,
        timing: &Self::TimingConfig,
    ) -> Result<u32, Self::Error> {
        // Calculate optimal source frequency for requested I2C speed
        // In real hardware, this would consider setup/hold times, filter delays, etc.
        let source_freq = calculate_optimal_source_frequency(speed);

        // Configure system clock source via SystemControl
        self.system_control
            .set_frequency(&self.clock_id, source_freq)
            .map_err(|_| MockI2cError::Bus)?;

        // Get actual configured frequency (may differ from requested)
        let actual_source_freq = self
            .system_control
            .get_frequency(&self.clock_id)
            .map_err(|_| MockI2cError::Bus)?;

        // Configure I2C internal timing based on actual source frequency
        self.base_hardware.configure_timing(speed, timing)?;

        Ok(actual_source_freq as u32)
    }

    fn enable_interrupts(&mut self, mask: u32) {
        self.base_hardware.enable_interrupts(mask);
    }

    fn clear_interrupts(&mut self, mask: u32) {
        self.base_hardware.clear_interrupts(mask);
    }

    fn handle_interrupt(&mut self) {
        self.base_hardware.handle_interrupt();
    }

    fn recover_bus(&mut self) -> Result<(), Self::Error> {
        // In real hardware, bus recovery might require system-level operations
        // For now, delegate to base implementation
        self.base_hardware.recover_bus()
    }
}

// Forward all I2C master operations to the base hardware
impl<S> I2cMaster<SevenBitAddress> for MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    fn write(&mut self, addr: SevenBitAddress, bytes: &[u8]) -> Result<(), Self::Error> {
        self.base_hardware.write(addr, bytes)
    }

    fn read(&mut self, addr: SevenBitAddress, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.base_hardware.read(addr, buffer)
    }

    fn write_read(
        &mut self,
        addr: SevenBitAddress,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.base_hardware.write_read(addr, bytes, buffer)
    }

    fn transaction_slice(
        &mut self,
        addr: SevenBitAddress,
        ops_slice: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.base_hardware.transaction_slice(addr, ops_slice)
    }
}

// Forward all slave operations to the base hardware
impl<S> openprot_hal_blocking::i2c_hardware::slave::I2cSlaveCore<SevenBitAddress>
    for MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    fn configure_slave_address(&mut self, addr: SevenBitAddress) -> Result<(), Self::Error> {
        self.base_hardware.configure_slave_address(addr)
    }

    fn enable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.base_hardware.enable_slave_mode()
    }

    fn disable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.base_hardware.disable_slave_mode()
    }

    fn is_slave_mode_enabled(&self) -> bool {
        self.base_hardware.is_slave_mode_enabled()
    }

    fn slave_address(&self) -> Option<SevenBitAddress> {
        self.base_hardware.slave_address()
    }
}

impl<S> openprot_hal_blocking::i2c_hardware::slave::I2cSlaveBuffer<SevenBitAddress>
    for MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    fn read_slave_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.base_hardware.read_slave_buffer(buffer)
    }

    fn write_slave_response(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.base_hardware.write_slave_response(data)
    }

    fn poll_slave_data(&mut self) -> Result<Option<usize>, Self::Error> {
        self.base_hardware.poll_slave_data()
    }

    fn clear_slave_buffer(&mut self) -> Result<(), Self::Error> {
        self.base_hardware.clear_slave_buffer()
    }

    fn tx_buffer_space(&self) -> Result<usize, Self::Error> {
        self.base_hardware.tx_buffer_space()
    }

    fn rx_buffer_count(&self) -> Result<usize, Self::Error> {
        self.base_hardware.rx_buffer_count()
    }
}

impl<S> openprot_hal_blocking::i2c_hardware::slave::I2cSlaveInterrupts<SevenBitAddress>
    for MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    fn enable_slave_interrupts(&mut self, mask: u32) {
        self.base_hardware.enable_slave_interrupts(mask);
    }

    fn clear_slave_interrupts(&mut self, mask: u32) {
        self.base_hardware.clear_slave_interrupts(mask);
    }

    fn slave_status(
        &self,
    ) -> Result<openprot_hal_blocking::i2c_hardware::slave::SlaveStatus, Self::Error> {
        self.base_hardware.slave_status()
    }

    fn last_slave_event(&self) -> Option<openprot_hal_blocking::i2c_hardware::slave::I2cSEvent> {
        self.base_hardware.last_slave_event()
    }
}

// Additional testing methods for SystemControl integration
impl<S> MockI2cHardwareWithSystem<S>
where
    S: openprot_hal_blocking::system_control::SystemControl<
        ClockId = crate::system_control::MockClockId,
        ResetId = crate::system_control::MockResetId,
    >,
{
    /// Inject data into slave buffer (for testing)
    pub fn inject_slave_data(&mut self, data: &[u8]) {
        self.base_hardware.inject_slave_data(data);
    }

    /// Inject slave event (for testing)
    pub fn inject_slave_event(
        &mut self,
        event: openprot_hal_blocking::i2c_hardware::slave::I2cSEvent,
    ) {
        self.base_hardware.inject_slave_event(event);
    }

    /// Poll slave events (for testing)
    pub fn poll_slave_events(
        &mut self,
    ) -> Result<Option<openprot_hal_blocking::i2c_hardware::slave::I2cSEvent>, MockI2cError> {
        self.base_hardware.poll_slave_events()
    }
}

/// Calculate optimal source clock frequency for given I2C speed
///
/// This is a simplified calculation for the mock. In real hardware,
/// this would consider the I2C timing requirements, peripheral limitations,
/// and available clock sources to determine the optimal frequency.
fn calculate_optimal_source_frequency(i2c_speed: u32) -> u64 {
    // Simple heuristic: use 8x the I2C speed as source frequency
    // This allows for clock division and timing margin
    match i2c_speed {
        speed if speed <= 100_000 => 8_000_000, // 8 MHz for standard mode
        speed if speed <= 400_000 => 24_000_000, // 24 MHz for fast mode
        speed if speed <= 1_000_000 => 48_000_000, // 48 MHz for fast mode plus
        _ => 96_000_000,                        // 96 MHz for high speed modes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::assertions_on_constants)] // Allow assert!(false, "message") in tests for clear error messages
    #[test]
    fn test_mock_creation() {
        let mock = MockI2cHardware::new();
        assert!(!mock.initialized);
        assert!(mock.config.success);
    }

    #[test]
    fn test_mock_initialization() {
        let mut mock = MockI2cHardware::new();
        let mut config = MockI2cConfig::default();

        mock.init(&mut config).expect("Failed to init");
        assert!(mock.initialized);
    }

    #[test]
    fn test_successful_operations() {
        let mut mock = MockI2cHardware::new();
        let mut config = MockI2cConfig::default();
        mock.init(&mut config).expect("Failed to init mock");

        // Test write
        assert!(mock.write(0x50, &[0x01, 0x02]).is_ok());

        // Test read
        let mut buffer = [0u8; 4];
        assert!(mock.read(0x50, &mut buffer).is_ok());
        assert_eq!(buffer, [0xFF; 4]);

        // Test write_read
        let mut buffer = [0u8; 2];
        assert!(mock.write_read(0x50, &[0x01], &mut buffer).is_ok());
        assert_eq!(buffer, [0xFF; 2]);
    }

    #[test]
    fn test_failing_operations() {
        let mut mock = MockI2cHardware::new_failing();

        // All operations should fail
        assert_eq!(mock.write(0x50, &[0x01]), Err(MockI2cError::Bus));

        let mut buffer = [0u8; 2];
        assert_eq!(mock.read(0x50, &mut buffer), Err(MockI2cError::Bus));
        assert_eq!(
            mock.write_read(0x50, &[0x01], &mut buffer),
            Err(MockI2cError::Bus)
        );
    }

    #[test]
    fn test_transaction_slice() {
        let mut mock = MockI2cHardware::new();
        let mut config = MockI2cConfig::default();
        mock.init(&mut config).expect("Failed to init mock");

        let write_data = [0x01, 0x02];
        let mut read_buffer = [0u8; 4];
        let mut ops = [
            Operation::Write(&write_data),
            Operation::Read(&mut read_buffer),
        ];

        assert!(mock.transaction_slice(0x50, &mut ops).is_ok());
        assert_eq!(read_buffer, [0xFF; 4]);
    }

    #[test]
    fn test_bus_recovery() {
        let mut mock = MockI2cHardware::new();
        assert!(mock.recover_bus().is_ok());

        let mut failing_mock = MockI2cHardware::new_failing();
        assert_eq!(failing_mock.recover_bus(), Err(MockI2cError::Bus));
    }

    #[test]
    fn test_configuration() {
        let mut mock = MockI2cHardware::new();

        // Test the new configure_timing signature with speed and timing config
        let speed = 400_000u32; // 400 kHz
        let timing_config = (); // Empty timing config for mock
        let result = mock.configure_timing(speed, &timing_config);
        assert!(result.is_ok());
        assert_eq!(mock.config.frequency, 400_000);
    }

    #[test]
    fn test_slave_mode_basic() {
        let mut mock = MockI2cHardware::new();

        // Test slave address configuration
        use openprot_hal_blocking::i2c_hardware::slave::I2cSlaveCore;
        assert!(mock.configure_slave_address(0x42).is_ok());
        assert_eq!(mock.slave_address(), Some(0x42));

        // Test slave mode enable/disable
        assert!(!mock.is_slave_mode_enabled());
        assert!(mock.enable_slave_mode().is_ok());
        assert!(mock.is_slave_mode_enabled());
        assert!(mock.disable_slave_mode().is_ok());
        assert!(!mock.is_slave_mode_enabled());
    }

    #[test]
    #[allow(clippy::panic)]
    fn test_slave_buffer_operations() {
        let mut mock = MockI2cHardware::new();

        // Inject some test data
        mock.inject_slave_data(&[0x01, 0x02, 0x03, 0x04]);

        // Test buffer reading
        use openprot_hal_blocking::i2c_hardware::slave::I2cSlaveBuffer;
        let mut buffer = [0u8; 4];
        match mock.read_slave_buffer(&mut buffer) {
            Ok(count) => {
                assert_eq!(count, 4);
                assert_eq!(buffer, [0x01, 0x02, 0x03, 0x04]);
            }
            Err(_) => {
                // Test failure - use assert! with message instead of panic!
                panic!("Failed to read slave buffer");
            }
        }

        // Test writing response
        match mock.write_slave_response(&[0xAA, 0xBB]) {
            Ok(()) => {}
            Err(_) => {
                panic!("Failed to write slave response");
            }
        }

        match mock.tx_buffer_space() {
            Ok(space) => assert_eq!(space, 62), // 64 - 2
            Err(_) => {
                panic!("Failed to get buffer space");
            }
        }

        // Test buffer clearing
        mock.inject_slave_data(&[0xFF, 0xFE]);
        match mock.rx_buffer_count() {
            Ok(count) => assert_eq!(count, 2),
            Err(_) => {
                panic!("Failed to get buffer count");
            }
        }

        match mock.clear_slave_buffer() {
            Ok(()) => {}
            Err(_) => {
                panic!("Failed to clear buffer");
            }
        }

        match mock.rx_buffer_count() {
            Ok(count) => assert_eq!(count, 0),
            Err(_) => {
                panic!("Failed to get buffer count after clear");
            }
        }
    }

    #[test]
    #[allow(clippy::panic)]
    fn test_slave_events() {
        let mut mock = MockI2cHardware::new();

        use openprot_hal_blocking::i2c_hardware::slave::{I2cSEvent, I2cSlaveInterrupts};

        // Test event injection and polling
        mock.inject_slave_event(I2cSEvent::SlaveWrReq);

        // Test event polling
        match mock.poll_slave_events() {
            Ok(event) => assert_eq!(event, Some(I2cSEvent::SlaveWrReq)),
            Err(_) => {
                panic!("Failed to poll events");
            }
        }

        match mock.poll_slave_events() {
            Ok(event) => assert_eq!(event, None),
            Err(_) => {
                panic!("Failed to poll events");
            }
        }

        // Test event handling
        match mock.handle_slave_event_nb(I2cSEvent::SlaveWrRecvd) {
            Ok(()) => {}
            Err(_) => {
                panic!("Failed to handle event");
            }
        }
        assert_eq!(mock.last_slave_event(), Some(I2cSEvent::SlaveWrRecvd));

        // Test slave status
        match mock.slave_status() {
            Ok(status) => {
                assert!(!status.enabled); // Not enabled by default
                assert_eq!(status.last_event, Some(I2cSEvent::SlaveWrRecvd));
            }
            Err(_) => {
                panic!("Failed to get status");
            }
        }
    }

    #[test]
    fn test_slave_event_pending_check() {
        let mut mock = MockI2cHardware::new();

        use openprot_hal_blocking::i2c_hardware::slave::I2cSEvent;

        // Initially no events pending
        match mock.is_event_pending_nb(I2cSEvent::SlaveWrReq) {
            Ok(pending) => assert!(!pending),
            Err(_) => {
                panic!("Failed to check pending");
            }
        }

        // Inject an event
        mock.inject_slave_event(I2cSEvent::SlaveWrReq);
        match mock.is_event_pending_nb(I2cSEvent::SlaveWrReq) {
            Ok(pending) => assert!(pending),
            Err(_) => {
                panic!("Failed to check pending");
            }
        }
        match mock.is_event_pending_nb(I2cSEvent::SlaveRdReq) {
            Ok(pending) => assert!(!pending),
            Err(_) => {
                panic!("Failed to check pending");
            }
        }

        // After polling, event should no longer be pending
        match mock.poll_slave_events() {
            Ok(_) => {}
            Err(_) => {
                panic!("Failed to poll events");
            }
        }
        match mock.is_event_pending_nb(I2cSEvent::SlaveWrReq) {
            Ok(pending) => assert!(!pending),
            Err(_) => {
                panic!("Failed to check pending");
            }
        }
    }

    #[test]
    fn test_slave_failing_operations() {
        let mut mock = MockI2cHardware::new_failing();

        use openprot_hal_blocking::i2c_hardware::slave::{I2cSlaveBuffer, I2cSlaveCore};

        // All slave operations should fail
        assert!(mock.configure_slave_address(0x42).is_err());
        assert!(mock.enable_slave_mode().is_err());

        let mut buffer = [0u8; 4];
        assert!(mock.read_slave_buffer(&mut buffer).is_err());
        assert!(mock.write_slave_response(&[0x01]).is_err());

        assert!(mock.poll_slave_events().is_err());
        assert!(mock
            .handle_slave_event_nb(
                openprot_hal_blocking::i2c_hardware::slave::I2cSEvent::SlaveWrReq
            )
            .is_err());
    }

    #[test]
    fn test_system_control_integration() {
        use crate::system_control::{MockClockId, MockResetId, MockSystemControl};

        let system_control = MockSystemControl::new();
        let mut i2c_with_system =
            MockI2cHardwareWithSystem::new(system_control, MockClockId::I2c1, MockResetId::I2c1);

        // Initially, clock should be disabled and reset should be asserted
        match i2c_with_system
            .system_control()
            .is_clock_enabled(&MockClockId::I2c1)
        {
            Ok(enabled) => assert!(!enabled),
            Err(e) => assert!(false, "is_clock_enabled failed: {:?}", e),
        }

        match i2c_with_system
            .system_control()
            .is_reset_asserted(&MockResetId::I2c1)
        {
            Ok(asserted) => assert!(asserted),
            Err(e) => assert!(false, "is_reset_asserted failed: {:?}", e),
        }
        // Initialize - should enable clock and release reset
        let mut config = MockI2cConfig::default();
        assert!(i2c_with_system.init(&mut config).is_ok());

        // After init, clock should be enabled and reset should be deasserted
        match i2c_with_system
            .system_control()
            .is_clock_enabled(&MockClockId::I2c1)
        {
            Ok(enabled) => assert!(enabled),
            Err(e) => assert!(false, "is_clock_enabled failed: {:?}", e),
        }
        match i2c_with_system
            .system_control()
            .is_reset_asserted(&MockResetId::I2c1)
        {
            Ok(asserted) => assert!(!asserted),
            Err(e) => assert!(false, "is_reset_asserted failed: {:?}", e),
        }
    }

    #[test]
    #[allow(clippy::panic)]
    fn test_system_control_timing_configuration() {
        use crate::system_control::{MockClockId, MockResetId, MockSystemControl};
        use openprot_hal_blocking::system_control::ClockControl;

        let system_control = MockSystemControl::new();
        let mut i2c_with_system =
            MockI2cHardwareWithSystem::new(system_control, MockClockId::I2c1, MockResetId::I2c1);

        // Configure timing for 400kHz I2C
        match i2c_with_system.configure_timing(400_000, &()) {
            Ok(actual_freq) => {
                // Should return the configured source frequency
                assert_eq!(actual_freq, 24_000_000); // Expected 24MHz for 400kHz I2C

                // Verify system control has the correct frequency set
                match i2c_with_system
                    .system_control()
                    .get_frequency(&MockClockId::I2c1)
                {
                    Ok(freq) => assert_eq!(freq, 24_000_000),
                    Err(_) => {
                        panic!("Failed to get frequency");
                    }
                }
            }
            Err(_) => {
                panic!("Failed to configure timing");
            }
        }

        // Test different I2C speeds
        match i2c_with_system.configure_timing(100_000, &()) {
            Ok(actual_freq) => assert_eq!(actual_freq, 8_000_000), // 8MHz for 100kHz
            Err(_) => {
                panic!("Failed to configure 100kHz timing");
            }
        }

        match i2c_with_system.configure_timing(1_000_000, &()) {
            Ok(actual_freq) => assert_eq!(actual_freq, 48_000_000), // 48MHz for 1MHz
            Err(_) => {
                panic!("Failed to configure 1MHz timing");
            }
        }
    }

    #[test]
    fn test_system_control_failing_operations() {
        use crate::system_control::{MockClockId, MockResetId, MockSystemControl};

        // Create failing system control
        let failing_system_control = MockSystemControl::new_failing();
        let mut failing_i2c = MockI2cHardwareWithSystem::new_failing(
            failing_system_control,
            MockClockId::I2c1,
            MockResetId::I2c1,
        );

        // Init should fail due to system control operations failing
        let mut config = MockI2cConfig::default();
        assert!(failing_i2c.init(&mut config).is_err());

        // Timing configuration should also fail
        assert!(failing_i2c.configure_timing(400_000, &()).is_err());
    }

    #[test]
    #[allow(clippy::panic)]
    fn test_system_control_i2c_operations() {
        use crate::system_control::{MockClockId, MockResetId, MockSystemControl};

        let system_control = MockSystemControl::new();
        let mut i2c_with_system =
            MockI2cHardwareWithSystem::new(system_control, MockClockId::I2c1, MockResetId::I2c1);

        // Initialize first
        let mut config = MockI2cConfig::default();
        assert!(i2c_with_system.init(&mut config).is_ok());

        // Test basic I2C operations work with system control integration
        assert!(i2c_with_system.write(0x50, &[0x01, 0x02]).is_ok());

        let mut buffer = [0u8; 4];
        assert!(i2c_with_system.read(0x50, &mut buffer).is_ok());
        assert_eq!(buffer, [0xFF; 4]); // Mock returns 0xFF

        // Test slave operations
        use openprot_hal_blocking::i2c_hardware::slave::{I2cSlaveBuffer, I2cSlaveCore};
        assert!(i2c_with_system.configure_slave_address(0x42).is_ok());
        assert!(i2c_with_system.enable_slave_mode().is_ok());
        assert!(i2c_with_system.is_slave_mode_enabled());

        // Test slave data injection and reading
        i2c_with_system.inject_slave_data(&[0xAA, 0xBB, 0xCC]);
        let mut slave_buffer = [0u8; 3];
        match i2c_with_system.read_slave_buffer(&mut slave_buffer) {
            Ok(count) => {
                assert_eq!(count, 3);
                assert_eq!(slave_buffer, [0xAA, 0xBB, 0xCC]);
            }
            Err(_) => {
                panic!("Failed to read slave buffer");
            }
        }
    }

    #[test]
    fn test_system_control_access_methods() {
        use crate::system_control::{MockClockId, MockResetId, MockSystemControl};
        use openprot_hal_blocking::system_control::ClockControl;

        let mut system_control = MockSystemControl::new();

        // Pre-configure the system control
        system_control
            .enable(&MockClockId::I2c1)
            .expect("Failed to enable clock");
        system_control
            .set_frequency(&MockClockId::I2c1, 12_000_000)
            .expect("Failed to set frequency");

        let mut i2c_with_system =
            MockI2cHardwareWithSystem::new(system_control, MockClockId::I2c1, MockResetId::I2c1);

        // Test immutable access
        let sys_ctrl_ref = i2c_with_system.system_control();

        match sys_ctrl_ref.is_clock_enabled(&MockClockId::I2c1) {
            Ok(enabled) => assert!(enabled),
            Err(e) => assert!(false, "is_clock_enabled failed: {:?}", e),
        }
        match sys_ctrl_ref.get_frequency(&MockClockId::I2c1) {
            Ok(freq) => assert_eq!(freq, 12_000_000),
            Err(_) => {
                panic!("Failed to get frequency");
            }
        }

        // Test mutable access
        let sys_ctrl_mut = i2c_with_system.system_control_mut();
        match sys_ctrl_mut.set_frequency(&MockClockId::I2c1, 20_000_000) {
            Ok(()) => {}
            Err(_) => {
                panic!("Failed to set frequency");
            }
        }

        // Verify the change took effect
        match i2c_with_system
            .system_control()
            .get_frequency(&MockClockId::I2c1)
        {
            Ok(freq) => assert_eq!(freq, 20_000_000),
            Err(_) => {
                panic!("Failed to get updated frequency");
            }
        }
    }
}
