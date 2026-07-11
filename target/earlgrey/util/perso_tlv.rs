// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Personalization TLV (Type-Length-Value) parsing utilities.
//!
//! Personalization data is written to the flash info pages during manufacturing (provisioning).
//! It contains certificates, cryptographic seeds, and device identifiers.
//!
//! The TLV format is packed:
//! * 2 bytes: Type (4 bits) and Object Size (12 bits).
//! * 2 bytes: Name Length (4 bits) and Value Size (12 bits).
//! * Name (UTF-8 string of length `namelen`).
//! * Value (raw bytes of length `value_size`).
//! * Padded to 8-byte boundary.

use crate::error::*;
use util_error::ErrorCode;

/// The type identifier for a personalization TLV object.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct PersoTlvType(pub u8);

#[allow(non_upper_case_globals)]
impl PersoTlvType {
    /// X.509 To-Be-Signed (TBS) certificate data.
    pub const X509Tbs: PersoTlvType = PersoTlvType(0);
    /// Completed X.509 certificate.
    pub const X509Cert: PersoTlvType = PersoTlvType(1);
    /// Device cryptographic seed.
    pub const DevSeed: PersoTlvType = PersoTlvType(2);
    /// CWT (CBOR Web Token) certificate.
    pub const CwtCert: PersoTlvType = PersoTlvType(3);
    /// HMAC of the To-Be-Signed data.
    pub const WasTbsHmac: PersoTlvType = PersoTlvType(4);
    /// Device Identification Number (DIN).
    pub const DeviceId: PersoTlvType = PersoTlvType(5);
    /// Generic personalization seed.
    pub const GenericSeed: PersoTlvType = PersoTlvType(6);
    /// SHA256 hash of personalization config.
    pub const PersoSha256Hash: PersoTlvType = PersoTlvType(7);
}

/// A parsed representation of a personalization certificate/object.
pub struct PersoCertificate<'a> {
    /// The object type (e.g. X509Cert, CwtCert).
    pub obj_type: PersoTlvType,
    /// The overall size of the object payload in bytes.
    pub obj_size: usize,
    /// The name of the certificate (e.g., "UDS", "CDI").
    pub name: &'a str,
    /// The raw certificate or value bytes.
    pub certificate: &'a [u8],
}

impl<'a> PersoCertificate<'a> {
    /// Parses a single `PersoCertificate` from a byte slice.
    ///
    /// Returns the parsed certificate and the remaining slice starting at the
    /// next 8-byte aligned offset.
    ///
    /// Returns `Err(EG_ERROR_CERT_NOT_FOUND)` if the data is empty or invalid.
    pub fn from_bytes(data: &'a [u8]) -> Result<(PersoCertificate<'a>, &'a [u8]), ErrorCode> {
        if data.len() < 4 {
            return Err(EG_ERROR_CERT_NOT_FOUND);
        }
        let type_size = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let namelen_certsz = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let rest = &data[4..];

        if type_size == 0xFFFF || type_size == 0 {
            return Err(EG_ERROR_CERT_NOT_FOUND);
        }

        let obj_type = PersoTlvType((type_size >> 12) as u8);
        let obj_size = (type_size & 0x0FFF) as usize;
        let namelen = (namelen_certsz >> 12) as usize;
        let certsz = (namelen_certsz & 0x0FFF) as usize;

        let end_idx = namelen.checked_add(certsz).ok_or(EG_ERROR_CERT_NOT_FOUND)?;
        let name_bytes = rest.get(..namelen).ok_or(EG_ERROR_CERT_NOT_FOUND)?;
        let name = core::str::from_utf8(name_bytes).map_err(|_| EG_ERROR_CERT_BAD_NAME)?;
        let certificate = rest.get(namelen..end_idx).ok_or(EG_ERROR_CERT_NOT_FOUND)?;
        // TLV objects are padded to 8-byte boundary (64-bit alignment)
        let end = (obj_size + 7) & !7;

        if rest.len() < end {
            return Err(EG_ERROR_CERT_NOT_FOUND);
        }

        Ok((
            PersoCertificate {
                obj_type,
                obj_size,
                name,
                certificate,
            },
            &data[end..],
        ))
    }
}
