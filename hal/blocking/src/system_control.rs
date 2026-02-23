// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Design Considerations
//!
//! This module defines traits and error types for abstracting system-level
//! clock and reset control operations. The design emphasizes **flexibility**,
//! **composability**, and **extensibility** to support a wide range of hardware
//! platforms and use cases.
//!
//! ## Flexibility
//!
//! - **Platform Independence**: The traits abstract hardware-specific operations,
//!   allowing implementations to target different platforms without modifying
//!   consumer code.
//! - **Custom Identifiers and Configurations**: Associated types like `ClockId`,
//!   `ClockConfig`, and `ResetId` allow implementers to define identifiers and
//!   configurations that match their hardware model.
//! - **Error Abstraction**: The `Error` trait and `ErrorKind` enum decouple
//!   error handling from specific implementations, enabling generic code to
//!   respond to common failure modes while supporting detailed diagnostics.
//!
//! ## Composability
//!
//! - **Unified Error Handling**: The `ErrorType` supertrait provides a consistent
//!   interface for accessing error types, enabling integration with other traits
//!   and systems.
//! - **Non-Exhaustive ErrorKind**: The `#[non_exhaustive]` attribute on `ErrorKind`
//!   allows future expansion without breaking existing code, supporting long-term
//!   composability.
//!
//! ## Extensibility
//!
//! - **Custom Error Types**: Implementers can define their own error types and
//!   map them to `ErrorKind`, enabling rich, context-specific error reporting
//!   while maintaining compatibility with generic consumers.
//! - **Vendor-Specific Configuration**: The `ClockConfig` type supports complex
//!   configuration structures such as PLL settings, dividers, or source selectors.
//! - **Partial Implementations**: While all trait methods are required, implementers
//!   can provide no-op or stub implementations for unsupported features, enabling
//!   partial functionality where appropriate.
//!
//! ## Summary
//!
//! This design provides a robust foundation for building portable, maintainable,
//! and scalable hardware abstraction layers. By leveraging Rust’s trait system,
//! it enables:
//!
//! - Reuse of generic drivers across platforms.
//! - Simplified testing and mocking.
//! - Clear contracts between hardware abstraction and higher-level logic.

use core::time::Duration;

/// Represents common system control operation errors.
///
/// This enumeration defines a standard set of error conditions that can occur
/// during clock and reset control operations. Implementations are free to define
/// more specific or additional error types. However, by providing a mapping to
/// these common errors, generic code can still react to them appropriately.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The specified clock identifier was not found or is invalid.
    ClockNotFound,
    /// Attempted to enable a clock that is already enabled.
    ClockAlreadyEnabled,
    /// Attempted to disable a clock that is already disabled.
    ClockAlreadyDisabled,
    /// The specified clock frequency is invalid or not supported.
    InvalidClockFrequency,
    /// Clock configuration operation failed due to invalid parameters or hardware constraints.
    ClockConfigurationFailed,
    /// The specified reset identifier was not found or is invalid.
    InvalidResetId,
    /// A hardware-level failure occurred during the operation.
    HardwareFailure,
    /// The operation was denied due to insufficient permissions or security restrictions.
    PermissionDenied,
    /// The operation timed out before completion.
    Timeout,
}

/// Trait for system control error types.
///
/// This trait provides a standard interface for all error types used in
/// system control operations. It requires implementors to provide a mapping
/// to the common `ErrorKind` enumeration, enabling generic error handling
/// while preserving implementation-specific error details.
pub trait Error: core::fmt::Debug {
    /// Convert error to a generic error kind.
    ///
    /// By using this method, errors freely defined by system control implementations
    /// can be converted to a set of generic errors upon which generic
    /// code can act.
    fn kind(&self) -> ErrorKind;
}

impl Error for core::convert::Infallible {
    /// Convert error to a generic error kind.
    ///
    /// Since `core::convert::Infallible` represents an error that can never occur,
    /// this implementation uses pattern matching on the uninhabited type to
    /// ensure this method can never actually be called.    
    fn kind(&self) -> ErrorKind {
        match *self {}
    }
}

/// Trait providing access to the associated error type.
///
/// This trait serves as a foundation for other traits that need to define
/// error handling. By separating error type definition from specific operations,
/// it enables composition and reuse across different trait implementations.
pub trait ErrorType {
    /// The error type used by this implementation.
    ///
    /// This associated type must implement the `Error` trait to ensure
    /// it can be converted to generic error kinds for interoperability.
    type Error: Error;
}

/// Trait for clock control operations.
/// Abstracts enabling, disabling, and configuring clocks for peripherals or system components.
pub trait ClockControl: ErrorType {
    /// Type for identifying a clock (e.g., peripheral ID, clock name, or register offset).
    type ClockId: Clone + PartialEq;
    /// Type for configuring a clock.
    type ClockConfig: PartialEq;

    /// Enables a clock for the specified clock ID.
    ///
    /// # Arguments
    ///
    /// * `clock_id` - A reference to the identifier of the clock to enable.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn enable(&mut self, clock_id: &Self::ClockId) -> Result<(), Self::Error>;

    /// Disables a clock for the specified clock ID.
    ///
    /// # Arguments
    ///
    /// * `clock_id` - A reference to the identifier of the clock to disable.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn disable(&mut self, clock_id: &Self::ClockId) -> Result<(), Self::Error>;

    /// Sets the frequency of a clock (in Hz).
    ///
    /// # Arguments
    ///
    /// * `clock_id` - A reference to the identifier of the clock to set the frequency for.
    /// * `frequency_hz` - The frequency to set, in Hertz.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn set_frequency(
        &mut self,
        clock_id: &Self::ClockId,
        frequency_hz: u64,
    ) -> Result<(), Self::Error>;

    /// Gets the current frequency of a clock (in Hz).
    ///
    /// # Arguments
    ///
    /// * `clock_id` - A reference to the identifier of the clock to get the frequency for.
    ///
    /// # Returns
    ///
    /// * `Result<u64, Self::Error>` - Ok with the current frequency in Hertz, or an error of type `Self::Error`.
    fn get_frequency(&self, clock_id: &Self::ClockId) -> Result<u64, Self::Error>;

    /// Configures clock-specific parameters (e.g., divider, source).
    /// Vendor-specific parameters can be passed via `ClockConfig`.
    ///
    /// # Arguments
    ///
    /// * `clock_id` - A reference to the identifier of the clock to configure.
    /// * `config` - The configuration parameters for the clock.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn configure(
        &mut self,
        clock_id: &Self::ClockId,
        config: Self::ClockConfig,
    ) -> Result<(), Self::Error>;

    /// Retrieves the current configuration of a clock.
    ///
    /// # Arguments
    ///
    /// * `clock_id` - A reference to the identifier of the clock to get the configuration for.
    ///
    /// # Returns
    ///
    /// * `Result<Self::ClockConfig, Self::Error>` - Ok with the current configuration, or an error of type `Self::Error`.
    fn get_config(&self, clock_id: &Self::ClockId) -> Result<Self::ClockConfig, Self::Error>;
}

/// Trait for reset control operations.
/// Abstracts asserting and deasserting reset signals for peripherals or system components.
pub trait ResetControl: ErrorType {
    /// Type for identifying a reset line (e.g., peripheral ID, reset name, or register offset).
    type ResetId: Clone + PartialEq;

    /// Asserts the reset signal for the specified reset ID (holds the component in reset).
    ///
    /// # Arguments
    ///
    /// * `reset_id` - A reference to the identifier of the reset line to assert.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn reset_assert(&mut self, reset_id: &Self::ResetId) -> Result<(), Self::Error>;

    /// Deasserts the reset signal for the specified reset ID (releases the component from reset).
    ///
    /// # Arguments
    ///
    /// * `reset_id` - A reference to the identifier of the reset line to deassert.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn reset_deassert(&mut self, reset_id: &Self::ResetId) -> Result<(), Self::Error>;

    /// Performs a pulse reset (asserts then deasserts) with a specified duration (in microseconds).
    ///
    /// # Arguments
    ///
    /// * `reset_id` - A reference to the identifier of the reset line to pulse.
    /// * `duration_us` - The duration of the pulse in microseconds.
    ///
    /// # Returns
    ///
    /// * `Result<(), Self::Error>` - Ok if the operation is successful, or an error of type `Self::Error`.
    fn reset_pulse(
        &mut self,
        reset_id: &Self::ResetId,
        duration: Duration,
    ) -> Result<(), Self::Error>;

    /// Checks if the reset signal is currently asserted for the specified reset ID.
    ///
    /// # Arguments
    ///
    /// * `reset_id` - A reference to the identifier of the reset line to check.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Self::Error>` - Ok with a boolean indicating if the reset is asserted, or an error of type `Self::Error`.
    fn reset_is_asserted(&self, reset_id: &Self::ResetId) -> Result<bool, Self::Error>;
}

/// Blanket trait that combines clock and reset control functionality.
///
/// This trait provides a unified interface for system control operations,
/// combining both clock management and reset control capabilities. It is
/// automatically implemented for any type that implements both `ClockControl`
/// and `ResetControl` with the same error type.
///
/// # Design
///
/// The blanket implementation pattern allows consumers to use a single trait
/// bound for system control operations while maintaining the flexibility of
/// separate, composable traits for clock and reset functionality.
///
/// # Example
///
/// ```rust,ignore
/// fn configure_system<T: SystemControl>(controller: &mut T)
/// where
///     T::ClockId: From<u32>,
///     T::ResetId: From<u32>,
///     T::ClockConfig: Default,
/// {
///     // Enable peripheral clock
///     controller.enable(&T::ClockId::from(42)).unwrap();
///
///     // Release from reset
///     controller.reset_deassert(&T::ResetId::from(42)).unwrap();
/// }
/// ```
pub trait SystemControl: ClockControl + ResetControl {}

/// Blanket implementation of SystemControl for types implementing both traits.
///
/// This implementation is automatically provided for any type that implements
/// both `ClockControl` and `ResetControl` with compatible error types.
impl<T> SystemControl for T where T: ClockControl + ResetControl {}
