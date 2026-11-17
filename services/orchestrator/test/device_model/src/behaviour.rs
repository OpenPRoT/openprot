// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What a device does on one boot attempt, and where it is in doing it.

/// What one boot attempt does. A script is a list of these, one per release
/// of reset; the last entry repeats once the script runs out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    /// Never raises ready.
    Hang,
    /// Raises ready `after_millis` after the release and stays up. A boot
    /// that is late rather than healthy is this variant with a delay past
    /// the window the orchestrator will wait: late is a property of the
    /// deadline, not of the device.
    Boots {
        /// Milliseconds from release to ready.
        after_millis: u64,
    },
    /// Boots, then dies: raises ready `after_millis` after the release,
    /// drops it `up_millis` later, and stays down until reset is asserted
    /// and released again. A device that took an update and never came back,
    /// or one that fell over after reporting in.
    BootsThenFails {
        /// Milliseconds from release to ready.
        after_millis: u64,
        /// Milliseconds the device stays ready before it fails.
        up_millis: u64,
    },
}

/// Where the device is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Reset is asserted. Ready is low.
    Held,
    /// Reset is released and the boot delay has not elapsed.
    Booting,
    /// Ready is high.
    Ready,
    /// Reset is released and this attempt will never report ready.
    Hung,
    /// The device was ready and failed. Ready is low until the next reset.
    Down,
}

/// A change of the ready level, stamped with when it was due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// When the change happened, on the caller's clock.
    pub at_millis: u64,
    /// The level it changed to: `true` is ready.
    pub ready: bool,
}
