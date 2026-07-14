// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIO type states, traits, and error types.

use core::marker::PhantomData;

/// All input modes implement this trait.
pub trait InputMode {}

/// All output modes implement this trait.
pub trait OutputMode {}

/// `OpenDrain` modes implement this trait.
pub trait OpenDrainMode {
    /// Returns whether pull-up is enabled.
    fn pup() -> bool;
}

/// Input mode (type state).
pub struct Input<MODE>
where
    MODE: InputMode,
{
    pub(super) _mode: PhantomData<MODE>,
}

/// Sub-mode of Input: Floating input (type state).
pub struct Floating;
impl InputMode for Floating {}
impl OpenDrainMode for Floating {
    fn pup() -> bool {
        false
    }
}

/// Sub-mode of Input: Pulled down input (type state).
pub struct PullDown;
impl InputMode for PullDown {}

/// Sub-mode of Input: Pulled up input (type state).
pub struct PullUp;
impl InputMode for PullUp {}
impl OpenDrainMode for PullUp {
    fn pup() -> bool {
        true
    }
}

/// Tri-state mode (type state).
pub struct Tristate;

/// Output mode (type state).
pub struct Output<MODE>
where
    MODE: OutputMode,
{
    pub(super) _mode: PhantomData<MODE>,
}

/// Sub-mode of Output: Push-pull output (type state).
pub struct PushPull;
impl OutputMode for PushPull {}
impl OutputMode for PullDown {}
impl OutputMode for PullUp {}

/// Sub-mode of Output: Open drain output (type state).
pub struct OpenDrain<ODM>
where
    ODM: OpenDrainMode,
{
    pub(super) _pull: PhantomData<ODM>,
}
impl<ODM> OutputMode for OpenDrain<ODM> where ODM: OpenDrainMode {}

/// Sets when a GPIO pin triggers an interrupt.
pub enum InterruptMode {
    /// Interrupt when level is low.
    LevelLow,
    /// Interrupt when level is high.
    LevelHigh,
    /// Interrupt on rising edge.
    EdgeRising,
    /// Interrupt on falling edge.
    EdgeFalling,
    /// Interrupt on both rising and falling edges.
    EdgeBoth,
    /// Disable interrupts on this pin.
    Disabled,
}

/// Extension trait to split a GPIO bank peripheral into individual pins.
pub trait GpioExt {
    /// The type returned by [`split`](GpioExt::split).
    type Parts;

    /// Consume the bank peripheral and return its individual pins.
    fn split(self) -> Self::Parts;
}

/// Error type for GPIO operations.
#[derive(Debug)]
pub enum GpioError {
    Unknown,
}

impl embedded_hal::digital::Error for GpioError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        match self {
            GpioError::Unknown => embedded_hal::digital::ErrorKind::Other,
        }
    }
}
