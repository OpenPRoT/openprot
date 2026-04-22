//! CDC-ACM (Serial) USB class implementation.

#![no_std]

pub use hal_usb::driver::{UsbEvent, UsbPacket, UsbDriver};
pub use hal_usb::{
    Direction, EndpointDescriptor, FunctionalDescriptor, InterfaceDescriptor, Recipient, Request,
    RequestType, SetupPacket, StringHandle, TransferType,
};
pub use usb_driver::{EpIn, EpOut};
use usb_stack::{UsbAction, UsbClass, EMPTY};
use util_ringbuffer::RingBuffer;

/// CDC-ACM specific constants.
pub const USB_CLASS_CDC: u8 = 0x02;
pub const USB_CLASS_CDC_DATA: u8 = 0x0a;
pub const CDC_SUBCLASS_ACM: u8 = 0x02;
pub const CDC_PROTOCOL_NONE: u8 = 0x00;

pub const CS_INTERFACE: u8 = 0x24;
pub const CDC_TYPE_HEADER: u8 = 0x00;
pub const CDC_TYPE_ACM: u8 = 0x02;
pub const CDC_TYPE_UNION: u8 = 0x06;

/// CDC-ACM specific requests.
pub const REQ_SEND_ENCAPSULATED_COMMAND: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    0x00,
);
pub const REQ_GET_ENCAPSULATED_COMMAND: Request = Request::new(
    Direction::DeviceToHost,
    RequestType::Class,
    Recipient::Interface,
    0x01,
);
pub const REQ_SET_LINE_CODING: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    0x20,
);
pub const REQ_GET_LINE_CODING: Request = Request::new(
    Direction::DeviceToHost,
    RequestType::Class,
    Recipient::Interface,
    0x21,
);
pub const REQ_SET_CONTROL_LINE_STATE: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    0x22,
);

#[derive(Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum StopBits {
    #[default]
    One = 0,
    OnePointFive = 1,
    Two = 2,
}

impl From<u8> for StopBits {
    fn from(x: u8) -> Self {
        match x {
            0 => StopBits::One,
            1 => StopBits::OnePointFive,
            2 => StopBits::Two,
            _ => StopBits::One,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Parity {
    #[default]
    None = 0,
    Odd = 1,
    Even = 2,
    Mark = 3,
    Space = 4,
}

impl From<u8> for Parity {
    fn from(x: u8) -> Self {
        match x {
            0 => Parity::None,
            1 => Parity::Odd,
            2 => Parity::Even,
            3 => Parity::Mark,
            4 => Parity::Space,
            _ => Parity::None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct LineCoding {
    pub data_rate: u32,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub data_bits: u8,
}

impl TryFrom<&[u8]> for LineCoding {
    type Error = ();
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() >= 7 {
            Ok(LineCoding {
                data_rate: u32::from_le_bytes(data[0..4].try_into().unwrap()),
                stop_bits: StopBits::from(data[4]),
                parity: Parity::from(data[5]),
                data_bits: data[6],
            })
        } else {
            Err(())
        }
    }
}

impl Default for LineCoding {
    fn default() -> Self {
        LineCoding {
            data_rate: 9600,
            stop_bits: StopBits::default(),
            parity: Parity::default(),
            data_bits: 8,
        }
    }
}

impl LineCoding {
    pub fn as_bytes(&self) -> &[u8] {
        let x = unsafe { core::mem::transmute::<&LineCoding, &[u8; 7]>(self) };
        &*x
    }
}

/// A builder for CDC-ACM class configuration.
///
/// This builder encapsulates the constants and configuration logic for a
/// CDC-ACM instance. It provides methods to generate descriptor fragments
/// and the final `InterfaceDescriptor` structs, hiding class-specific details
/// from the application.
///
/// # Convention
/// USB classes should provide a `Builder` struct with `const` methods that:
/// 1. Return arrays of child descriptors (functional, endpoints).
/// 2. Construct fully-populated `InterfaceDescriptor`s from application-provided
///    handles and static fragments.
/// 3. Provide hardware configuration (`EpIn`, `EpOut`).
#[derive(Copy, Clone)]
pub struct CdcAcmBuilder {
    pub comm_if: u8,
    pub data_if: u8,
    pub comm_ep: u8,
    pub data_out_ep: u8,
    pub data_in_ep: u8,
}

impl CdcAcmBuilder {
    /// Creates a new CDC-ACM configuration.
    pub const fn new(comm_if: u8, data_if: u8, comm_ep: u8, data_out_ep: u8, data_in_ep: u8) -> Self {
        Self {
            comm_if,
            data_if,
            comm_ep,
            data_out_ep,
            data_in_ep,
        }
    }

    /// Returns the functional descriptors for the control interface.
    pub const fn comm_func_descs(&self) -> [FunctionalDescriptor; 3] {
        [
            FunctionalDescriptor::raw(CS_INTERFACE, &[CDC_TYPE_HEADER, 0x10, 0x01]),
            FunctionalDescriptor::raw(CS_INTERFACE, &[CDC_TYPE_ACM, 0x02]),
            FunctionalDescriptor::raw(CS_INTERFACE, &[CDC_TYPE_UNION, self.comm_if, self.data_if]),
        ]
    }

    /// Returns the endpoints for the control interface.
    pub const fn comm_endpoints(&self) -> [EndpointDescriptor; 1] {
        [EndpointDescriptor {
            direction: Direction::DeviceToHost,
            endpoint_num: self.comm_ep,
            interval: 255,
            max_packet_size: 8,
            transfer_type: TransferType::Interrupt,
        }]
    }

    /// Returns the endpoints for the data interface.
    pub const fn data_endpoints(&self) -> [EndpointDescriptor; 2] {
        [
            EndpointDescriptor {
                direction: Direction::HostToDevice,
                endpoint_num: self.data_out_ep,
                interval: 0,
                max_packet_size: 64,
                transfer_type: TransferType::Bulk,
            },
            EndpointDescriptor {
                direction: Direction::DeviceToHost,
                endpoint_num: self.data_in_ep,
                interval: 0,
                max_packet_size: 64,
                transfer_type: TransferType::Bulk,
            },
        ]
    }

    /// Constructs the CDC-ACM Communication (Control) interface descriptor.
    pub const fn comm_interface(
        &self,
        name: StringHandle,
        func_descs: &'static [FunctionalDescriptor],
        endpoints: &'static [EndpointDescriptor],
    ) -> InterfaceDescriptor {
        InterfaceDescriptor {
            name,
            interface_number: self.comm_if,
            alternate_setting: 0,
            interface_class: USB_CLASS_CDC,
            interface_sub_class: CDC_SUBCLASS_ACM,
            interface_protocol: CDC_PROTOCOL_NONE,
            func_descs,
            endpoints,
        }
    }

    /// Constructs the CDC-ACM Data interface descriptor.
    pub const fn data_interface(
        &self,
        name: StringHandle,
        endpoints: &'static [EndpointDescriptor],
    ) -> InterfaceDescriptor {
        InterfaceDescriptor {
            name,
            interface_number: self.data_if,
            alternate_setting: 0,
            interface_class: USB_CLASS_CDC_DATA,
            interface_sub_class: 0,
            interface_protocol: CDC_PROTOCOL_NONE,
            func_descs: &[],
            endpoints,
        }
    }

    /// Returns the hardware endpoint configuration for this CDC-ACM instance.
    pub const fn eps(&self) -> ([EpIn; 2], [EpOut; 1]) {
        (
            [
                EpIn {
                    num: self.comm_ep,
                    buf_pool_size: 1,
                },
                EpIn {
                    num: self.data_in_ep,
                    buf_pool_size: 1,
                },
            ],
            [EpOut {
                num: self.data_out_ep,
                set_nak: false,
            }],
        )
    }
}

/// CDC-ACM class handler.
pub struct CdcAcm<const RX_SIZE: usize, const TX_SIZE: usize> {
    config: CdcAcmBuilder,
    line_coding: LineCoding,
    expecting_control_out: bool,
    control_buf: [u8; 8],
    pub rx_queue: RingBuffer<u8, RX_SIZE>,
    pub tx_queue: RingBuffer<u8, TX_SIZE>,
}

impl<const RX_SIZE: usize, const TX_SIZE: usize> CdcAcm<RX_SIZE, TX_SIZE> {
    /// Creates a new CDC-ACM class handler from a builder.
    pub fn new(config: CdcAcmBuilder) -> Self {
        Self {
            config,
            line_coding: LineCoding::default(),
            expecting_control_out: false,
            control_buf: [0u8; 8],
            rx_queue: RingBuffer::default(),
            tx_queue: RingBuffer::default(),
        }
    }

    /// Returns the configuration builder for this instance.
    pub fn config(&self) -> &CdcAcmBuilder {
        &self.config
    }

    fn handle_setup<'a>(&'a mut self, pkt: SetupPacket) -> (UsbAction<'a>, bool) {
        if !(pkt.request().recipient() == Recipient::Interface
            && (pkt.index() as u8) == self.config.comm_if)
        {
            return (UsbAction::None, false);
        }

        match pkt.request() {
            REQ_SEND_ENCAPSULATED_COMMAND => (
                UsbAction::TransferIn {
                    endpoint: 0,
                    data: EMPTY,
                    zlp: true,
                },
                false,
            ),
            REQ_GET_ENCAPSULATED_COMMAND => (UsbAction::StallInAndOut { endpoint: 0 }, false),
            REQ_SET_LINE_CODING => {
                self.expecting_control_out = true;
                (UsbAction::None, true)
            }
            REQ_GET_LINE_CODING => (
                UsbAction::TransferInUnaligned {
                    endpoint: 0,
                    data: self.line_coding.as_bytes(),
                    zlp: true,
                },
                false,
            ),
            REQ_SET_CONTROL_LINE_STATE => (
                UsbAction::TransferIn {
                    endpoint: 0,
                    data: EMPTY,
                    zlp: true,
                },
                false,
            ),
            _ => (UsbAction::StallInAndOut { endpoint: 0 }, false),
        }
    }

    fn handle_control_out<'a>(&'a mut self, pkt: impl UsbPacket) -> UsbAction<'a> {
        let buf = pkt.copy_to_unaligned(&mut self.control_buf);
        self.expecting_control_out = false;
        match LineCoding::try_from(buf) {
            Ok(x) => {
                self.line_coding = x;
                UsbAction::TransferIn {
                    endpoint: 0,
                    data: EMPTY,
                    zlp: true,
                }
            }
            Err(_) => UsbAction::StallInAndOut { endpoint: 0 },
        }
    }

    /// Polls the send buffer and initiates IN transfers if data is available.
    pub fn poll_transmit<D: UsbDriver>(&mut self, driver: &mut D) {
        let data = self.tx_queue.as_slice();
        if !data.is_empty() {
            let n = driver.transfer_in_unaligned(self.config.data_in_ep, data, true);
            self.tx_queue.consume(n);
        }
    }
}

impl<const RX_SIZE: usize, const TX_SIZE: usize> UsbClass for CdcAcm<RX_SIZE, TX_SIZE> {
    fn handle_event<'a, P: UsbPacket>(
        &'a mut self,
        event: UsbEvent<P>,
    ) -> Result<UsbAction<'a>, UsbEvent<P>> {
        match event {
            UsbEvent::SetupPacket { pkt, endpoint } if endpoint == 0 => {
                let (action, claimed) = self.handle_setup(pkt);
                if action != UsbAction::None || claimed {
                    Ok(action)
                } else {
                    Err(UsbEvent::SetupPacket { pkt, endpoint })
                }
            }
            UsbEvent::DataOutPacket(pkt) => {
                if pkt.endpoint_index() == 0 && self.expecting_control_out {
                    Ok(self.handle_control_out(pkt))
                } else if pkt.endpoint_index() == self.config.data_out_ep as usize {
                    let mut tmp = [0u8; 64];
                    let buf = pkt.copy_to_unaligned(&mut tmp);
                    let _ = self.rx_queue.push_slice(buf);
                    Ok(UsbAction::None)
                } else {
                    Err(UsbEvent::DataOutPacket(pkt))
                }
            }
            _ => Err(event),
        }
    }
}
