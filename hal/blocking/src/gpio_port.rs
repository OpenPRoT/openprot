// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

/// This represents a common set of GPIO operation errors. Implementations are
/// free to define more specific or additional error types. However, by providing
/// a mapping to these common errors, generic code can still react to them.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum GpioErrorKind {
    /// The specified GPIO port does not exist
    InvalidPort,
    /// The specified pin(s) do not exist on this port
    InvalidPin,
    /// The requested configuration is not supported
    UnsupportedConfiguration,
    /// The pins cannot be configured as requested (e.g., reserved pins)
    ConfigurationFailed,
    /// Cannot change pins currently used by another peripheral
    PinInUse,
    /// The interrupt requested cannot be configured
    InterruptConfigurationFailed,
    /// The requested operation is not allowed in the current state
    PermissionDenied,
    /// Hardware failure during operation
    HardwareFailure,
    /// Operation timed out
    Timeout,
    /// The pin is not configured for the requested operation
    /// (e.g., reading output value from input pin)
    InvalidMode,
}

/// Trait for GPIO errors
pub trait GpioError: core::fmt::Debug {
    /// Convert error to a generic error kind
    ///
    /// By using this method, errors freely defined by GPIO implementations
    /// can be converted to a set of generic errors upon which generic
    /// code can act.
    fn kind(&self) -> GpioErrorKind;
}

impl GpioError for core::convert::Infallible {
    fn kind(&self) -> GpioErrorKind {
        match *self {}
    }
}

/// Edge sensitivity for interrupt configuration
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EdgeSensitivity {
    /// Trigger on rising edge
    RisingEdge,
    /// Trigger on falling edge
    FallingEdge,
    /// Trigger on both rising and falling edges
    BothEdges,
    /// Trigger on high level
    HighLevel,
    /// Trigger on low level
    LowLevel,
}

/// Operations for interrupt control
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InterruptOperation {
    /// Enable interrupts
    Enable,
    /// Disable interrupts
    Disable,
    /// Clear pending interrupts
    Clear,
    /// Check if interrupts are pending
    IsPending,
}

/// Trait for types that define an error type for GPIO operations
pub trait GpioErrorType {
    /// Error type for GPIO operations
    type Error: GpioError;
}

/// Type used to represent pin masks
pub trait PinMask: Copy + core::fmt::Debug {
    /// Create an empty mask (no pins selected)
    fn empty() -> Self;

    /// Create a mask with all pins selected
    fn all() -> Self;

    /// Check if the mask is empty
    fn is_empty(&self) -> bool;

    /// Check if the mask contains the specified pins
    fn contains(&self, other: Self) -> bool;

    /// Merge two masks
    fn union(&self, other: Self) -> Self;

    /// Get intersection of two masks
    fn intersection(&self, other: Self) -> Self;

    /// Toggle pins in mask
    fn toggle(&self) -> Self;
}

/// Base trait for GPIO port operations with integrated error handling
pub trait GpioPort: GpioErrorType {
    /// Configuration type for GPIO pins
    type Config;

    /// Mask type for pin identification
    type Mask: PinMask;

    /// Configure GPIO pins with specified configuration
    fn configure(&mut self, pins: Self::Mask, config: Self::Config) -> Result<(), Self::Error>;

    /// Set and clear pins atomically using set and reset masks
    fn set_reset(
        &mut self,
        set_mask: Self::Mask,
        reset_mask: Self::Mask,
    ) -> Result<(), Self::Error>;

    /// Read current state of input pins
    fn read_input(&self) -> Result<Self::Mask, Self::Error>;

    /// Toggle specified output pins
    fn toggle(&mut self, pins: Self::Mask) -> Result<(), Self::Error>;
}

/// Trait for GPIO interrupt capabilities with integrated error handling
pub trait GpioInterrupt: GpioErrorType {
    /// Mask type for pin identification
    type Mask: PinMask;

    /// Configure interrupt sensitivity for specified pins
    fn irq_configure(
        &mut self,
        mask: Self::Mask,
        sensitivity: EdgeSensitivity,
    ) -> Result<(), Self::Error>;

    /// Control interrupt operations (enable, disable, etc.)
    fn irq_control(
        &mut self,
        mask: Self::Mask,
        operation: InterruptOperation,
    ) -> Result<bool, Self::Error>;

    /// Register a callback for interrupt handling
    fn register_interrupt_handler<F>(
        &mut self,
        mask: Self::Mask,
        handler: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Self::Mask) + Send + 'static;
}

/// Trait for splitting a GPIO port into individual pins
pub trait SplitPort: GpioPort + Sized {
    /// Container type returned when splitting the port
    type PortPins;

    /// Split the port into a container of pins
    fn split(self) -> Self::PortPins;
}

/// Combined trait for full GPIO functionality
pub trait GpioController: GpioPort + GpioInterrupt {}

/// Automatically implement GpioController for any type implementing both required traits
impl<T: GpioPort + GpioInterrupt> GpioController for T {}
