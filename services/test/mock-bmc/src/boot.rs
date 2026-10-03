// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the modelled device does, and where it is in doing it.

/// What the mock BMC does once reset is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootBehaviour {
    /// Raises the ready line `after_millis` after release. A boot that is
    /// late rather than healthy is the same variant with a delay past the
    /// window the RoT is willing to wait, so there is no third variant for
    /// it: late is a property of the deadline, not of the device.
    Boots {
        /// Milliseconds from release to the ready line going active.
        after_millis: u64,
    },
    /// Never raises the ready line. The RoT finds out by its deadline
    /// expiring, which is the only way it ever finds out.
    Hangs,
}

/// Where the modelled device is in its boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Reset is asserted. The ready line is inactive.
    Held,
    /// Reset is released and the boot delay has not elapsed.
    Booting {
        /// When the delay started, in the caller's millisecond count.
        released_at_millis: u64,
    },
    /// The ready line is active.
    Ready,
    /// Reset is released and this device will never report ready.
    Hung,
}
