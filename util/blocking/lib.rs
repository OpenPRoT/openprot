// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

pub mod pigweed;

/// A trait for blocking on notifications.
///
/// This trait is typically implemented by mechanisms that need to wait for
/// an event or notification from another part of the system (e.g., an interrupt).
pub trait Blocking {
    /// Waits until a notification is received.
    fn wait_for_notification(&self) -> impl BlockingAckToken;
}

/// A trait for values returned by wait_for_notification
///
/// Users must generally keep the value returned by wait_for_notification in
/// scope for as long as work is being performed in relation to the notification.
pub trait BlockingAckToken {
}
