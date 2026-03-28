//! USB peripheral driver traits and events.
//!
//! This module defines the interface between the USB protocol stack and the
//! hardware-specific peripheral driver.

use aligned::Aligned;
use aligned::A4;
use core::mem::MaybeUninit;

use crate::SetupPacket;

/// A trait implemented by drivers for USB peripheral controllers.
pub trait UsbDriver {
    /// The maximum packet size supported by the hardware (typically 64 bytes).
    const MAX_PACKET_SIZE: usize;
    /// The type of packet returned by the driver.
    type Packet<'a>: UsbPacket
    where
        Self: 'a;

    /// Store data in a peripheral buffer that will be transferred to the host
    /// when it requests data from the IN endpoint at `endpoint_idx`.
    ///
    /// If `zlp` is true and `data.len()` is a multiple of `MAX_PACKET_SIZE`,
    /// the driver should send a zero-length packet (ZLP) after all data has
    /// been acknowledged.
    ///
    /// The return value is the number of bytes that were copied into the
    /// peripheral buffer. It will be either a multiple of `MAX_PACKET_SIZE`,
    /// or `data.len()`.
    ///
    /// This function may fault or panic if `endpoint_idx` is invalid, or if the
    /// hardware is in an invalid state.
    fn transfer_in(&mut self, endpoint_idx: u8, data: &Aligned<A4, [u8]>, zlp: bool) -> usize;

    /// Store data in a peripheral buffer that will be transferred to the host.
    ///
    /// This version accepts an unaligned data buffer.
    fn transfer_in_unaligned(&mut self, endpoint_idx: u8, data: &[u8], zlp: bool) -> usize;

    /// Stalls or unstalls an endpoint.
    ///
    /// Note: the driver will automatically unstall all endpoints upon a USB
    /// reset or upon receiving a new SETUP packet on Endpoint 0.
    fn stall(&mut self, endpoint_num: u8, stalled: bool);

    /// Returns whether the specified endpoint is currently stalled.
    fn is_stalled(&mut self, endpoint_num: u8) -> bool;

    /// Sets the address the peripheral responds to.
    ///
    /// The USB stack must call this function in response to a `SET_ADDRESS`
    /// control request on Endpoint 0.
    fn set_address(&mut self, address: u8);

    /// Polls the driver for a USB event.
    ///
    /// When a USB interrupt occurs, the USB stack should call this function
    /// repeatedly until it returns `None`.
    fn poll(&mut self) -> Option<UsbEvent<Self::Packet<'_>>>;
}

/// A trait representing a received USB packet.
pub trait UsbPacket {
    /// Returns the index of the endpoint the packet was received on.
    fn endpoint_index(&self) -> usize;

    /// Returns the length of the packet data in bytes.
    fn len(&self) -> usize;

    /// Copies the packet data from the peripheral buffer into system memory.
    ///
    /// This method allows copying into uninitialized memory.
    /// Will fault if `self.len() > dest.len() * 4`.
    fn copy_to_uninit(self, dest: &mut [MaybeUninit<u32>]) -> &[u8];

    /// Copies the packet data from the peripheral buffer into system memory.
    ///
    /// Will fault if `self.len() > dest.len() * 4`.
    fn copy_to(self, dest: &mut [u32]) -> &[u8];

    /// Copies the packet data from the peripheral buffer into system memory.
    ///
    /// This version accepts an unaligned destination buffer.
    fn copy_to_unaligned(self, dest: &mut [u8]) -> &[u8];

    /// Returns `true` if the packet is empty (zero-length).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Events that can be reported by a USB driver.
pub enum UsbEvent<TPacket: UsbPacket> {
    /// A SETUP packet has been received from the host.
    SetupPacket {
        /// The decoded SETUP packet.
        pkt: SetupPacket,
        /// The endpoint index (always 0 for standard SETUP packets).
        endpoint: u8,
    },

    /// An OUT packet has been received from the host.
    DataOutPacket(TPacket),

    /// A packet has been sent by the peripheral and acknowledged by the host.
    ///
    /// This indicates that buffer space is now available on the specified
    /// endpoint for further `transfer_in` calls.
    PacketSent {
        /// The index of the endpoint that sent the packet.
        endpoint: u32,
    },

    /// VBus presence detected.
    VBus,
    /// VBus presence lost.
    VBusLost,
    /// USB link is down.
    LinkDown,
    /// USB link is up.
    LinkUp,
    /// USB bus reset received.
    UsbReset,
    /// USB bus suspend received.
    Suspend,
    /// USB bus resume received.
    Resume,

    /// Unexpected buffer ID error.
    ErrorUnexpectedBufId,
}
