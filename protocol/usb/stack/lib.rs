// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Generic USB protocol stack.
//!
//! This module provides the core logic for a USB device stack, including
//! Endpoint 0 control request handling, descriptor management, and
//! multi-packet transfer accumulation.

#![no_std]

use aligned::Aligned;
use aligned::A4;
use core::mem::size_of;
use hal_usb::driver::UsbDriver;
use hal_usb::driver::UsbEvent;
use hal_usb::driver::UsbPacket;
use hal_usb::DescriptorInfo;
use hal_usb::DescriptorType;
use hal_usb::Request;
use hal_usb::SetupPacket;
use hal_usb::StringDescriptorRef;
use hal_usb::StringHandle;
use zerocopy::IntoBytes;

use pw_status::Error;

pub const CONFIG_0: Aligned<A4, [u8; 1]> = Aligned([0]);
pub const CONFIG_1: Aligned<A4, [u8; 1]> = Aligned([1]);
pub const STATUS_OK: Aligned<A4, [u8; 2]> = Aligned([0, 0]);
pub const STATUS_HALTED: Aligned<A4, [u8; 2]> = Aligned([1, 0]);

#[inline(always)]
const fn endpoint_number(endpoint_addr: u8) -> u8 {
    endpoint_addr & 0x0f
}

#[inline(always)]
const fn endpoint_is_in(endpoint_addr: u8) -> bool {
    (endpoint_addr & 0x80) != 0
}

#[inline(always)]
const fn endpoint_bit_index(endpoint_addr: u8) -> u32 {
    let num = endpoint_number(endpoint_addr) as u32;
    if endpoint_is_in(endpoint_addr) {
        16 + num
    } else {
        num
    }
}

/// A trait for providing USB descriptors to the stack.
///
/// Applications must implement this trait to define the device's identity
/// and capabilities.
pub trait DescriptorSource {
    /// Device descriptor bytes.
    const DEVICE_DESC_BYTES: &'static Aligned<A4, [u8]>;
    /// Configuration descriptor bytes (including interfaces and endpoints).
    const CONFIG_DESC_BYTES: &'static Aligned<A4, [u8]>;
    /// String descriptor 0 bytes (supported languages).
    const STRING_DESC_0_BYTES: &'static Aligned<A4, [u8]>;
    /// Device status bytes (2 bytes, usually [0, 0]).
    const DEVICE_STATUS: Aligned<A4, [u8; 2]>;

    /// Returns a string descriptor by handle and language ID.
    fn get_string(&self, handle: StringHandle, lang: u16) -> Option<StringDescriptorRef<'_>>;
    /// Returns the device status bytes.
    fn get_device_status(&self) -> &Aligned<A4, [u8]> {
        &Self::DEVICE_STATUS
    }
}

/// An empty aligned buffer.
pub const EMPTY: &Aligned<A4, [u8]> = &Aligned([]);

/// A simple implementation of USB Endpoint 0 (control endpoint).
pub struct SimpleEp0 {
    new_address: Option<u8>,
    configuration: u8,
    halted_endpoints: u32,
    pending_halt: Option<(u8, bool)>,
}

/// A trait for modular USB class implementations.
pub trait UsbClass {
    /// Attempt to handle a USB event.
    ///
    /// If the event is handled by this class, it returns `Ok(UsbAction)`.
    /// Otherwise, it returns the original event in `Err`.
    fn handle_event<'a, P: UsbPacket>(
        &'a mut self,
        event: UsbEvent<P>,
    ) -> Result<UsbAction<'a>, UsbEvent<P>>;
}

/// Indicates the result of running a USB action.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum UsbActionRun {
    /// No operation was performed.
    NoOp,
    /// The action has more data to transfer.
    HasMoreData,
    /// The action is complete.
    Done,
}

/// Actions to be performed on a USB driver.
pub enum UsbAction<'a> {
    /// No action.
    None,
    /// Perform an IN transfer on the specified endpoint.
    TransferIn {
        /// The endpoint index.
        endpoint: u8,
        /// The data to transfer.
        data: &'a Aligned<A4, [u8]>,
        /// If true, send a zero-length packet (ZLP) if the data length
        /// is a multiple of the maximum packet size.
        zlp: bool,
    },
    /// Perform an IN transfer on the specified endpoint using unaligned data.
    TransferInUnaligned {
        /// The endpoint index.
        endpoint: u8,
        /// The data to transfer.
        data: &'a [u8],
        /// If true, send a zero-length packet (ZLP) if the data length
        /// is a multiple of the maximum packet size.
        zlp: bool,
    },
    /// Stall both IN and OUT directions on the specified endpoint.
    StallInAndOut {
        /// The endpoint index.
        endpoint: u8,
    },
    /// Set the device address.
    SetAddress {
        /// The new device address.
        new_address: u8,
    },
    /// Set the stall status of an endpoint.
    EndpointHalt {
        /// The endpoint address.
        endpoint_addr: u8,
        /// Whether to stall or unstall the endpoint.
        halted: bool,
    },
}

impl PartialEq for UsbAction<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (
                Self::TransferIn {
                    endpoint: e1,
                    data: d1,
                    zlp: z1,
                },
                Self::TransferIn {
                    endpoint: e2,
                    data: d2,
                    zlp: z2,
                },
            ) => e1 == e2 && core::ptr::eq(*d1, *d2) && z1 == z2,
            (
                Self::TransferInUnaligned {
                    endpoint: e1,
                    data: d1,
                    zlp: z1,
                },
                Self::TransferInUnaligned {
                    endpoint: e2,
                    data: d2,
                    zlp: z2,
                },
            ) => e1 == e2 && core::ptr::eq(*d1, *d2) && z1 == z2,
            (Self::StallInAndOut { endpoint: e1 }, Self::StallInAndOut { endpoint: e2 }) => {
                e1 == e2
            }
            (Self::SetAddress { new_address: a1 }, Self::SetAddress { new_address: a2 }) => {
                a1 == a2
            }
            (
                Self::EndpointHalt {
                    endpoint_addr: a1,
                    halted: h1,
                },
                Self::EndpointHalt {
                    endpoint_addr: a2,
                    halted: h2,
                },
            ) => a1 == a2 && h1 == h2,
            _ => false,
        }
    }
}
impl Eq for UsbAction<'_> {}

impl<'a> UsbAction<'a> {
    /// Helper to create a TransferIn action for a control transfer,
    /// or a StallInAndOut if the requested length is too small.
    #[inline(always)]
    #[track_caller]
    pub fn control_transfer_in_or_stall(
        endpoint: u8,
        pkt: &SetupPacket,
        data: &'a Aligned<A4, [u8]>,
    ) -> Self {
        if data.len() > pkt.length().into() {
            Self::StallInAndOut { endpoint }
        } else {
            Self::TransferIn {
                endpoint,
                data,
                // Per USB Specs 5.5.3, we need to send ZLP for control transfers
                // if the response is less than requested.
                zlp: data.is_empty() || data.len() < pkt.length().into(),
            }
        }
    }
    /// Merges another action into this one.
    pub fn merge(&mut self, new_action: UsbAction<'a>) {
        match new_action {
            UsbAction::None => {}
            _ => *self = new_action,
        }
    }

    /// Executes the action on the provided driver.
    pub fn run<TDriver: UsbDriver>(&mut self, driver: &mut TDriver) -> UsbActionRun {
        match self {
            Self::None => return UsbActionRun::NoOp,
            Self::TransferIn {
                endpoint,
                data,
                zlp,
            } => {
                let bytes_transferred = driver.transfer_in(*endpoint, data, *zlp);
                // Note: bytes_transferred is guaranteed to be a multiple of
                // UsbDriver::MAX_PACKET_SIZE, which is guaranteed to be a
                // multiple of 4.
                if bytes_transferred < data.len() && (bytes_transferred & 3) == 0 {
                    // We're not done yet...
                    *data = &data[bytes_transferred..];
                    return UsbActionRun::HasMoreData;
                }
            }
            Self::TransferInUnaligned {
                endpoint,
                data,
                zlp,
            } => {
                let bytes_transferred = driver.transfer_in_unaligned(*endpoint, data, *zlp);
                if bytes_transferred < data.len() {
                    // We're not done yet...
                    *data = &data[bytes_transferred..];
                    return UsbActionRun::HasMoreData;
                }
            }
            Self::SetAddress { new_address } => driver.set_address(*new_address),
            Self::StallInAndOut { endpoint } => {
                driver.stall((*endpoint) & 0x7f, true);
                driver.stall((*endpoint) | 0x80, true);
            }
            Self::EndpointHalt {
                endpoint_addr,
                halted,
            } => {
                driver.stall(*endpoint_addr, *halted);
            }
        }
        *self = UsbAction::None;
        UsbActionRun::Done
    }
}

impl SimpleEp0 {
    /// Creates a new `SimpleEp0` handler.
    pub fn new() -> Self {
        Self {
            new_address: None,
            configuration: 0,
            halted_endpoints: 0,
            pending_halt: None,
        }
    }

    pub fn is_endpoint_halted(&self, endpoint_addr: u8) -> bool {
        let idx = endpoint_bit_index(endpoint_addr);
        (self.halted_endpoints & (1 << idx)) != 0
    }

    pub fn set_endpoint_halted(&mut self, endpoint_addr: u8, halted: bool) {
        let idx = endpoint_bit_index(endpoint_addr);
        if halted {
            self.halted_endpoints |= 1 << idx;
        } else {
            self.halted_endpoints &= !(1 << idx);
        }
    }

    /// A helper function to process a driver UsbEvent.
    ///
    /// This function returns the action that should be performed on the driver.
    pub fn handle_event<'a, P: UsbPacket>(
        &mut self,
        ev: UsbEvent<P>,
        descriptor_source: &'a impl DescriptorSource,
    ) -> Result<UsbAction<'a>, UsbEvent<P>> {
        match ev {
            UsbEvent::SetupPacket { endpoint: 0, pkt } => {
                use hal_usb::RequestType;
                if pkt.request().request_type() == RequestType::Standard {
                    Ok(self.handle_setup(pkt, descriptor_source))
                } else {
                    Err(ev)
                }
            }
            UsbEvent::PacketSent { endpoint: 0 } => Ok(self.handle_packet_sent()),
            UsbEvent::UsbReset => {
                self.configuration = 0;
                self.new_address = None;
                self.halted_endpoints = 0;
                self.pending_halt = None;
                Ok(UsbAction::None)
            }
            _ => Err(ev),
        }
    }

    /// Process a SETUP transfer and return the resulting action.
    fn handle_setup<'a, TDescriptorSource: DescriptorSource>(
        &mut self,
        setup_pkt: SetupPacket,
        descriptor_source: &'a TDescriptorSource,
    ) -> UsbAction<'a> {
        match setup_pkt.request() {
            Request::DEVICE_GET_DESCRIPTOR => {
                let descriptor = DescriptorInfo::from(&setup_pkt);
                #[rustfmt::skip]
                let mut response: Option<&Aligned<A4, [u8]>> = match descriptor {
                    DescriptorInfo { ty: DescriptorType::DEVICE, index: 0, .. } => {
                        Some(TDescriptorSource::DEVICE_DESC_BYTES)
                    }
                    DescriptorInfo { ty: DescriptorType::CONFIGURATION, index: 0, .. } => {
                        Some(TDescriptorSource::CONFIG_DESC_BYTES)
                    }
                    DescriptorInfo { ty: DescriptorType::STRING, index: 0, .. } => {
                        Some(TDescriptorSource::STRING_DESC_0_BYTES)
                    }
                    DescriptorInfo { ty: DescriptorType::STRING, index, .. } => {
                        descriptor_source
                            .get_string(StringHandle(index), setup_pkt.index())
                            .map(|desc| desc.as_bytes())
                    }
                    _ => None,
                };
                if let Some(response) = &mut response {
                    if response.len() > setup_pkt.length().into() {
                        *response = &(*response)[..setup_pkt.length().into()];
                    }
                    UsbAction::control_transfer_in_or_stall(0, &setup_pkt, response)
                } else {
                    UsbAction::StallInAndOut { endpoint: 0 }
                }
            }
            Request::DEVICE_GET_STATUS => UsbAction::control_transfer_in_or_stall(
                0,
                &setup_pkt,
                descriptor_source.get_device_status(),
            ),
            Request::INTERFACE_GET_STATUS => {
                UsbAction::control_transfer_in_or_stall(0, &setup_pkt, &STATUS_OK)
            }
            Request::ENDPOINT_GET_STATUS => {
                let ep_addr = u8::try_from(setup_pkt.index() & 0xff).unwrap();
                let data = if self.is_endpoint_halted(ep_addr) {
                    &STATUS_HALTED
                } else {
                    &STATUS_OK
                };
                UsbAction::control_transfer_in_or_stall(0, &setup_pkt, data)
            }
            Request::ENDPOINT_SET_FEATURE => {
                match setup_pkt.value() {
                    0 => {
                        // ENDPOINT_HALT
                        let ep_addr = u8::try_from(setup_pkt.index() & 0xff).unwrap();
                        if endpoint_number(ep_addr) == 0 {
                            // Endpoint 0 cannot be halted. Stall the control pipe to reject the request.
                            UsbAction::StallInAndOut { endpoint: 0 }
                        } else {
                            self.set_endpoint_halted(ep_addr, true);
                            self.pending_halt = Some((ep_addr, true));
                            UsbAction::TransferIn {
                                endpoint: 0,
                                data: EMPTY,
                                zlp: true,
                            }
                        }
                    }
                    _ => {
                        // Feature not supported.
                        UsbAction::StallInAndOut { endpoint: 0 }
                    }
                }
            }
            Request::ENDPOINT_CLEAR_FEATURE => {
                match setup_pkt.value() {
                    0 => {
                        // ENDPOINT_HALT
                        let ep_addr = u8::try_from(setup_pkt.index() & 0xff).unwrap();
                        if endpoint_number(ep_addr) == 0 {
                            // Endpoint 0 cannot be cleared. Stall the control pipe to reject the request.
                            UsbAction::StallInAndOut { endpoint: 0 }
                        } else {
                            self.set_endpoint_halted(ep_addr, false);
                            self.pending_halt = Some((ep_addr, false));
                            UsbAction::TransferIn {
                                endpoint: 0,
                                data: EMPTY,
                                zlp: true,
                            }
                        }
                    }
                    _ => {
                        // Feature not supported.
                        UsbAction::StallInAndOut { endpoint: 0 }
                    }
                }
            }
            Request::DEVICE_SET_ADDRESS => {
                self.new_address = Some(setup_pkt.value() as u8);
                UsbAction::TransferIn {
                    endpoint: 0,
                    data: EMPTY,
                    zlp: true,
                }
            }
            Request::DEVICE_SET_CONFIGURATION => {
                let val = setup_pkt.value();
                if val == 0 || val == 1 {
                    self.configuration = val as u8;
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data: EMPTY,
                        zlp: true,
                    }
                } else {
                    UsbAction::StallInAndOut { endpoint: 0 }
                }
            }
            Request::DEVICE_GET_CONFIGURATION => {
                let data = match self.configuration {
                    1 => &CONFIG_1,
                    _ => &CONFIG_0,
                };
                UsbAction::control_transfer_in_or_stall(0, &setup_pkt, data)
            }
            Request::INTERFACE_GET_INTERFACE => {
                UsbAction::control_transfer_in_or_stall(0, &setup_pkt, &CONFIG_0)
            }
            Request::INTERFACE_SET_INTERFACE => {
                if setup_pkt.value() == 0 {
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data: EMPTY,
                        zlp: true,
                    }
                } else {
                    UsbAction::StallInAndOut { endpoint: 0 }
                }
            }
            _ => UsbAction::StallInAndOut { endpoint: 0 },
        }
    }
    fn handle_packet_sent(&mut self) -> UsbAction<'static> {
        if let Some(new_address) = self.new_address.take() {
            // Now that the transfer is complete it's safe to change the address..
            return UsbAction::SetAddress { new_address };
        }
        if let Some((endpoint_addr, halted)) = self.pending_halt.take() {
            return UsbAction::EndpointHalt {
                endpoint_addr,
                halted,
            };
        }
        UsbAction::None
    }
}

impl Default for SimpleEp0 {
    fn default() -> Self {
        Self::new()
    }
}

/// A helper struct to handle multi-packet USB transfers.
///
/// It accumulates incoming USB packets into an internal buffer until a short
/// packet or a zero-length packet (ZLP) is received, indicating the end of a transfer.
///
/// `N` is the number of **words** (`u32`s) in the internal buffer and NOT bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct Transfer<const N: usize> {
    buffer: [u32; N],
    word_offset: usize,
}

impl<const N: usize> Transfer<N> {
    /// Maximum packet size supported (fixed at 64 bytes).
    pub const MAX_PACKET_SIZE: usize = 64;

    /// Creates a new `Transfer` buffer.
    pub fn new() -> Self {
        Self {
            buffer: [0; N],
            word_offset: 0,
        }
    }

    /// Splices a USB packet into the buffer.
    ///
    /// Returns `Ok(Some(slice))` if the transfer is complete, `Ok(None)` otherwise.
    pub fn splice(&mut self, packet: impl UsbPacket) -> Result<Option<&Aligned<A4, [u8]>>, Error> {
        const {
            assert!(Self::MAX_PACKET_SIZE % size_of::<u32>() == 0);
        }
        let packet_len = packet.len();
        let dest = {
            let start = self.word_offset;
            let end = start + packet_len.div_ceil(size_of::<u32>());
            self.buffer.get_mut(start..end).ok_or(Error::OutOfRange)?
        };
        packet.copy_to(dest);
        if packet_len < Self::MAX_PACKET_SIZE {
            let result = &self
                .buffer
                .as_bytes()
                .get(..self.word_offset * size_of::<u32>() + packet_len)
                .ok_or(Error::OutOfRange)?;
            self.word_offset = 0;
            Ok(Some(
                // nosemgrep
                unsafe {
                    // SAFETY: `self.buffer` is `[u32]` which has alignment of 4.
                    core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(result)
                },
            ))
        } else {
            self.word_offset += Self::MAX_PACKET_SIZE / size_of::<u32>();
            Ok(None)
        }
    }
}

impl<const N: usize> Default for Transfer<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub mod testing {
    use aligned::Aligned;
    use aligned::A4;
    use hal_usb::driver::UsbPacket;
    use zerocopy::IntoBytes;

    #[derive(Debug)]
    pub struct FakeUsbPacket<'a> {
        pub data: &'a [u8],
        pub ep: usize,
    }

    impl UsbPacket for FakeUsbPacket<'_> {
        fn endpoint_index(&self) -> usize {
            self.ep
        }

        fn len(&self) -> usize {
            self.data.len()
        }

        fn copy_to_uninit(self, _dest: &mut [core::mem::MaybeUninit<u32>]) -> &[u8] {
            unimplemented!()
        }

        fn copy_to(self, dest: &mut [u32]) -> &[u8] {
            let dest_bytes = dest.as_mut_bytes();
            let copy_len = self.data.len().min(dest_bytes.len());
            dest_bytes[..copy_len].copy_from_slice(&self.data[..copy_len]);
            // nosemgrep
            unsafe {
                // SAFETY: `dest` is a `&mut [u32]`, which is guaranteed to be 4-byte
                // aligned. `dest_bytes` is a byte slice view of the same memory, so it's also
                // 4-byte aligned. The subslice `&dest_bytes[..copy_len]` maintains this alignment.
                core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(&dest_bytes[..copy_len])
            }
        }

        fn copy_to_unaligned(self, dest: &mut [u8]) -> &[u8] {
            let copy_len = self.data.len().min(dest.len());
            dest[..copy_len].copy_from_slice(&self.data[..copy_len]);
            &dest[..copy_len]
        }
    }
}

#[cfg(test)]
mod splice_tests {
    use super::testing::FakeUsbPacket;
    use super::*;

    const MAX_PACKET_SIZE: usize = Transfer::<0>::MAX_PACKET_SIZE;

    #[test]
    fn test_splice_single_short_packet() {
        let packet_data = [1, 2, 3, 4];
        let packet = FakeUsbPacket {
            data: &packet_data,
            ep: 0,
        };

        let mut transfer = Transfer::<32>::new();
        let result = transfer.splice(packet).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), &packet_data[..]);
    }

    #[test]
    fn test_splice_single_full_packet_then_zlp() {
        let packet_data = [42; MAX_PACKET_SIZE];
        let packet = FakeUsbPacket {
            data: &packet_data,
            ep: 0,
        };

        let mut transfer = Transfer::<32>::new();
        let result = transfer.splice(packet).unwrap();

        assert!(result.is_none());

        let zlp = FakeUsbPacket { data: &[], ep: 0 };
        let result = transfer.splice(zlp).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), &packet_data[..]);
    }

    #[test]
    fn test_splice_multiple_packets() {
        let packet1_data = [1; MAX_PACKET_SIZE];
        let packet2_data = [2; MAX_PACKET_SIZE];
        let packet3_data = [3; 32];

        // Packet 1
        let packet1 = FakeUsbPacket {
            data: &packet1_data,
            ep: 0,
        };
        let mut transfer = Transfer::<64>::new();
        let result = transfer.splice(packet1).unwrap();
        assert!(result.is_none());

        // Packet 2
        let packet2 = FakeUsbPacket {
            data: &packet2_data,
            ep: 0,
        };
        let result = transfer.splice(packet2).unwrap();
        assert!(result.is_none());

        // Packet 3 (short packet)
        let packet3 = FakeUsbPacket {
            data: &packet3_data,
            ep: 0,
        };
        let result = transfer.splice(packet3).unwrap();
        assert!(result.is_some());

        let mut expected_data = [0u8; 2 * MAX_PACKET_SIZE + 32];
        expected_data[..MAX_PACKET_SIZE].copy_from_slice(&packet1_data);
        expected_data[MAX_PACKET_SIZE..2 * MAX_PACKET_SIZE].copy_from_slice(&packet2_data);
        expected_data[2 * MAX_PACKET_SIZE..].copy_from_slice(&packet3_data);

        assert_eq!(result.unwrap().as_ref(), &expected_data[..]);
    }

    #[test]
    fn test_splice_buffer_overflow() {
        let packet_data = [1; 1];
        let packet = FakeUsbPacket {
            data: &packet_data,
            ep: 0,
        };

        let mut transfer = Transfer::<16>::new();
        transfer
            .splice(FakeUsbPacket {
                data: &[0; MAX_PACKET_SIZE],
                ep: 0,
            })
            .unwrap();
        let result = transfer.splice(packet);
        assert_eq!(result.err(), Some(Error::OutOfRange));
    }

    #[test]
    fn test_full_capacity_with_full_packets_then_partial_packet() {
        const PARTIAL_SIZE: usize = 16;
        const FULL1_DATA: &[u8] = &[0xaa; MAX_PACKET_SIZE];
        const FULL2_DATA: &[u8] = &[0xbb; MAX_PACKET_SIZE];
        const PARTIAL_DATA: &[u8] = &[0xcc; PARTIAL_SIZE];
        const RECEIVE_BUFFER_WORDS: usize =
            (FULL1_DATA.len() + FULL2_DATA.len() + PARTIAL_DATA.len()) / size_of::<u32>();
        let full1 = FakeUsbPacket {
            data: FULL1_DATA,
            ep: 0,
        };
        let full2 = FakeUsbPacket {
            data: FULL2_DATA,
            ep: 0,
        };
        let partial = FakeUsbPacket {
            data: PARTIAL_DATA,
            ep: 0,
        };
        let mut transfer = Transfer::<RECEIVE_BUFFER_WORDS>::new();
        assert!(transfer.splice(full1).unwrap().is_none());
        assert!(transfer.splice(full2).unwrap().is_none());
        let buffer = transfer.splice(partial).unwrap().unwrap().as_ref();
        assert_eq!(&buffer[..MAX_PACKET_SIZE], FULL1_DATA);
        assert_eq!(&buffer[MAX_PACKET_SIZE..2 * MAX_PACKET_SIZE], FULL2_DATA);
    }
}

#[cfg(test)]
mod simple_ep0_tests {
    use super::*;
    use aligned::Aligned;
    use aligned::A4;
    use hal_usb::Request;
    use hal_usb::SetupPacket;
    use hal_usb::StringDescriptorRef;
    use hal_usb::StringHandle;

    struct DummyDescriptors;
    impl DescriptorSource for DummyDescriptors {
        const DEVICE_DESC_BYTES: &'static Aligned<A4, [u8]> = &Aligned([]);
        const CONFIG_DESC_BYTES: &'static Aligned<A4, [u8]> = &Aligned([]);
        const STRING_DESC_0_BYTES: &'static Aligned<A4, [u8]> = &Aligned([]);
        const DEVICE_STATUS: Aligned<A4, [u8; 2]> = Aligned([0, 0]);

        fn get_string(&self, _handle: StringHandle, _lang: u16) -> Option<StringDescriptorRef<'_>> {
            None
        }
    }

    fn unwrap_action<'a, P: UsbPacket>(res: Result<UsbAction<'a>, UsbEvent<P>>) -> UsbAction<'a> {
        match res {
            Ok(a) => a,
            Err(_) => panic!("Expected Ok action"),
        }
    }

    #[test]
    fn test_set_configuration() {
        let mut ep0 = SimpleEp0::new();
        let descriptors = DummyDescriptors;

        // Value = 0 (de-configure/address state) should be accepted
        let req = Request::DEVICE_SET_CONFIGURATION;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0]);
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(
            action,
            UsbAction::TransferIn {
                endpoint: 0,
                zlp: true,
                ..
            }
        ));

        // Value = 1 (active configuration) should be accepted
        let buf0 = (u16::from(req) as u32) | (1u32 << 16);
        let setup_pkt = SetupPacket::new([buf0, 0]);
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(
            action,
            UsbAction::TransferIn {
                endpoint: 0,
                zlp: true,
                ..
            }
        ));

        // Value = 2 (invalid configuration) should be stalled
        let buf0 = (u16::from(req) as u32) | (2u32 << 16);
        let setup_pkt = SetupPacket::new([buf0, 0]);
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(action, UsbAction::StallInAndOut { endpoint: 0 }));
    }

    #[test]
    fn test_get_configuration() {
        let mut ep0 = SimpleEp0::new();
        let descriptors = DummyDescriptors;

        // Initial state should be unconfigured (0)
        let req = Request::DEVICE_GET_CONFIGURATION;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0001_0000]); // Length = 1
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0]);
        } else {
            panic!("Expected TransferIn action");
        }

        // Set configuration to 1
        let req_set = Request::DEVICE_SET_CONFIGURATION;
        let buf0 = (u16::from(req_set) as u32) | (1u32 << 16);
        let setup_pkt = SetupPacket::new([buf0, 0]);
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let _ = ep0.handle_event(ev, &descriptors);

        // Now GET_CONFIGURATION should return 1
        let req_get = Request::DEVICE_GET_CONFIGURATION;
        let buf0 = u16::from(req_get) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0001_0000]); // Length = 1
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[1]);
        } else {
            panic!("Expected TransferIn action");
        }

        // USB reset should reset configuration back to 0
        let ev_reset: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::UsbReset;
        let _ = ep0.handle_event(ev_reset, &descriptors);

        // Now GET_CONFIGURATION should return 0
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0]);
        } else {
            panic!("Expected TransferIn action");
        }
    }

    #[test]
    fn test_get_status() {
        let mut ep0 = SimpleEp0::new();
        let descriptors = DummyDescriptors;

        // Device recipient
        let req = Request::DEVICE_GET_STATUS;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0002_0000]); // Length = 2
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0, 0]);
        } else {
            panic!("Expected TransferIn action");
        }

        // Interface recipient
        let req = Request::INTERFACE_GET_STATUS;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0002_0000]); // Length = 2
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0, 0]);
        } else {
            panic!("Expected TransferIn action");
        }

        // Endpoint recipient
        let req = Request::ENDPOINT_GET_STATUS;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0002_0000]); // Length = 2
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0, 0]);
        } else {
            panic!("Expected TransferIn action");
        }
    }

    #[test]
    fn test_fallback_interface_requests() {
        let mut ep0 = SimpleEp0::new();
        let descriptors = DummyDescriptors;

        // GET_INTERFACE should return alternate setting 0
        let req = Request::INTERFACE_GET_INTERFACE;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0001_0000]); // Length = 1
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0]);
        } else {
            panic!("Expected TransferIn action");
        }

        // SET_INTERFACE with alternate setting 0 should succeed
        let req = Request::INTERFACE_SET_INTERFACE;
        let buf0 = u16::from(req) as u32; // Value = 0
        let setup_pkt = SetupPacket::new([buf0, 0]);
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(
            action,
            UsbAction::TransferIn {
                endpoint: 0,
                zlp: true,
                ..
            }
        ));

        // SET_INTERFACE with alternate setting 1 should be stalled (only 0 supported as fallback)
        let buf0 = (u16::from(req) as u32) | (1u32 << 16); // Value = 1
        let setup_pkt = SetupPacket::new([buf0, 0]);
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(action, UsbAction::StallInAndOut { endpoint: 0 }));
    }

    #[test]
    fn test_endpoint_halt_requests() {
        let mut ep0 = SimpleEp0::new();
        let descriptors = DummyDescriptors;

        // 1. Check Endpoint 1 IN status (initially not halted)
        let req = Request::ENDPOINT_GET_STATUS;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0002_0081]); // Length = 2, Endpoint = 0x81 (1 IN)
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0, 0]);
        } else {
            panic!("Expected TransferIn action");
        }

        // 2. SET_FEATURE(ENDPOINT_HALT) to Endpoint 1 IN
        let req = Request::ENDPOINT_SET_FEATURE;
        let buf0 = u16::from(req) as u32; // Value = 0 (ENDPOINT_HALT)
        let setup_pkt = SetupPacket::new([buf0, 0x0000_0081]); // Endpoint = 0x81
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(
            action,
            UsbAction::TransferIn {
                endpoint: 0,
                zlp: true,
                ..
            }
        ));

        // Verify that handle_packet_sent returns EndpointHalt action
        let action_sent = ep0.handle_packet_sent();
        assert!(matches!(
            action_sent,
            UsbAction::EndpointHalt {
                endpoint_addr: 0x81,
                halted: true
            }
        ));

        // 3. Check Endpoint 1 IN status again (should be halted)
        let req = Request::ENDPOINT_GET_STATUS;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0002_0081]); // Length = 2, Endpoint = 0x81
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[1, 0]); // Halt bit set
        } else {
            panic!("Expected TransferIn action");
        }

        // 4. CLEAR_FEATURE(ENDPOINT_HALT) to Endpoint 1 IN
        let req = Request::ENDPOINT_CLEAR_FEATURE;
        let buf0 = u16::from(req) as u32; // Value = 0 (ENDPOINT_HALT)
        let setup_pkt = SetupPacket::new([buf0, 0x0000_0081]); // Endpoint = 0x81
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(
            action,
            UsbAction::TransferIn {
                endpoint: 0,
                zlp: true,
                ..
            }
        ));

        // Verify that handle_packet_sent returns EndpointHalt action (halted = false)
        let action_sent = ep0.handle_packet_sent();
        assert!(matches!(
            action_sent,
            UsbAction::EndpointHalt {
                endpoint_addr: 0x81,
                halted: false
            }
        ));

        // 5. Check Endpoint 1 IN status again (should be not halted)
        let req = Request::ENDPOINT_GET_STATUS;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0002_0081]); // Length = 2, Endpoint = 0x81
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        if let UsbAction::TransferIn {
            endpoint: 0, data, ..
        } = action
        {
            assert_eq!(data.as_ref(), &[0, 0]);
        } else {
            panic!("Expected TransferIn action");
        }

        // 6. Trying to halt Endpoint 0 should fail/stall
        let req = Request::ENDPOINT_SET_FEATURE;
        let buf0 = u16::from(req) as u32;
        let setup_pkt = SetupPacket::new([buf0, 0x0000_0000]); // Endpoint = 0
        let ev: UsbEvent<testing::FakeUsbPacket<'static>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup_pkt,
        };
        let action = unwrap_action(ep0.handle_event(ev, &descriptors));
        assert!(matches!(action, UsbAction::StallInAndOut { endpoint: 0 }));
    }
}
