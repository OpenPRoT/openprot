// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock system control implementation for testing and development
//!
//! This module provides a simple mock implementation of the SystemControl traits
//! that can be used for testing I2C hardware with external system control dependencies.
//! The mock simulates clock and reset control operations without actual hardware interaction.
//!
//! # Features
//!
//! - **Clock Control**: Enable/disable clocks, set/get frequencies, configure parameters
//! - **Reset Control**: Assert/deassert resets, pulse reset with timing
//! - **Configurable Behavior**: Success/failure modes for testing error paths
//! - **State Tracking**: Tracks clock and reset states for verification
//! - **Realistic Simulation**: Provides reasonable default frequencies and timing
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```text
//! use openprot_platform_mock::system_control::{MockSystemControl, MockClockId, MockResetId};
//! use openprot_hal_blocking::system_control::{ClockControl, ResetControl};
//!
//! let mut sys_ctrl = MockSystemControl::new();
//!
//! // Enable I2C clock
//! let clock_id = MockClockId::I2c1;
//! sys_ctrl.enable(&clock_id).unwrap();
//!
//! // Configure clock frequency
//! sys_ctrl.set_frequency(&clock_id, 48_000_000).unwrap(); // 48 MHz
//!
//! // Release I2C from reset
//! let reset_id = MockResetId::I2c1;
//! sys_ctrl.reset_deassert(&reset_id).unwrap();
//! ```
//!
//! ## Error Testing
//!
//! ```text
//! use openprot_platform_mock::system_control::MockSystemControl;
//!
//! // Create failing mock for error path testing
//! let mut failing_ctrl = MockSystemControl::new_failing();
//!
//! // All operations will fail
//! let result = failing_ctrl.enable(&MockClockId::I2c1);
//! assert!(result.is_err());
//! ```

use core::time::Duration;
use openprot_hal_blocking::system_control::{
    ClockControl, Error, ErrorKind, ErrorType, ResetControl,
};

/// Mock error type for system control operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSystemControlError {
    /// Clock operation failed
    ClockError,
    /// Reset operation failed
    ResetError,
    /// Invalid configuration
    InvalidConfig,
    /// Hardware simulation failure
    HardwareFailure,
}

impl Error for MockSystemControlError {
    fn kind(&self) -> ErrorKind {
        match self {
            MockSystemControlError::ClockError => ErrorKind::ClockConfigurationFailed,
            MockSystemControlError::ResetError => ErrorKind::HardwareFailure,
            MockSystemControlError::InvalidConfig => ErrorKind::InvalidClockFrequency,
            MockSystemControlError::HardwareFailure => ErrorKind::HardwareFailure,
        }
    }
}

/// Mock clock identifiers for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MockClockId {
    /// I2C controller 1 clock
    I2c1,
    /// I2C controller 2 clock
    I2c2,
    /// System clock
    SystemClock,
    /// Peripheral clock
    PeripheralClock,
}

impl From<u32> for MockClockId {
    fn from(value: u32) -> Self {
        match value {
            0 => MockClockId::I2c1,
            1 => MockClockId::I2c2,
            2 => MockClockId::SystemClock,
            3 => MockClockId::PeripheralClock,
            _ => MockClockId::I2c1, // Default fallback
        }
    }
}

/// Mock reset identifiers for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MockResetId {
    /// I2C controller 1 reset
    I2c1,
    /// I2C controller 2 reset
    I2c2,
    /// System reset
    SystemReset,
    /// Peripheral reset
    PeripheralReset,
}

impl From<u32> for MockResetId {
    fn from(value: u32) -> Self {
        match value {
            0 => MockResetId::I2c1,
            1 => MockResetId::I2c2,
            2 => MockResetId::SystemReset,
            3 => MockResetId::PeripheralReset,
            _ => MockResetId::I2c1, // Default fallback
        }
    }
}

/// Mock clock configuration parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MockClockConfig {
    /// Clock divider value
    pub divider: u32,
    /// Clock source selector
    pub source: MockClockSource,
    /// Enable PLL if applicable
    pub enable_pll: bool,
}

impl Default for MockClockConfig {
    fn default() -> Self {
        Self {
            divider: 1,
            source: MockClockSource::Internal,
            enable_pll: false,
        }
    }
}

/// Mock clock source options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockClockSource {
    /// Internal oscillator
    Internal,
    /// External crystal
    External,
    /// PLL output
    Pll,
}

/// Internal state for clock tracking
#[derive(Debug, Clone, Copy)]
struct ClockState {
    enabled: bool,
    frequency: u64,
    config: MockClockConfig,
}

impl Default for ClockState {
    fn default() -> Self {
        Self {
            enabled: false,
            frequency: 100_000_000, // Default 100 MHz
            config: MockClockConfig::default(),
        }
    }
}

/// Internal state for reset tracking
#[derive(Debug, Clone, Copy)]
struct ResetState {
    asserted: bool,
}

impl Default for ResetState {
    fn default() -> Self {
        Self {
            asserted: true, // Start in reset
        }
    }
}

/// Mock system control implementation
///
/// Provides a realistic simulation of system clock and reset control
/// functionality for testing I2C hardware integration patterns.
pub struct MockSystemControl {
    /// Whether operations should succeed (for error testing)
    success_mode: bool,
    /// Clock states indexed by MockClockId
    clock_states: [ClockState; 4],
    /// Reset states indexed by MockResetId
    reset_states: [ResetState; 4],
}

impl MockSystemControl {
    /// Create a new mock system control in success mode
    pub fn new() -> Self {
        Self {
            success_mode: true,
            clock_states: [ClockState::default(); 4],
            reset_states: [ResetState::default(); 4],
        }
    }

    /// Create a new mock that fails all operations (for error testing)
    pub fn new_failing() -> Self {
        Self {
            success_mode: false,
            clock_states: [ClockState::default(); 4],
            reset_states: [ResetState::default(); 4],
        }
    }

    /// Check if operations should succeed
    fn check_success(&self) -> Result<(), MockSystemControlError> {
        if self.success_mode {
            Ok(())
        } else {
            Err(MockSystemControlError::HardwareFailure)
        }
    }

    /// Convert clock ID to array index
    fn clock_index(&self, clock_id: &MockClockId) -> usize {
        match clock_id {
            MockClockId::I2c1 => 0,
            MockClockId::I2c2 => 1,
            MockClockId::SystemClock => 2,
            MockClockId::PeripheralClock => 3,
        }
    }

    /// Convert reset ID to array index
    fn reset_index(&self, reset_id: &MockResetId) -> usize {
        match reset_id {
            MockResetId::I2c1 => 0,
            MockResetId::I2c2 => 1,
            MockResetId::SystemReset => 2,
            MockResetId::PeripheralReset => 3,
        }
    }

    /// Check if a clock is enabled (for testing)
    pub fn is_clock_enabled(
        &self,
        clock_id: &<MockSystemControl as ClockControl>::ClockId,
    ) -> Result<bool, <MockSystemControl as ErrorType>::Error> {
        let index = self.clock_index(clock_id);
        match self.clock_states.get(index) {
            Some(state) => Ok(state.enabled),
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    /// Check if a reset is asserted (for testing)
    pub fn is_reset_asserted(
        &self,
        reset_id: &<MockSystemControl as ResetControl>::ResetId,
    ) -> Result<bool, <MockSystemControl as ErrorType>::Error> {
        let index = self.reset_index(reset_id);
        match self.reset_states.get(index) {
            Some(state) => Ok(state.asserted),
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }
}

impl Default for MockSystemControl {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorType for MockSystemControl {
    type Error = MockSystemControlError;
}

impl ClockControl for MockSystemControl {
    type ClockId = MockClockId;
    type ClockConfig = MockClockConfig;

    fn enable(&mut self, clock_id: &Self::ClockId) -> Result<(), Self::Error> {
        self.check_success()?;
        let index = self.clock_index(clock_id);
        match self.clock_states.get_mut(index) {
            Some(state) => {
                state.enabled = true;
                Ok(())
            }
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn disable(&mut self, clock_id: &Self::ClockId) -> Result<(), Self::Error> {
        self.check_success()?;
        let index = self.clock_index(clock_id);
        match self.clock_states.get_mut(index) {
            Some(state) => {
                state.enabled = false;
                Ok(())
            }
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn set_frequency(
        &mut self,
        clock_id: &Self::ClockId,
        frequency_hz: u64,
    ) -> Result<(), Self::Error> {
        self.check_success()?;
        let index = self.clock_index(clock_id);
        match self.clock_states.get_mut(index) {
            Some(state) => {
                state.frequency = frequency_hz;
                Ok(())
            }
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn get_frequency(&self, clock_id: &Self::ClockId) -> Result<u64, Self::Error> {
        self.check_success()?;
        let index = self.clock_index(clock_id);
        match self.clock_states.get(index) {
            Some(state) => Ok(state.frequency),
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn configure(
        &mut self,
        clock_id: &Self::ClockId,
        config: Self::ClockConfig,
    ) -> Result<(), Self::Error> {
        self.check_success()?;
        let index = self.clock_index(clock_id);
        match self.clock_states.get_mut(index) {
            Some(state) => {
                state.config = config;
                // Simulate frequency adjustment based on configuration
                let base_freq = state.frequency;
                let adjusted_freq = base_freq
                    .checked_div(config.divider as u64)
                    .ok_or(MockSystemControlError::InvalidConfig)?;
                state.frequency = adjusted_freq;
                Ok(())
            }
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn get_config(&self, clock_id: &Self::ClockId) -> Result<Self::ClockConfig, Self::Error> {
        self.check_success()?;
        let index = self.clock_index(clock_id);
        match self.clock_states.get(index) {
            Some(state) => Ok(state.config),
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }
}

impl ResetControl for MockSystemControl {
    type ResetId = MockResetId;

    fn reset_assert(&mut self, reset_id: &Self::ResetId) -> Result<(), Self::Error> {
        self.check_success()?;
        let index = self.reset_index(reset_id);
        // For asserting reset:
        match self.reset_states.get_mut(index) {
            Some(state) => {
                state.asserted = true;
                Ok(())
            }
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn reset_deassert(&mut self, reset_id: &Self::ResetId) -> Result<(), Self::Error> {
        self.check_success()?;
        let index = self.reset_index(reset_id);
        // For deasserting reset:
        match self.reset_states.get_mut(index) {
            Some(state) => {
                state.asserted = false;
                Ok(())
            }
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }

    fn reset_pulse(
        &mut self,
        reset_id: &Self::ResetId,
        _duration: Duration,
    ) -> Result<(), Self::Error> {
        self.check_success()?;
        // Simulate pulse: assert, wait (simulated), then deassert
        self.reset_assert(reset_id)?;
        self.reset_deassert(reset_id)?;
        Ok(())
    }

    fn reset_is_asserted(&self, reset_id: &Self::ResetId) -> Result<bool, Self::Error> {
        self.check_success()?;
        let index = self.reset_index(reset_id);
        match self.reset_states.get(index) {
            Some(state) => Ok(state.asserted),
            None => Err(MockSystemControlError::HardwareFailure),
        }
    }
}

// SystemControl is automatically implemented via blanket implementation
// since MockSystemControl implements both ClockControl and ResetControl

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::assertions_on_constants)] // Allow assert!(false, "message") in tests for clear error messages
    #[test]
    fn test_mock_system_control_creation() {
        let sys_ctrl = MockSystemControl::new();
        assert!(sys_ctrl.success_mode);

        let failing_ctrl = MockSystemControl::new_failing();
        assert!(!failing_ctrl.success_mode);
    }

    #[test]
    fn test_clock_operations() {
        let mut sys_ctrl = MockSystemControl::new();
        let clock_id = MockClockId::I2c1;

        // Initially disabled
        match sys_ctrl.is_clock_enabled(&clock_id) {
            Ok(enabled) => assert!(!enabled),
            Err(e) => assert!(false, "is_clock_enabled failed: {:?}", e),
        }

        // Enable clock
        assert!(sys_ctrl.enable(&clock_id).is_ok());
        match sys_ctrl.is_clock_enabled(&clock_id) {
            Ok(enabled) => assert!(enabled),
            Err(e) => assert!(false, "is_clock_enabled failed: {:?}", e),
        }

        // Set frequency
        assert!(sys_ctrl.set_frequency(&clock_id, 48_000_000).is_ok());
        match sys_ctrl.get_frequency(&clock_id) {
            Ok(freq) => assert_eq!(freq, 48_000_000),
            Err(_) => {
                panic!("Failed to get frequency");
            }
        }

        // Disable clock
        assert!(sys_ctrl.disable(&clock_id).is_ok());
        match sys_ctrl.is_clock_enabled(&clock_id) {
            Ok(enabled) => assert!(!enabled),
            Err(e) => assert!(false, "is_clock_enabled failed: {:?}", e),
        }
    }

    #[test]
    fn test_reset_operations() {
        let mut sys_ctrl = MockSystemControl::new();
        let reset_id = MockResetId::I2c1;

        // Initially in reset
        match sys_ctrl.is_reset_asserted(&reset_id) {
            Ok(asserted) => assert!(asserted),
            Err(e) => assert!(false, "is_reset_asserted failed: {:?}", e),
        }
        // Deassert reset
        assert!(sys_ctrl.reset_deassert(&reset_id).is_ok());
        match sys_ctrl.is_reset_asserted(&reset_id) {
            Ok(asserted) => assert!(!asserted),
            Err(e) => assert!(false, "is_reset_asserted failed: {:?}", e),
        }
        // Assert reset
        assert!(sys_ctrl.reset_assert(&reset_id).is_ok());
        match sys_ctrl.is_reset_asserted(&reset_id) {
            Ok(asserted) => assert!(asserted),
            Err(e) => assert!(false, "is_reset_asserted failed: {:?}", e),
        }
        // Pulse reset
        assert!(sys_ctrl
            .reset_pulse(&reset_id, Duration::from_millis(1))
            .is_ok());
        match sys_ctrl.is_reset_asserted(&reset_id) {
            Ok(asserted) => assert!(asserted == false),
            Err(e) => assert!(false, "is_reset_asserted failed: {:?}", e),
        }
    }

    #[test]
    fn test_clock_configuration() {
        let mut sys_ctrl = MockSystemControl::new();
        let clock_id = MockClockId::I2c1;

        let config = MockClockConfig {
            divider: 4,
            source: MockClockSource::External,
            enable_pll: true,
        };

        // Set initial frequency
        assert!(sys_ctrl.set_frequency(&clock_id, 200_000_000).is_ok()); // 200 MHz

        // Configure clock (should divide by 4)
        assert!(sys_ctrl.configure(&clock_id, config).is_ok());

        // Check adjusted frequency
        match sys_ctrl.get_frequency(&clock_id) {
            Ok(freq) => assert_eq!(freq, 50_000_000), // 200 MHz / 4 = 50 MHz
            Err(_) => {
                panic!("Failed to get frequency");
            }
        }

        // Verify configuration was stored
        match sys_ctrl.get_config(&clock_id) {
            Ok(stored_config) => assert_eq!(stored_config, config),
            Err(_) => {
                panic!("Failed to get config");
            }
        }
    }

    #[test]
    fn test_failing_operations() {
        let mut failing_ctrl = MockSystemControl::new_failing();
        let clock_id = MockClockId::I2c1;
        let reset_id = MockResetId::I2c1;

        // All operations should fail
        assert!(failing_ctrl.enable(&clock_id).is_err());
        assert!(failing_ctrl.disable(&clock_id).is_err());
        assert!(failing_ctrl.set_frequency(&clock_id, 1000).is_err());
        assert!(failing_ctrl.get_frequency(&clock_id).is_err());
        assert!(failing_ctrl.reset_assert(&reset_id).is_err());
        assert!(failing_ctrl.reset_deassert(&reset_id).is_err());
        assert!(failing_ctrl
            .reset_pulse(&reset_id, Duration::from_millis(1))
            .is_err());
        assert!(failing_ctrl.reset_is_asserted(&reset_id).is_err());
    }

    #[test]
    fn test_id_conversions() {
        // Test clock ID conversions
        assert_eq!(MockClockId::from(0), MockClockId::I2c1);
        assert_eq!(MockClockId::from(1), MockClockId::I2c2);
        assert_eq!(MockClockId::from(999), MockClockId::I2c1); // Default fallback

        // Test reset ID conversions
        assert_eq!(MockResetId::from(0), MockResetId::I2c1);
        assert_eq!(MockResetId::from(1), MockResetId::I2c2);
        assert_eq!(MockResetId::from(999), MockResetId::I2c1); // Default fallback
    }
}
