use aligned::Aligned;
use aligned::A4;
use ufmt::uWrite;

pub const USB_CLASS_AUDIO: u8 = 0x01;
pub const USB_CLASS_COMMUNIATIONS: u8 = 0x02;
pub const USB_CLASS_HID: u8 = 0x03;
pub const USB_CLASS_PHYSICAL: u8 = 0x05;
pub const USB_CLASS_IMAGE: u8 = 0x06;
pub const USB_CLASS_PRINTER: u8 = 0x07;
pub const USB_CLASS_MASS_STORAGE: u8 = 0x08;
pub const USB_CLASS_HUB: u8 = 0x09;
pub const USB_CLASS_CDC_DATA: u8 = 0x0a;
pub const USB_CLASS_SMART_CARD: u8 = 0x0b;
pub const USB_CLASS_CONTENT_SECURITY: u8 = 0x0d;
pub const USB_CLASS_VIDEO: u8 = 0x0e;
pub const USB_CLASS_PERSONAL_HEALTHCARE: u8 = 0x0f;
pub const USB_CLASS_AUDIO_VIDEO: u8 = 0x10;
pub const USB_CLASS_BILLBOARD: u8 = 0x11;
pub const USB_CLASS_USB_TYPEC_BRIDGE: u8 = 0x12;
pub const USB_CLASS_BULK_DISPLAY: u8 = 0x13;
pub const USB_CLASS_MCTP: u8 = 0x14;
pub const USB_CLASS_I3C: u8 = 0x3c;
pub const USB_CLASS_DIAGNOSTIC_DEVICE: u8 = 0xdc;
pub const USB_CLASS_WIRELESS_CONTROLLER: u8 = 0xe0;
pub const USB_CLASS_MISC: u8 = 0xef;
pub const USB_CLASS_APPLICATION_SPECIFIC: u8 = 0xfe;
pub const USB_CLASS_VENDOR: u8 = 0xff;

pub const USB_SUBCLASS_APPLICATION_SPECIFIC_DFU: u8 = 0x01;

pub const USB_PROTOCOL_APPLICATION_SPECIFIC_DFU_RUNTIME_MODE: u8 = 0x01;
pub const USB_PROTOCOL_APPLICATION_SPECIFIC_DFU_DFU_MODE: u8 = 0x02;

use crate::DescriptorType;
use crate::Direction;

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct StringHandle(pub u8);
impl StringHandle {
    pub const NONE: Self = StringHandle(0);
}

pub struct DeviceDescriptor {
    pub device_class: DeviceClass,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_release_num: u16,
    pub manufacturer: StringHandle,
    pub product: StringHandle,
    pub serial_num: StringHandle,
}
impl DeviceDescriptor {
    const SIZE: usize = 18;

    #[allow(dead_code)]
    pub(crate) const fn total_size(&self) -> usize {
        Self::SIZE
    }

    #[allow(clippy::identity_op)]
    pub const fn serialize(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        // sizeof descriptor
        buf[0] = 18;
        // bDescriptorType = Device
        buf[1] = 1;
        // USB version 2.0
        buf[2] = 0x00;
        buf[3] = 0x02;

        buf[4] = self.device_class.0;
        buf[5] = self.device_sub_class;
        buf[6] = self.device_protocol;
        buf[7] = self.max_packet_size;

        buf[8] = ((self.vendor_id & 0x00ff) >> 0) as u8;
        buf[9] = ((self.vendor_id & 0xff00) >> 8) as u8;

        buf[10] = ((self.product_id & 0x00ff) >> 0) as u8;
        buf[11] = ((self.product_id & 0xff00) >> 8) as u8;

        buf[12] = ((self.device_release_num & 0x00ff) >> 0) as u8;
        buf[13] = ((self.device_release_num & 0xff00) >> 8) as u8;

        buf[14] = self.manufacturer.0;
        buf[15] = self.product.0;
        buf[16] = self.serial_num.0;

        // num configurations
        buf[17] = 1;
        buf
    }
}

pub struct ConfigDescriptor {
    pub configuration_value: u8,
    // in 2 mA units
    pub max_power: u8,
    pub self_powered: bool,
    pub remote_wakeup: bool,
    pub interfaces: &'static [InterfaceDescriptor],
}

impl ConfigDescriptor {
    const SIZE: usize = 9;

    pub const fn total_size(&self) -> usize {
        let mut result = Self::SIZE;
        let mut i = 0;
        while i < self.interfaces.len() {
            result += self.interfaces[i].total_size();
            i += 1;
        }
        result
    }

    #[allow(clippy::identity_op)]
    pub const fn serialize<const RESULT_SIZE: usize>(&self) -> [u8; RESULT_SIZE] {
        assert!(self.total_size() == RESULT_SIZE);
        let mut buf = [0u8; RESULT_SIZE];

        // alternates don't count towards total
        // as interfaces numbers are per spec monotonically increasing, we can use that as the count
        let mut uniq_interface_count = 0;
        let mut i = 0;
        while i < self.interfaces.len() {
            if self.interfaces[i].interface_number + 1 > uniq_interface_count {
                uniq_interface_count = self.interfaces[i].interface_number + 1
            }
            i += 1;
        }

        // sizeof descriptor
        buf[0] = 9;
        // bDescriptorType = Configuration
        buf[1] = 2;
        buf[2] = ((RESULT_SIZE & 0x00ff) >> 0) as u8;
        buf[3] = ((RESULT_SIZE & 0xff00) >> 8) as u8;
        buf[4] = uniq_interface_count;
        buf[5] = self.configuration_value;
        // iConfiguration
        buf[6] = 0;
        buf[7] = (1 << 7) | // must be 1 (USB 1.0 bus powered)
                 if self.self_powered { 1 << 6 } else { 0 } |
                 if self.remote_wakeup { 1 << 5 } else { 0 };
        buf[8] = self.max_power;

        let mut offset = 9;

        let mut i = 0;
        while i < self.interfaces.len() {
            let (iface_buf, iface_buf_len) = self.interfaces[i].serialize::<RESULT_SIZE>();
            let mut iface_offset = 0;
            while iface_offset < iface_buf_len {
                buf[offset] = iface_buf[iface_offset];
                iface_offset += 1;
                offset += 1;
            }
            i += 1;
        }
        buf
    }
}

pub struct InterfaceDescriptor {
    pub name: StringHandle,
    pub alternate_setting: u8,
    pub interface_number: u8,
    pub interface_class: u8,
    pub interface_sub_class: u8,
    pub interface_protocol: u8,
    pub func_descs: &'static [FunctionalDescriptor],
    pub endpoints: &'static [EndpointDescriptor],
}
impl InterfaceDescriptor {
    const SIZE: usize = 9;

    const fn total_size(&self) -> usize {
        let mut result = Self::SIZE;
        let mut i = 0;
        while i < self.func_descs.len() {
            result += self.func_descs[i].total_size();
            i += 1;
        }
        let mut i = 0;
        while i < self.endpoints.len() {
            result += self.endpoints[i].total_size();
            i += 1;
        }
        result
    }
    pub const fn serialize<const RESULT_SIZE: usize>(&self) -> ([u8; RESULT_SIZE], usize) {
        assert!(RESULT_SIZE >= self.total_size());

        let mut buf = [0u8; RESULT_SIZE];

        // sizeof descriptor
        buf[0] = 9;
        // bDescriptorType = Interface
        buf[1] = 4;
        buf[2] = self.interface_number;
        buf[3] = self.alternate_setting;
        buf[4] = self.endpoints.len() as u8;
        buf[5] = self.interface_class;
        buf[6] = self.interface_sub_class;
        buf[7] = self.interface_protocol;
        // iInterface: Index of string descriptor describing this interface
        buf[8] = self.name.0;
        let mut offset = 9;

        let mut i = 0;
        while i < self.func_descs.len() {
            self.func_descs[i].serialize(&mut buf, offset);
            offset += self.func_descs[i].total_size();
            i += 1;
        }

        let mut i = 0;
        while i < self.endpoints.len() {
            let ep_buf = self.endpoints[i].serialize(i as u8);
            let mut ep_offset = 0;
            while ep_offset < ep_buf.len() {
                buf[offset] = ep_buf[ep_offset];
                offset += 1;
                ep_offset += 1;
            }
            i += 1;
        }
        (buf, offset)
    }
}

pub struct EndpointDescriptor {
    pub direction: Direction,
    pub endpoint_num: u8,
    pub transfer_type: TransferType,
    pub max_packet_size: u16,
    pub interval: u8,
}
impl EndpointDescriptor {
    const SIZE: usize = 7;

    const fn total_size(&self) -> usize {
        Self::SIZE
    }

    #[allow(clippy::identity_op)]
    const fn serialize(&self, _index: u8) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        // sizeof descriptor
        buf[0] = Self::SIZE as u8;
        // bDescriptorType = endpoint
        buf[1] = 5;
        buf[2] = self.endpoint_num & 0x7
            | match self.direction {
                Direction::HostToDevice => 0,
                Direction::DeviceToHost => 1 << 7,
            };
        buf[3] = match self.transfer_type {
            TransferType::Control => 0,
            TransferType::Isochronous(sync_type, usage_type) => {
                1 | match sync_type {
                    SynchronizationType::None => 0 << 2,
                    SynchronizationType::Asynchronous => 1 << 2,
                    SynchronizationType::Adaptive => 2 << 2,
                    SynchronizationType::Synchronous => 3 << 3,
                } | match usage_type {
                    UsageType::DataEndpoint => 0 << 4,
                    UsageType::FeedbackEndpoint => 1 << 4,
                    UsageType::ExplicitFeedbackDataEndpoint => 2 << 4,
                }
            }
            TransferType::Bulk => 2,
            TransferType::Interrupt => 3,
        };

        buf[4] = ((self.max_packet_size & 0x00ff) >> 0) as u8;
        buf[5] = ((self.max_packet_size & 0xff00) >> 8) as u8;
        buf[6] = self.interval;
        buf
    }
}

pub struct StringDescriptor0 {
    pub langs: &'static [u16],
}
impl StringDescriptor0 {
    pub const fn total_size(&self) -> usize {
        2 + core::mem::size_of_val(self.langs)
    }

    #[allow(clippy::identity_op)]
    pub const fn serialize<const RESULT_SIZE: usize>(&self) -> [u8; RESULT_SIZE] {
        assert!(RESULT_SIZE == self.total_size());
        assert!(self.total_size() <= (u8::MAX as usize));

        let mut buf = [0u8; RESULT_SIZE];
        // sizeof descriptor
        buf[0] = self.total_size() as u8;
        // bDescriptorType = String
        buf[1] = 3;

        let mut offset = 2;
        let mut i = 0;
        while i < self.langs.len() {
            let bytes = self.langs[i].to_le_bytes();
            buf[offset + 0] = bytes[0];
            buf[offset + 1] = bytes[1];
            i += 1;
            offset += 2;
        }
        buf
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum TransferType {
    Control,
    Isochronous(SynchronizationType, UsageType),
    Bulk,
    Interrupt,
}
impl TransferType {
    #[allow(dead_code)]
    fn as_eptyp(self) -> u32 {
        match self {
            TransferType::Control => 0,
            TransferType::Isochronous(_, _) => 1,
            TransferType::Bulk => 2,
            TransferType::Interrupt => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum SynchronizationType {
    None = 0,
    Asynchronous = 1,
    Adaptive = 2,
    Synchronous = 3,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code, clippy::enum_variant_names)]
pub enum UsageType {
    DataEndpoint,
    FeedbackEndpoint,
    ExplicitFeedbackDataEndpoint,
}

pub struct DeviceClass(pub u8);
impl DeviceClass {
    pub const SPECIFIED_BY_INTERFACE: Self = Self(0x00);
    pub const COMMUNICATIONS_AND_CDC: Self = Self(0x02);
    pub const HUB: Self = Self(0x09);
    pub const BILLBOARD: Self = Self(0x11);
    pub const DIAGNOSTIC_DEVICE: Self = Self(0x3c);
    pub const MISCELLANEOUS: Self = Self(0xef);
    pub const VENDOR_SPECIFIED: Self = Self(0xff);
}
impl From<DeviceClass> for u8 {
    fn from(val: DeviceClass) -> Self {
        val.0
    }
}

pub struct StringDescriptor<const BYTE_LEN: usize>(Aligned<A4, [u8; BYTE_LEN]>);

impl<const BYTE_LEN: usize> StringDescriptor<BYTE_LEN> {
    pub const fn const_from_ascii(s: &str) -> Self {
        assert!(BYTE_LEN <= (u8::MAX as usize));
        assert!(s.len() * 2 + 2 == BYTE_LEN);
        let mut result = [0u8; BYTE_LEN];
        result[0] = BYTE_LEN as u8;
        result[1] = 0x03; // DescriptorType string

        let s = s.as_bytes();
        let mut i = 0;
        while i < s.len() {
            if s[i] >= 0x80 {
                panic!("ascii characters only");
            }
            result[2 + i * 2] = s[i];
            i += 1;
        }
        StringDescriptor(Aligned(result))
    }
    pub const fn as_ref(&self) -> StringDescriptorRef<'_> {
        StringDescriptorRef(&self.0)
    }
}

#[derive(Clone, Copy)]
pub struct StringDescriptorRef<'a>(pub &'a Aligned<A4, [u8]>);
impl<'a> StringDescriptorRef<'a> {
    pub const fn as_bytes(self) -> &'a Aligned<A4, [u8]> {
        self.0
    }
}

#[macro_export]
macro_rules! string_descriptor {
    ($s:expr) => {
        $crate::StringDescriptor::<{ $s.len() * 2 + 2 }>::const_from_ascii($s)
    };
}

#[derive(Debug)]
pub enum DescriptorErr {
    Overflow,
    Encoding,
}

#[inline(always)]
pub fn hex_utf16_descriptor(dest: &mut [u8], src: &[u8]) -> Result<usize, DescriptorErr> {
    const { assert!(cfg!(target_endian = "little")) };
    const HEX_CHARS: [u8; 16] = *b"0123456789abcdef";
    let total_len = src.len() * 4 + 2;
    if dest.len() < total_len || total_len > 255 {
        return Err(DescriptorErr::Overflow);
    }
    dest[0] = total_len as u8;
    dest[1] = DescriptorType::STRING.0;

    let mut i = 2;
    for src_byte in src.iter() {
        dest[i] = HEX_CHARS[usize::from(*src_byte >> 4)];
        dest[i + 1] = 0;
        dest[i + 2] = HEX_CHARS[usize::from(*src_byte & 0xf)];
        dest[i + 3] = 0;
        i += 4;
    }
    Ok(total_len)
}

#[inline(always)]
pub fn hex_utf16_descriptor_aligned<'a>(
    dest: &'a mut Aligned<A4, [u8]>,
    src: &[u8],
) -> Result<StringDescriptorRef<'a>, DescriptorErr> {
    let len = hex_utf16_descriptor(dest, src)?;
    Ok(StringDescriptorRef(&dest[..len]))
}

pub struct StringDescriptorWritter<'a> {
    buf: &'a mut Aligned<A4, [u8]>,
    index: usize,
}
impl<'a> StringDescriptorWritter<'a> {
    pub fn new(buf: &'a mut Aligned<A4, [u8]>) -> Result<Self, DescriptorErr> {
        if buf.len() < 2 || buf.len() > 2 + 255 {
            return Err(DescriptorErr::Overflow);
        }
        *buf.get_mut(1).unwrap() = DescriptorType::STRING.0;
        Ok(StringDescriptorWritter { buf, index: 2 })
    }
    pub fn finalize(self) -> Result<StringDescriptorRef<'a>, DescriptorErr> {
        *self.buf.get_mut(0).ok_or(DescriptorErr::Overflow)? =
            u8::try_from(self.index).map_err(|_| DescriptorErr::Overflow)?;

        if self.index > self.buf.len() {
            return Err(DescriptorErr::Overflow);
        }
        Ok(StringDescriptorRef(&self.buf[..self.index]))
    }
}
impl uWrite for StringDescriptorWritter<'_> {
    type Error = core::fmt::Error;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        let bytes = s.as_bytes();
        let remaining_buf = self.buf.get_mut(self.index..).ok_or(core::fmt::Error)?;

        if remaining_buf.len() < bytes.len() * 2 {
            return Err(core::fmt::Error);
        }

        for &b in bytes {
            if b >= 0x80 {
                return Err(core::fmt::Error);
            }
            *self.buf.get_mut(self.index).ok_or(core::fmt::Error)? = b;
            *self.buf.get_mut(self.index + 1).ok_or(core::fmt::Error)? = 0;
            self.index += 2;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test_string_descriptor_writter {
    use aligned::Aligned;
    use aligned::A4;
    use core::ops::Deref;
    use ufmt::uwrite;

    use crate::StringDescriptorWritter;

    #[test]
    fn works() {
        let mut buf = Aligned::<A4, _>([0u8; 30]);
        let mut writter = StringDescriptorWritter::new(&mut buf).unwrap();
        uwrite!(writter, "Hello").unwrap();
        uwrite!(writter, " ").unwrap();
        uwrite!(writter, "World").unwrap();
        let result = writter.finalize().unwrap();
        assert_eq!(
            result.as_bytes().deref(),
            &[
                24, 3, b'H', 0, b'e', 0, b'l', 0, b'l', 0, b'o', 0, b' ', 0, b'W', 0, b'o', 0,
                b'r', 0, b'l', 0, b'd', 0
            ]
        );
    }

    #[test]
    fn too_small_buffer() {
        let mut buf = Aligned::<A4, _>([0u8; 1]);
        assert!(StringDescriptorWritter::new(&mut buf).is_err());
    }

    #[test]
    fn too_big_buffer() {
        let mut buf = Aligned::<A4, _>([0u8; 258]);
        assert!(StringDescriptorWritter::new(&mut buf).is_err());
    }

    #[test]
    fn too_small_to_fit() {
        let mut buf = Aligned::<A4, _>([0u8; 12]);
        let mut writter = StringDescriptorWritter::new(&mut buf).unwrap();
        uwrite!(writter, "Hello").unwrap();
        assert!(uwrite!(writter, " ").is_err());
    }

    #[test]
    fn non_ascii_char() {
        let mut buf = Aligned::<A4, _>([0u8; 20]);
        let mut writter = StringDescriptorWritter::new(&mut buf).unwrap();
        assert!(uwrite!(writter, "Héllö").is_err());
    }
}

pub struct DfuFunctionalDescriptor {
    /// New firmware can be received from the host
    pub can_download: bool,
    /// Current firmware can be sent back to the host
    pub can_upload: bool,
    /// Device can still communicate with the host after the manifestation phase.
    pub manifestation_tolerant: bool,
    /// Device will detach from the USB bus autonomously after receiving
    /// DFU_DETACH; the host does not need to explicitly issue a bus reset.
    pub will_detach: bool,
    /// Timeout the device will wait to be reset by host after receiving DFU_DETACH.
    pub detach_timeout_ms: u16,
    /// The number of bytes the device can receive per control request.
    pub transfer_size: u16,
}
impl DfuFunctionalDescriptor {
    pub const fn total_size(&self) -> usize {
        9
    }
    pub const fn serialize(&self, dest: &mut [u8], offset: usize) {
        const fn bit(index: u8, val: bool) -> u8 {
            (if val { 1 } else { 0 }) << index
        }
        const fn copy_u16(dest: &mut [u8], index: usize, val: u16) {
            let bytes = val.to_le_bytes();
            dest[index] = bytes[0];
            dest[index + 1] = bytes[1];
        }
        // sizeof descriptor
        dest[offset] = 9;
        // bDescriptorType = DFU Functional
        dest[offset + 1] = 0x21;
        // bmAttributes
        dest[offset + 2] = bit(0, self.can_download)
            | bit(1, self.can_upload)
            | bit(2, self.manifestation_tolerant)
            | bit(3, self.will_detach);
        copy_u16(dest, offset + 3, self.detach_timeout_ms);
        copy_u16(dest, offset + 5, self.transfer_size);
        // bcdDFUVersion
        copy_u16(dest, offset + 7, 0x0100);
    }
}

pub struct RawFunctionalDescriptor {
    pub descriptor_type: u8,
    pub content: &'static [u8],
}
impl RawFunctionalDescriptor {
    pub const fn total_size(&self) -> usize {
        self.content.len() + 2
    }
    pub const fn serialize(&self, dest: &mut [u8], offset: usize) {
        dest[offset] = self.total_size() as u8;
        dest[offset + 1] = self.descriptor_type;
        let mut i = 0;
        while i < self.content.len() {
            dest[offset + 2 + i] = self.content[i];
            i += 1;
        }
    }
}

// This should be a trait, but traits can't be used from const functions :(
pub enum FunctionalDescriptor {
    Dfu(DfuFunctionalDescriptor),
    Raw(RawFunctionalDescriptor),
}

impl FunctionalDescriptor {
    pub const fn raw(descriptor_type: u8, content: &'static [u8]) -> Self {
        Self::Raw(RawFunctionalDescriptor {
            descriptor_type,
            content,
        })
    }
    pub const fn total_size(&self) -> usize {
        match self {
            Self::Dfu(dfu) => dfu.total_size(),
            Self::Raw(raw) => raw.total_size(),
        }
    }
    #[allow(clippy::identity_op)]
    pub const fn serialize(&self, dest: &mut [u8], offset: usize) {
        assert!(offset + self.total_size() <= dest.len());
        match self {
            Self::Dfu(dfu) => dfu.serialize(dest, offset),
            Self::Raw(raw) => raw.serialize(dest, offset),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const INTERFACE_NAME_HANDLE: StringHandle = StringHandle(5);

    const CONFIG_DESC: ConfigDescriptor = ConfigDescriptor {
        configuration_value: 1,
        max_power: 250,
        self_powered: false,
        remote_wakeup: false,
        interfaces: &[InterfaceDescriptor {
            name: INTERFACE_NAME_HANDLE,
            interface_number: 0,
            alternate_setting: 0,
            interface_class: 0xff,
            interface_sub_class: 0xff,
            interface_protocol: 0xff,
            func_descs: &[],
            endpoints: &[
                EndpointDescriptor {
                    direction: Direction::DeviceToHost,
                    endpoint_num: 1,
                    transfer_type: TransferType::Bulk,
                    max_packet_size: 64,
                    interval: 0,
                },
                EndpointDescriptor {
                    direction: Direction::HostToDevice,
                    endpoint_num: 2,
                    transfer_type: TransferType::Bulk,
                    max_packet_size: 64,
                    interval: 0,
                },
            ],
        }],
    };
    const CONFIG_DESC_RAW: [u8; CONFIG_DESC.total_size()] = CONFIG_DESC.serialize();

    #[test]
    fn test_config_desc() {
        assert_eq!(
            &CONFIG_DESC_RAW,
            &[
                0x09, 0x02, 0x20, 0x00, 0x01, 0x01, 0x00, 0x80, 0xfa, 0x09, 0x04, 0x00, 0x00, 0x02,
                0xff, 0xff, 0xff, 0x05, 0x07, 0x05, 0x81, 0x02, 0x40, 0x00, 0x00, 0x07, 0x05, 0x02,
                0x02, 0x40, 0x00, 0x00
            ]
        )
    }

    #[test]
    fn test_config_desc_dfu() {
        const CONFIG_DESC_DFU: ConfigDescriptor = ConfigDescriptor {
            configuration_value: 1,
            max_power: 250,
            self_powered: false,
            remote_wakeup: false,
            interfaces: &[InterfaceDescriptor {
                name: INTERFACE_NAME_HANDLE,
                interface_number: 0,
                alternate_setting: 0,
                interface_class: 0xfe,
                interface_sub_class: 0x01,
                interface_protocol: 0x02,
                func_descs: &[FunctionalDescriptor::Dfu(DfuFunctionalDescriptor {
                    can_download: true,
                    can_upload: false,
                    manifestation_tolerant: true,
                    will_detach: true,
                    transfer_size: 2048,
                    detach_timeout_ms: 8000,
                })],
                endpoints: &[],
            }],
        };
        const CONFIG_DESC_BYTES: [u8; CONFIG_DESC_DFU.total_size()] = CONFIG_DESC_DFU.serialize();

        assert_eq!(
            &CONFIG_DESC_BYTES,
            &[
                0x09, 0x02, 0x1b, 0x00, 0x01, 0x01, 0x00, 0x80, 0xfa, 0x09, 0x04, 0x00, 0x00, 0x00,
                0xfe, 0x01, 0x02, 0x05, 0x09, 0x21, 0x0d, 0x40, 0x1f, 0x00, 0x08, 0x00, 0x01
            ]
        )
    }

    #[test]
    fn test_string_descriptor() {
        use core::ops::Deref;
        const USB_VENDOR: StringDescriptorRef = string_descriptor!("Mutask").as_ref();
        assert_eq!(
            USB_VENDOR.as_bytes().deref(),
            &[14, 3, b'M', 0, b'u', 0, b't', 0, b'a', 0, b's', 0, b'k', 0,]
        );
    }

    #[test]
    pub fn test_hex_utf16_descriptor() {
        let mut buf = [0_u8; 80];
        let len = hex_utf16_descriptor(&mut buf, &[0xab, 0x1c, 0xd2, 0xe3, 0x4f, 0x56, 0x78, 0x90])
            .unwrap();
        assert_eq!(
            [
                34, 3, b'a', 0, b'b', 0, b'1', 0, b'c', 0, b'd', 0, b'2', 0, b'e', 0, b'3', 0,
                b'4', 0, b'f', 0, b'5', 0, b'6', 0, b'7', 0, b'8', 0, b'9', 0, b'0', 0
            ],
            &buf[..len]
        );

        // empty string; tight fit
        let mut buf = [0_u8; 2];
        let len = hex_utf16_descriptor(&mut buf, b"").unwrap();
        assert_eq!(&[2, 3], &buf[..len]);

        // 1 byte; tight fit
        let mut buf = [0_u8; 6];
        let len = hex_utf16_descriptor(&mut buf, &[0xca]).unwrap();
        assert_eq!(&[6, 3, b'c', 0, b'a', 0], &buf[..len]);

        // 2 bytes; tight fit
        let mut buf = [0_u8; 10];
        let len = hex_utf16_descriptor(&mut buf, &[0xca, 0xfe]).unwrap();
        assert_eq!(&[10, 3, b'c', 0, b'a', 0, b'f', 0, b'e', 0], &buf[..len]);

        // too small to fit descriptor
        let mut buf = [0_u8; 1];
        hex_utf16_descriptor(&mut buf, b"").unwrap_err();

        // too small to fit 1 byte hex string
        let mut buf = [0_u8; 5];
        hex_utf16_descriptor(&mut buf, b"H").unwrap_err();

        // too small to fit 2 byte hex string
        let mut buf = [0_u8; 9];
        hex_utf16_descriptor(&mut buf, b"Hi").unwrap_err();

        // length too big to fit in length field
        let mut buf = [0_u8; 258];
        hex_utf16_descriptor(&mut buf, &[0x42_u8; 64]).unwrap_err();
    }
}
