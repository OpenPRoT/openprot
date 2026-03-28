//! CDC-ACM (Serial) USB class implementation.

#![no_std]

use aligned::{Aligned, A4};
use hal_usb::driver::{UsbEvent, UsbPacket};
use hal_usb::{Direction, Recipient, Request, RequestType, SetupPacket};
use usb_stack::{UsbAction, UsbClass, EMPTY};

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

/// CDC-ACM class handler.
#[allow(dead_code)]
pub struct CdcAcm {
    comm_if: u8,
    data_if: u8,
    out_ep: u8,
    in_ep: u8,
    line_coding: LineCoding,
    expecting_control_out: bool,
    rx_buffer: Aligned<A4, [u32; 16]>,
}

impl CdcAcm {
    pub fn new(comm_if: u8, data_if: u8, out_ep: u8, in_ep: u8) -> Self {
        Self {
            comm_if,
            data_if,
            out_ep,
            in_ep,
            line_coding: LineCoding::default(),
            expecting_control_out: false,
            rx_buffer: Aligned([0u32; 16]),
        }
    }

    fn handle_setup<'a>(&'a mut self, pkt: SetupPacket) -> (UsbAction<'a>, bool) {
        if !(pkt.request().recipient() == Recipient::Interface
            && (pkt.index() as u8) == self.comm_if)
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
            REQ_GET_LINE_CODING => {
                let data = unsafe {
                    core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(self.line_coding.as_bytes())
                };
                (
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data,
                        zlp: true,
                    },
                    false,
                )
            }
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
        let mut data = [0u32; 2];
        let buf = pkt.copy_to(&mut data);
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
}

impl UsbClass for CdcAcm {
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
                } else if pkt.endpoint_index() == self.out_ep as usize {
                    let buf = pkt.copy_to(self.rx_buffer.as_mut());
                    Ok(UsbAction::TransferIn {
                        endpoint: self.in_ep,
                        data: unsafe { core::mem::transmute(buf) },
                        zlp: true,
                    })
                } else {
                    Err(UsbEvent::DataOutPacket(pkt))
                }
            }
            _ => Err(event),
        }
    }
}
