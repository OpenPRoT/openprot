// Licensed under the Apache-2.0 license

//! I2C target mode operations.
//!
//! This module provides types and traits for operating an I2C controller
//! in target (slave) mode, allowing the device to respond to transactions
//! initiated by other controllers on the bus.
//!
//! This is essential for protocols like MCTP where the device needs to
//! receive messages from a BMC or other system controller.

use core::time::Duration;

use embedded_hal::i2c::ErrorType;

use crate::address::I2cAddress;
use crate::client::BusIndex;

/// I2C target mode operations.
///
/// Allows the device to respond to I2C transactions initiated
/// by other controllers on the bus. Uses notification-based message
/// delivery rather than polling for efficiency.
///
/// # Examples
///
/// ```rust,ignore
/// use i2c_api::{I2cTargetClient, I2cAddress, BusIndex, TargetMessage};
/// use core::time::Duration;
///
/// fn setup_target<C: I2cTargetClient>(
///     client: &mut C,
///     bus: BusIndex,
///     my_address: I2cAddress,
/// ) -> Result<(), C::Error> {
///     // Configure our address and enable receive
///     client.configure_target_address(bus, my_address)?;
///     client.enable_receive(bus)?;
///     
///     // Wait for incoming messages
///     let mut messages = [TargetMessage::default(); 4];
///     let count = client.wait_for_messages(
///         bus,
///         &mut messages,
///         Some(Duration::from_secs(5)),
///     )?;
///     
///     for msg in &messages[..count] {
///         process_message(msg);
///     }
///     Ok(())
/// }
/// ```
pub trait I2cTargetClient: ErrorType {
    /// Configure this controller to respond at the given address.
    ///
    /// The controller will ACK transactions sent to this address.
    fn configure_target_address(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
    ) -> Result<(), Self::Error>;

    /// Enable target receive mode.
    ///
    /// After this call, incoming transactions to the configured
    /// address will be buffered and trigger notifications if registered.
    fn enable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error>;

    /// Disable target receive mode.
    ///
    /// Stops accepting incoming transactions. Buffered messages
    /// are retained until retrieved.
    fn disable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error>;

    /// Wait for incoming target messages.
    ///
    /// Blocks until one or more messages are available, or timeout expires.
    /// Returns the number of messages retrieved.
    ///
    /// # Arguments
    ///
    /// * `bus` - I2C bus to receive on
    /// * `messages` - Buffer to store received messages
    /// * `timeout` - Optional timeout; `None` waits indefinitely
    fn wait_for_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
        timeout: Option<Duration>,
    ) -> Result<usize, Self::Error>;

    /// Register a notification callback for incoming messages.
    ///
    /// When a target message arrives, the kernel will post a notification
    /// to the calling task using the provided mask. The task can then
    /// call `get_pending_messages` to retrieve the buffered data.
    ///
    /// # Arguments
    ///
    /// * `bus` - I2C bus to monitor
    /// * `notification_mask` - Bit mask to use for notifications
    fn register_notification(
        &mut self,
        bus: BusIndex,
        notification_mask: u32,
    ) -> Result<(), Self::Error>;

    /// Retrieve pending messages after receiving a notification.
    ///
    /// Call this after receiving a target message notification.
    /// Returns the number of messages retrieved.
    ///
    /// # Arguments
    ///
    /// * `bus` - I2C bus to read from
    /// * `messages` - Buffer to store pending messages
    fn get_pending_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
    ) -> Result<usize, Self::Error>;
}

/// Maximum size of a target message payload.
pub const TARGET_MESSAGE_MAX_LEN: usize = 255;

/// A message received in target mode.
///
/// Contains the data received from a controller and metadata
/// about the transaction.
#[derive(Clone)]
pub struct TargetMessage {
    /// Address of the controller that sent this message.
    pub source_address: I2cAddress,
    /// Message data buffer.
    data: [u8; TARGET_MESSAGE_MAX_LEN],
    /// Actual length of data.
    len: u8,
}

impl TargetMessage {
    /// Creates a new empty target message.
    pub const fn new() -> Self {
        Self {
            source_address: I2cAddress::GENERAL_CALL,
            data: [0u8; TARGET_MESSAGE_MAX_LEN],
            len: 0,
        }
    }

    /// Creates a target message with the given data.
    ///
    /// # Panics
    ///
    /// Panics if `data.len() > TARGET_MESSAGE_MAX_LEN`.
    pub fn from_data(source_address: I2cAddress, data: &[u8]) -> Self {
        assert!(data.len() <= TARGET_MESSAGE_MAX_LEN);
        let mut msg = Self::new();
        msg.source_address = source_address;
        msg.data[..data.len()].copy_from_slice(data);
        msg.len = data.len() as u8;
        msg
    }

    /// Returns the message data.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    /// Returns a mutable reference to the message data buffer.
    ///
    /// Use `set_len` to update the valid data length after writing.
    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8; TARGET_MESSAGE_MAX_LEN] {
        &mut self.data
    }

    /// Returns the message length.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Sets the valid data length.
    ///
    /// # Panics
    ///
    /// Panics if `len > TARGET_MESSAGE_MAX_LEN`.
    pub fn set_len(&mut self, len: usize) {
        assert!(len <= TARGET_MESSAGE_MAX_LEN);
        self.len = len as u8;
    }

    /// Returns `true` if the message is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Clears the message data.
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl Default for TargetMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for TargetMessage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TargetMessage")
            .field("source_address", &self.source_address)
            .field("len", &self.len)
            .field("data", &self.data())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_message_new() {
        let msg = TargetMessage::new();
        assert!(msg.is_empty());
        assert_eq!(msg.len(), 0);
        assert_eq!(msg.data(), &[]);
    }

    #[test]
    fn test_target_message_from_data() {
        let source = I2cAddress::new(0x50).unwrap();
        let data = [0x01, 0x02, 0x03, 0x04];
        let msg = TargetMessage::from_data(source, &data);
        
        assert_eq!(msg.source_address, source);
        assert_eq!(msg.len(), 4);
        assert_eq!(msg.data(), &data);
    }

    #[test]
    fn test_target_message_set_len() {
        let mut msg = TargetMessage::new();
        msg.data_mut()[..3].copy_from_slice(&[0xAA, 0xBB, 0xCC]);
        msg.set_len(3);
        
        assert_eq!(msg.len(), 3);
        assert_eq!(msg.data(), &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_target_message_clear() {
        let source = I2cAddress::new(0x50).unwrap();
        let mut msg = TargetMessage::from_data(source, &[0x01, 0x02]);
        
        msg.clear();
        assert!(msg.is_empty());
    }

    #[test]
    fn test_target_message_default() {
        let msg = TargetMessage::default();
        assert!(msg.is_empty());
    }
}
