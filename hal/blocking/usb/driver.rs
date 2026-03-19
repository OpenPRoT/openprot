use aligned::Aligned;
use aligned::A4;
use core::mem::MaybeUninit;

use crate::SetupPacket;

/// A trait implemented by drivers for USB peripheral controllers.
pub trait UsbDriver {
    const MAX_PACKET_SIZE: usize;
    type Packet<'a>: UsbPacket
    where
        Self: 'a;

    /// Store data in a peripheral buffer that will be transferred to the host
    /// when it requests data from the IN endpoint at `endpoint_idx`. If
    /// zlp=true and `data.len()` is a multiple of MAX_PACKET_SIZE,
    /// send a zero-length packet after sending all the data.
    ///
    /// The return value is the number of bytes that were copied into the
    /// peripheral buffer. It will be either a multiple of MAX_PACKET_SIZE, or `data.len()`.
    ///
    /// This function may fault or panic if endpoint_idx is invalid, or the hardware is misbehaving.
    fn transfer_in(&mut self, endpoint_idx: u8, data: &Aligned<A4, [u8]>, zlp: bool) -> usize;

    /// Stalls an input endpoint. Note: the driver will automatically unstall all endpoints upon a USB reset or a new SETUP packet.
    fn stall_in(&mut self, endpoint_idx: u8, stalled: bool);

    /// Stalls an output endpoint. Note: the driver will automatically unstall all endpoints upon a USB reset or a new SETUP packet.
    fn stall_out(&mut self, endpoint_idx: u8, stalled: bool);

    /// Sets the address the peripheral responds to. The USB stack must call
    /// this function in response to a SET_ADDRESS control request on endpoint 0.
    fn set_address(&mut self, address: u8);

    /// Polls the driver for an event. When a USB interrupt occurs, the USB
    /// stack should call this function repeatedly until it returns None and
    /// process the returned events.
    fn poll(&mut self) -> Option<UsbEvent<Self::Packet<'_>>>;
}

pub trait UsbPacket {
    /// The endpoint the packet was received on.
    fn endpoint_index(&self) -> usize;

    /// The length of the packet in bytes
    fn len(&self) -> usize;

    /// Copy the packet data from the peripheral buffer into SRAM. Will fault if
    /// `self.len()` > `dest.len()`.
    fn copy_to_uninit(self, dest: &mut [MaybeUninit<u32>]) -> &Aligned<A4, [u8]>;

    /// Copy the packet data from the peripheral buffer into SRAM.
    fn copy_to(self, dest: &mut [u32]) -> &Aligned<A4, [u8]>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub enum UsbEvent<TPacket: UsbPacket> {
    /// A SETUP packet has been received from the host. It can be read with TPacket::copy_to()...
    SetupPacket {
        pkt: SetupPacket,
        endpoint: u8,
    },

    /// An OUT packet has been received from the host. It can be read with TPacket::copy_to()...
    DataOutPacket(TPacket),

    /// A packet has been sent by the peripheral and an ACK has been received
    /// from the host. This will have freed up some buffer space, so if the USB
    /// stack has more data to send on this endpoint, it should attempt to
    /// buffer it now with `UsbDriver::transfer_in()`.
    PacketSent {
        endpoint: u32,
    },

    VBus,
    VBusLost,
    LinkDown,
    LinkUp,
    UsbReset,
    Suspend,
    Resume,

    // TODO: Put these into the global error namespace...
    ErrorUnexpectedBufId,
}
