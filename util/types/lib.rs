// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Common utility types.

#![no_std]

mod opcode;
mod power_of_2;

pub use opcode::Opcode;
pub use power_of_2::PowerOf2Usize;

/// A trait for blocking on notifications.
///
/// This trait is typically implemented by mechanisms that need to wait for
/// an event or notification from another part of the system (e.g., an interrupt).
pub trait Blocking {
    /// Waits until a notification is received.
    fn wait_for_notification(&self);
}
