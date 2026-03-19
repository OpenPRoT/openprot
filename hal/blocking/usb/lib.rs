#![cfg_attr(not(test), no_std)]

mod descriptor;
pub mod driver;

use ufmt::derive::uDebug;

pub use descriptor::*;

// Big endian is dead; code in this file assumes little-endian
const _: () = assert!(cfg!(target_endian = "little"));

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Request(u16);
#[allow(clippy::identity_op)]
impl Request {
    pub const fn new(
        direction: Direction,
        ty: RequestType,
        recipient: Recipient,
        request: u8,
    ) -> Self {
        Self(
            ((direction as u16) << 7)
                | ((ty as u16) << 5)
                | ((recipient as u16) << 0)
                | ((request as u16) << 8),
        )
    }
    pub fn direction(&self) -> Direction {
        Direction::try_from((u32::from(self.0) >> 7) & 0x1).unwrap()
    }
    pub fn request_type(&self) -> RequestType {
        RequestType::try_from(u32::from((self.0 >> 5) & 0x3)).unwrap()
    }
    pub fn recipient(&self) -> Recipient {
        Recipient::try_from(u32::from((self.0 >> 0) & 0x1f)).unwrap()
    }
    pub fn request(&self) -> u8 {
        u8::try_from((self.0 >> 8) & 0xff).unwrap()
    }
}
impl ufmt::uDebug for Request {
    fn fmt<W: ufmt::uWrite + ?Sized>(
        &self,
        f: &mut ufmt::Formatter<'_, W>,
    ) -> Result<(), W::Error> {
        f.debug_struct("usb::Request")?
            .field("request_type", &self.request_type())?
            .field("direction", &self.direction())?
            .field("recipient", &self.recipient())?
            .field("request", &self.request())?
            .finish()
    }
}
impl Request {
    pub const DEVICE_GET_STATUS: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Device,
        0x00,
    );
    pub const DEVICE_CLEAR_FEATURE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Device,
        0x01,
    );
    pub const DEVICE_SET_FEATURE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Device,
        0x03,
    );
    pub const DEVICE_SET_ADDRESS: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Device,
        0x05,
    );
    pub const DEVICE_GET_DESCRIPTOR: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Device,
        0x06,
    );
    pub const DEVICE_SET_DESCRIPTOR: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Device,
        0x07,
    );
    pub const DEVICE_GET_CONFIGURATION: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Device,
        0x08,
    );
    pub const DEVICE_SET_CONFIGURATION: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Device,
        0x09,
    );
    pub const INTERFACE_GET_STATUS: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Interface,
        0x00,
    );
    pub const INTERFACE_CLEAR_FEATURE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Interface,
        0x01,
    );
    pub const INTERFACE_SET_FEATURE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Interface,
        0x03,
    );
    pub const INTERFACE_GET_INTERFACE: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Interface,
        0x0a,
    );
    pub const INTERFACE_SET_INTERFACE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Interface,
        0x0b,
    );
    pub const ENDPOINT_GET_STATUS: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Endpoint,
        0x00,
    );
    pub const ENDPOINT_CLEAR_FEATURE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Endpoint,
        0x01,
    );
    pub const ENDPOINT_SET_FEATURE: Self = Self::new(
        Direction::HostToDevice,
        RequestType::Standard,
        Recipient::Endpoint,
        0x03,
    );
    pub const ENDPOINT_SYNCH_FRAME: Self = Self::new(
        Direction::DeviceToHost,
        RequestType::Standard,
        Recipient::Endpoint,
        0x12,
    );
}
impl From<Request> for u16 {
    fn from(val: Request) -> Self {
        val.0
    }
}
#[cfg(test)]
mod request_tests {
    use super::*;
    #[test]
    fn test_constants() {
        assert_eq!(u16::from(Request::DEVICE_GET_STATUS), 0x0080);
        assert_eq!(u16::from(Request::DEVICE_CLEAR_FEATURE), 0x0100);
        assert_eq!(u16::from(Request::DEVICE_SET_FEATURE), 0x0300);
        assert_eq!(u16::from(Request::DEVICE_SET_ADDRESS), 0x0500);
        assert_eq!(u16::from(Request::DEVICE_GET_DESCRIPTOR), 0x0680);
        assert_eq!(u16::from(Request::DEVICE_SET_DESCRIPTOR), 0x0700);
        assert_eq!(u16::from(Request::DEVICE_GET_CONFIGURATION), 0x0880);
        assert_eq!(u16::from(Request::DEVICE_SET_CONFIGURATION), 0x0900);
        assert_eq!(u16::from(Request::INTERFACE_GET_STATUS), 0x0081);
        assert_eq!(u16::from(Request::INTERFACE_CLEAR_FEATURE), 0x0101);
        assert_eq!(u16::from(Request::INTERFACE_SET_FEATURE), 0x0301);
        assert_eq!(u16::from(Request::INTERFACE_GET_INTERFACE), 0x0a81);
        assert_eq!(u16::from(Request::INTERFACE_SET_INTERFACE), 0x0b01);
        assert_eq!(u16::from(Request::ENDPOINT_GET_STATUS), 0x0082);
        assert_eq!(u16::from(Request::ENDPOINT_CLEAR_FEATURE), 0x0102);
        assert_eq!(u16::from(Request::ENDPOINT_SET_FEATURE), 0x0302);
        assert_eq!(u16::from(Request::ENDPOINT_SYNCH_FRAME), 0x1282);
    }
}

#[derive(Clone, Copy, Eq, PartialEq, uDebug)]
pub struct DescriptorInfo {
    pub index: u8,
    pub ty: DescriptorType,
    pub lang: u16,
}
impl From<&SetupPacket> for DescriptorInfo {
    fn from(pkt: &SetupPacket) -> Self {
        DescriptorInfo {
            index: u8::try_from(pkt.value() & 0xff).unwrap(),
            ty: DescriptorType::from(u8::try_from((pkt.value() >> 8) & 0xff).unwrap()),
            lang: pkt.index(),
        }
    }
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SetupPacket {
    buf: [u32; 2],
}
impl SetupPacket {
    pub fn new(buf: [u32; 2]) -> SetupPacket {
        SetupPacket { buf }
    }
    pub fn request(&self) -> Request {
        Request(u16::try_from(self.buf[0] & 0xffff).unwrap())
    }
    pub fn value(&self) -> u16 {
        u16::try_from((self.buf[0] >> 16) & 0xffff).unwrap()
    }
    #[allow(clippy::identity_op)]
    pub fn index(&self) -> u16 {
        u16::try_from((self.buf[1] >> 0) & 0xffff).unwrap()
    }
    pub fn length(&self) -> u16 {
        u16::try_from((self.buf[1] >> 16) & 0xffff).unwrap()
    }
}
impl ufmt::uDebug for SetupPacket {
    fn fmt<W: ufmt::uWrite + ?Sized>(
        &self,
        f: &mut ufmt::Formatter<'_, W>,
    ) -> Result<(), W::Error> {
        f.debug_struct("usb::SetupPacket")?
            .field("request", &self.request())?
            .field("value", &self.value())?
            .field("index", &self.index())?
            .field("length", &self.length())?
            .finish()
    }
}

#[derive(Clone, Copy, Eq, PartialEq, uDebug)]
pub enum Direction {
    HostToDevice = 0,
    DeviceToHost = 1,
}
impl From<Direction> for u32 {
    fn from(val: Direction) -> u32 {
        val as u32
    }
}
impl TryFrom<u32> for Direction {
    type Error = ();
    #[inline(always)]
    fn try_from(val: u32) -> Result<Direction, ()> {
        match val {
            0 => Ok(Self::HostToDevice),
            1 => Ok(Self::DeviceToHost),
            _ => Err(()),
        }
    }
}
#[derive(Clone, Copy, Eq, PartialEq, uDebug)]
pub enum RequestType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}
impl TryFrom<u32> for RequestType {
    type Error = ();
    #[inline(always)]
    fn try_from(val: u32) -> Result<RequestType, ()> {
        match val {
            0 => Ok(Self::Standard),
            1 => Ok(Self::Class),
            2 => Ok(Self::Vendor),
            3 => Ok(Self::Reserved),
            _ => Err(()),
        }
    }
}
impl From<RequestType> for u32 {
    fn from(val: RequestType) -> Self {
        val as u32
    }
}
#[derive(Clone, Copy, Eq, PartialEq, uDebug)]
pub enum Recipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
    Reserved4 = 4,
    Reserved5 = 5,
    Reserved6 = 6,
    Reserved7 = 7,
    Reserved8 = 8,
    Reserved9 = 9,
    Reserved10 = 10,
    Reserved11 = 11,
    Reserved12 = 12,
    Reserved13 = 13,
    Reserved14 = 14,
    Reserved15 = 15,
    Reserved16 = 16,
    Reserved17 = 17,
    Reserved18 = 18,
    Reserved19 = 19,
    Reserved20 = 20,
    Reserved21 = 21,
    Reserved22 = 22,
    Reserved23 = 23,
    Reserved24 = 24,
    Reserved25 = 25,
    Reserved26 = 26,
    Reserved27 = 27,
    Reserved28 = 28,
    Reserved29 = 29,
    Reserved30 = 30,
    Reserved31 = 31,
}
impl TryFrom<u32> for Recipient {
    type Error = ();
    #[inline(always)]
    fn try_from(val: u32) -> Result<Recipient, ()> {
        // TODO: Evaluate whether the optimizer is smart enough for this, and use
        // transmute if it's not.
        match val {
            0 => Ok(Self::Device),
            1 => Ok(Self::Interface),
            2 => Ok(Self::Endpoint),
            3 => Ok(Self::Other),
            4 => Ok(Self::Reserved4),
            5 => Ok(Self::Reserved5),
            6 => Ok(Self::Reserved6),
            7 => Ok(Self::Reserved7),
            8 => Ok(Self::Reserved8),
            9 => Ok(Self::Reserved9),
            10 => Ok(Self::Reserved10),
            11 => Ok(Self::Reserved11),
            12 => Ok(Self::Reserved12),
            13 => Ok(Self::Reserved13),
            14 => Ok(Self::Reserved14),
            15 => Ok(Self::Reserved15),
            16 => Ok(Self::Reserved16),
            17 => Ok(Self::Reserved17),
            18 => Ok(Self::Reserved18),
            19 => Ok(Self::Reserved19),
            20 => Ok(Self::Reserved20),
            21 => Ok(Self::Reserved21),
            22 => Ok(Self::Reserved22),
            23 => Ok(Self::Reserved23),
            24 => Ok(Self::Reserved24),
            25 => Ok(Self::Reserved25),
            26 => Ok(Self::Reserved26),
            27 => Ok(Self::Reserved27),
            28 => Ok(Self::Reserved28),
            29 => Ok(Self::Reserved29),
            30 => Ok(Self::Reserved30),
            31 => Ok(Self::Reserved31),
            _ => Err(()),
        }
    }
}
impl From<Recipient> for u32 {
    fn from(val: Recipient) -> Self {
        val as u32
    }
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DescriptorType(u8);
impl DescriptorType {
    pub const DEVICE: Self = Self(1);
    pub const CONFIGURATION: Self = Self(2);
    pub const STRING: Self = Self(3);
    pub const INTERFACE: Self = Self(4);
    pub const ENDPOINT: Self = Self(5);
    pub const DEVICE_QUALIFIER: Self = Self(6);
}
impl From<u8> for DescriptorType {
    fn from(val: u8) -> Self {
        DescriptorType(val)
    }
}
impl From<DescriptorType> for u8 {
    fn from(val: DescriptorType) -> Self {
        val.0
    }
}
impl From<DescriptorType> for u32 {
    fn from(val: DescriptorType) -> Self {
        u32::from(val.0)
    }
}
impl ufmt::uDebug for DescriptorType {
    fn fmt<W: ufmt::uWrite + ?Sized>(
        &self,
        f: &mut ufmt::Formatter<'_, W>,
    ) -> Result<(), W::Error> {
        match *self {
            Self::DEVICE => f.write_str("DEVICE"),
            Self::CONFIGURATION => f.write_str("CONFIGURATION"),
            Self::STRING => f.write_str("STRING"),
            Self::INTERFACE => f.write_str("INTERFACE"),
            Self::ENDPOINT => f.write_str("ENDPOINT"),
            Self::DEVICE_QUALIFIER => f.write_str("DEVICE_QUALIFIER"),
            other => ufmt::uwrite!(f, "{}", other.0),
        }
    }
}
