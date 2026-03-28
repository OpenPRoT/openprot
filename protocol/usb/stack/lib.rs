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
    /// Get the status of an endpoint.
    GetEndpointStatus {
        /// The endpoint index.
        endpoint: u8,
    },
    /// Set the stall status of an endpoint.
    SetEndpointStatus {
        /// The endpoint index.
        endpoint: u8,
        /// Whether to stall or unstall the endpoint.
        stall: bool,
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
            (Self::StallInAndOut { endpoint: e1 }, Self::StallInAndOut { endpoint: e2 }) => e1 == e2,
            (Self::SetAddress { new_address: a1 }, Self::SetAddress { new_address: a2 }) => a1 == a2,
            (
                Self::GetEndpointStatus { endpoint: e1 },
                Self::GetEndpointStatus { endpoint: e2 },
            ) => e1 == e2,
            (
                Self::SetEndpointStatus {
                    endpoint: e1,
                    stall: s1,
                },
                Self::SetEndpointStatus {
                    endpoint: e2,
                    stall: s2,
                },
            ) => e1 == e2 && s1 == s2,
            _ => false,
        }
    }
}
impl Eq for UsbAction<'_> {}

impl<'a> UsbAction<'a> {
    const EP_CLEAR: Aligned<A4, [u8; 2]> = Aligned([0u8, 0]);
    const EP_HALTED: Aligned<A4, [u8; 2]> = Aligned([1u8, 0]);

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
            Self::GetEndpointStatus { endpoint } => {
                let data = if driver.is_stalled(*endpoint) {
                    &Self::EP_HALTED
                } else {
                    &Self::EP_CLEAR
                };
                let _ = driver.transfer_in(0, data, true);
            }
            Self::SetEndpointStatus { endpoint, stall } => {
                driver.stall(*endpoint, *stall);
                let _ = driver.transfer_in(0, EMPTY, true);
            }
        }
        *self = UsbAction::None;
        UsbActionRun::Done
    }
}

impl SimpleEp0 {
    /// Creates a new `SimpleEp0` handler.
    pub fn new() -> Self {
        Self { new_address: None }
    }
    /// A helper function to process a driver UsbEvent.
    ///
    /// This function returns the action that should be performed on the driver.
    pub fn handle_event<'a>(
        &mut self,
        ev: UsbEvent<impl UsbPacket>,
        descriptor_source: &'a impl DescriptorSource,
    ) -> UsbAction<'a> {
        match ev {
            UsbEvent::SetupPacket { endpoint, pkt } => {
                if endpoint == 0 {
                    return self.handle_setup(pkt, descriptor_source);
                }
            }
            UsbEvent::PacketSent { endpoint } => {
                if endpoint == 0 {
                    return self.handle_packet_sent();
                }
            }
            _ => {}
        }
        UsbAction::None
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
            Request::DEVICE_GET_STATUS => UsbAction::TransferIn {
                endpoint: 0,
                data: descriptor_source.get_device_status(),
                zlp: true,
            },
            Request::ENDPOINT_GET_STATUS => UsbAction::GetEndpointStatus {
                endpoint: setup_pkt.index() as u8,
            },
            Request::ENDPOINT_SET_FEATURE => UsbAction::SetEndpointStatus {
                endpoint: setup_pkt.index() as u8,
                stall: true,
            },
            Request::ENDPOINT_CLEAR_FEATURE => UsbAction::SetEndpointStatus {
                endpoint: setup_pkt.index() as u8,
                stall: false,
            },
            Request::DEVICE_SET_ADDRESS => {
                self.new_address = Some(setup_pkt.value() as u8);
                UsbAction::TransferIn {
                    endpoint: 0,
                    data: EMPTY,
                    zlp: true,
                }
            }
            Request::DEVICE_SET_CONFIGURATION => {
                if setup_pkt.value() == 1 {
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
            // This is safe because `self.buffer` is `[u32]` which has alignment of 4.
            Ok(Some(unsafe {
                core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(result)
            }))
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
            // This is safe because `dest` is a `&mut [u32]`, which is guaranteed to be 4-byte
            // aligned. `dest_bytes` is a byte slice view of the same memory, so it's also
            // 4-byte aligned. The subslice `&dest_bytes[..copy_len]` maintains this alignment.
            unsafe { core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(&dest_bytes[..copy_len]) }
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
        assert_eq!(&buffer[2 * MAX_PACKET_SIZE..], PARTIAL_DATA);
    }
}
