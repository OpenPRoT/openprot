use crate::error::*;
use util_error::ErrorCode;

#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct PersoTlvType(u8);

#[allow(non_upper_case_globals)]
impl PersoTlvType {
    pub const X509Tbs: PersoTlvType = PersoTlvType(0);
    pub const X509Cert: PersoTlvType = PersoTlvType(1);
    pub const DevSeed: PersoTlvType = PersoTlvType(2);
    pub const CwtCert: PersoTlvType = PersoTlvType(3);
    pub const WasTbsHmac: PersoTlvType = PersoTlvType(4);
    pub const DeviceId: PersoTlvType = PersoTlvType(5);
    pub const GenericSeed: PersoTlvType = PersoTlvType(6);
    pub const PersoSha256Hash: PersoTlvType = PersoTlvType(7);
}

pub struct PersoCertificate<'a> {
    pub obj_type: PersoTlvType,
    pub obj_size: usize,
    pub name: &'a str,
    pub certificate: &'a [u8],
}

impl PersoCertificate<'_> {
    pub fn from_bytes<'a>(data: &'a [u8]) -> Result<(PersoCertificate<'a>, &'a [u8]), ErrorCode> {
        let type_size = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let namelen_certsz = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let rest = &data[4..];

        if type_size == 0xFFFF || type_size == 0 {
            return Err(EG_ERROR_CERT_NOT_FOUND);
        }
        // Really should check the object type and return an error if not a certificate.
        let obj_type = PersoTlvType((type_size >> 12) as u8);
        let obj_size = (type_size & 0x0FFF) as usize;
        let namelen = (namelen_certsz >> 12) as usize;
        let certsz = (namelen_certsz & 0x0FFF) as usize;

        let name = core::str::from_utf8(&rest[..namelen]).map_err(|_| EG_ERROR_CERT_BAD_NAME)?;
        let certificate = &rest[namelen..namelen + certsz];
        let end = (obj_size + 7) & !7;
        Ok((
            PersoCertificate {
                obj_type,
                obj_size,
                name,
                certificate,
            },
            &rest[end..],
        ))
    }
}
