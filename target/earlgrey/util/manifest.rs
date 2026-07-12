// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Target-side Rust `zerocopy` structures and constants for OpenTitan manifests.
//!
//! This module provides a Rust translation of the structures defined in
//! `sw/device/silicon_creator/lib/manifest.h` and related headers in the OpenTitan
//! repository. These structures are used to parse and validate boot stage images
//! stored in flash.
//!
//! Note: All structures assume 4-byte (word) alignment in memory, which is the
//! standard alignment for flash images on OpenTitan.

use core::mem::size_of;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::rom_error::RomError;
use crate::tags::{HardenedBool, ManifestIdentifier};

/// Size of the manifest structure in bytes.
pub const CHIP_MANIFEST_SIZE: usize = 1024;

/// Number of entries in the manifest extensions table.
pub const CHIP_MANIFEST_EXT_TABLE_ENTRY_COUNT: usize = 15;

/// Alias for `CHIP_MANIFEST_EXT_TABLE_ENTRY_COUNT` used in table sizing.
pub const MANIFEST_EXT_TABLE_COUNT: usize = CHIP_MANIFEST_EXT_TABLE_ENTRY_COUNT;

/// Value to use for unselected usage constraint words.
pub const MANIFEST_USAGE_CONSTRAINT_UNSELECTED_WORD_VAL: u32 = 0xA5A5A5A5;

/// Manifest format major and minor versions.
pub const MANIFEST_VERSION_MAJOR_1: u16 = 0x71c3;
pub const MANIFEST_VERSION_MAJOR_2: u16 = 0x0002;
pub const MANIFEST_VERSION_MINOR_1: u16 = 0x6c47;

/// Extension identifiers.
pub const MANIFEST_EXT_ID_SPX_KEY: u32 = 0x94ac01ec;
pub const MANIFEST_EXT_ID_SPX_SIGNATURE: u32 = 0xad77f84a;
pub const MANIFEST_EXT_ID_SECVER_WRITE: u32 = 0x3f086a41;
pub const MANIFEST_EXT_ID_ISFB: u32 = 0x42465349;
pub const MANIFEST_EXT_ID_ISFB_ERASE: u32 = 0x45465349;
pub const MANIFEST_EXT_ID_OWNER_TRANSFER_BLOB: u32 = 0x4254574f;

/// Extension names (ASCII tags for debugging).
pub const MANIFEST_EXT_NAME_SPX_KEY: u32 = 0x30545845; // 'EXT0'
pub const MANIFEST_EXT_NAME_SPX_SIGNATURE: u32 = 0x31545845; // 'EXT1'
pub const MANIFEST_EXT_NAME_SECVER_WRITE: u32 = 0x56434553; // 'SECV'
pub const MANIFEST_EXT_NAME_ISFB: u32 = 0x42465349; // 'ISFB'
pub const MANIFEST_EXT_NAME_ISFB_ERASE: u32 = 0x45465349; // 'ISFE'
pub const MANIFEST_EXT_NAME_OWNER_TRANSFER_BLOB: u32 = 0x4254574f; // 'OWTB'

/// `selector_bits` bit indices for usage constraints fields.
pub const MANIFEST_SELECTOR_BIT_DEVICE_ID_FIRST: u32 = 0;
pub const MANIFEST_SELECTOR_BIT_DEVICE_ID_LAST: u32 = 7;
pub const MANIFEST_SELECTOR_BIT_MANUF_STATE_CREATOR: u32 = 8;
pub const MANIFEST_SELECTOR_BIT_MANUF_STATE_OWNER: u32 = 9;
pub const MANIFEST_SELECTOR_BIT_LIFE_CYCLE_STATE: u32 = 10;

/// Buffer of reserved bytes, padded with `0xa5` by default to match OpenTitan
/// image layout specifications.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, PartialEq, Eq)]
pub struct ReservedBuffer<const COUNT: usize> {
    a: [u8; COUNT],
}

impl<const COUNT: usize> Default for ReservedBuffer<COUNT> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            a: core::array::from_fn(|_| 0xa5),
        }
    }
}

/// Manifest format version (major and minor).
#[repr(C)]
#[derive(
    Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, Debug, PartialEq, Eq,
)]
pub struct ManifestVersion {
    pub minor: u16,
    pub major: u16,
}

/// Manifest timestamp (Unix epoch seconds).
#[repr(C)]
#[derive(
    Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, Debug, PartialEq, Eq,
)]
pub struct Timestamp {
    pub timestamp_low: u32,
    pub timestamp_high: u32,
}

/// Binding value used by the key manager to derive secret values.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct KeymgrBindingValue {
    pub data: [u32; 8],
}

/// The 256-bit device identifier stored in the `HW_CFG0` partition in OTP.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct LifecycleDeviceId {
    pub device_id: [u32; 8],
}

/// Holds an ECDSA-P256 public key.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct EcdsaP256PublicKey {
    pub x: [u32; 8],
    pub y: [u32; 8],
}

/// Holds an ECDSA-P256 signature.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct EcdsaP256Signature {
    pub r: [u32; 8],
    pub s: [u32; 8],
}

/// Holds an SPX (SPHINCS+) signature.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, PartialEq, Eq)]
pub struct SigverifySpxSignature {
    pub data: [u32; 1964],
}

impl Default for SigverifySpxSignature {
    #[inline(always)]
    fn default() -> Self {
        Self { data: [0; 1964] }
    }
}

/// Holds an SPX (SPHINCS+) public key.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct SigverifySpxKey {
    pub data: [u32; 8],
}

/// Usage constraints applied to the boot stage image.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, PartialEq, Eq)]
pub struct ManifestUsageConstraints {
    pub selector_bits: u32,
    pub device_id: LifecycleDeviceId,
    pub manuf_state_creator: u32,
    pub manuf_state_owner: u32,
    pub life_cycle_state: u32,
}

impl Default for ManifestUsageConstraints {
    #[inline(always)]
    fn default() -> Self {
        Self {
            selector_bits: 0,
            device_id: LifecycleDeviceId {
                device_id: [MANIFEST_USAGE_CONSTRAINT_UNSELECTED_WORD_VAL; 8],
            },
            manuf_state_creator: MANIFEST_USAGE_CONSTRAINT_UNSELECTED_WORD_VAL,
            manuf_state_owner: MANIFEST_USAGE_CONSTRAINT_UNSELECTED_WORD_VAL,
            life_cycle_state: MANIFEST_USAGE_CONSTRAINT_UNSELECTED_WORD_VAL,
        }
    }
}

/// An entry in the manifest extensions table.
#[repr(C)]
#[derive(
    Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, Debug, PartialEq, Eq,
)]
pub struct ManifestExtTableEntry {
    pub identifier: u32,
    pub offset: u32,
}

/// The table of manifest extensions.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct ManifestExtTable {
    pub entries: [ManifestExtTableEntry; MANIFEST_EXT_TABLE_COUNT],
}

/// The common header for all manifest extensions.
#[repr(C)]
#[derive(
    Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, Debug, PartialEq, Eq,
)]
pub struct ManifestExtHeader {
    pub identifier: u32,
    pub name: u32,
}

/// Manifest extension: SPX public key.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct ManifestExtSpxKey {
    pub header: ManifestExtHeader,
    pub key: SigverifySpxKey,
}

/// Manifest extension: SPX signature.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct ManifestExtSpxSignature {
    pub header: ManifestExtHeader,
    pub signature: SigverifySpxSignature,
}

/// Manifest extension: Security Version Write enable flag.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, PartialEq, Eq)]
pub struct ManifestExtSecVerWrite {
    pub header: ManifestExtHeader,
    pub write: HardenedBool,
}

/// Manifest extension: ISFB Erase Policy enable flag.
#[repr(C)]
#[derive(Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, PartialEq, Eq)]
pub struct ManifestExtIsfbErase {
    pub header: ManifestExtHeader,
    pub erase_allowed: HardenedBool,
}

/// Integrator Specific Firmware Binding (ISFB) product expression.
#[repr(C)]
#[derive(
    Clone, Copy, Immutable, IntoBytes, FromBytes, KnownLayout, Default, Debug, PartialEq, Eq,
)]
pub struct ManifestExtProductExpr {
    pub mask: u32,
    pub value: u32,
}

/// Manifest extension: Integrator Specific Firmware Binding (ISFB).
///
/// This structure covers the FIXED part of the extension. Use `product_exprs`
/// or `parse_full` to access the variable-length `product_expr` array that follows it.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, Default, PartialEq, Eq)]
pub struct ManifestExtIsfb {
    pub header: ManifestExtHeader,
    pub strike_mask: [u32; 4],
    pub product_expr_count: u32,
}

impl ManifestExtIsfb {
    /// Gets a slice of the `ManifestExtProductExpr` array following the fixed part,
    /// given the remainder of the byte slice for this extension.
    pub fn product_exprs<'a>(
        &self,
        remainder: &'a [u8],
    ) -> Option<(&'a [ManifestExtProductExpr], &'a [u8])> {
        let count = self.product_expr_count as usize;
        <[ManifestExtProductExpr]>::ref_from_prefix_with_elems(remainder, count).ok()
    }

    /// Parses the fixed part and the variable-length product expression array
    /// from a single byte slice sized to this extension.
    pub fn parse_full(ext_bytes: &[u8]) -> Option<(&Self, &[ManifestExtProductExpr], &[u8])> {
        let (fixed, remainder) = Self::ref_from_prefix(ext_bytes).ok()?;
        let count = fixed.product_expr_count as usize;
        let (exprs, suffix) =
            <[ManifestExtProductExpr]>::ref_from_prefix_with_elems(remainder, count).ok()?;
        Some((fixed, exprs, suffix))
    }
}

/// Manifest extension: Owner Transfer Blob.
///
/// This structure covers the FIXED part of the extension. The detached signature
/// (if present) immediately follows this structure in flash.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout, PartialEq, Eq)]
pub struct ManifestExtOwnerTransferBlob {
    pub header: ManifestExtHeader,
    pub owner_block: [u8; 2048],
}

impl ManifestExtOwnerTransferBlob {
    /// Parses the fixed part and returns the remaining bytes as the detached signature
    /// (assuming `ext_bytes` is bounded to this extension's actual length).
    pub fn parse_full(ext_bytes: &[u8]) -> Option<(&Self, &[u8])> {
        let (fixed, remainder) = Self::ref_from_prefix(ext_bytes).ok()?;
        Some((fixed, remainder))
    }
}

/// The Manifest structure for boot stage images stored in flash.
#[repr(C)]
#[derive(Clone, Immutable, IntoBytes, FromBytes, KnownLayout)]
pub struct Manifest {
    /// ECDSA P256 signature of the image.
    pub ecdsa_signature: EcdsaP256Signature,
    /// Reserved space for signature padding.
    pub reserved_signature: ReservedBuffer<160>,
    /// Reserved space (unsigned region).
    pub reserved_unsigned: ReservedBuffer<160>,
    /// Usage constraints applied to this image.
    pub usage_constraints: ManifestUsageConstraints,
    /// Signer's ECDSA NIST P256 public key.
    pub ecdsa_public_key: EcdsaP256PublicKey,
    /// Reserved space for public key padding.
    pub reserved_public_key: ReservedBuffer<160>,
    /// Reserved space.
    pub reserved: ReservedBuffer<156>,
    /// Address where the manifest is expected to be loaded.
    pub manifest_base_address: u32,
    /// Address translation (hardened boolean).
    pub address_translation: u32,
    /// Manifest identifier ('OTRE' or 'OTB0').
    pub identifier: ManifestIdentifier,
    /// Manifest format major and minor version.
    pub manifest_version: ManifestVersion,
    /// Offset of the end of the signed region relative to the start of the manifest.
    pub signed_region_end: u32,
    /// Length of the image including the manifest in bytes.
    pub length: u32,
    /// Image major version.
    pub version_major: u32,
    /// Image minor version.
    pub version_minor: u32,
    /// Security version of the image for anti-rollback.
    pub security_version: u32,
    /// Image timestamp.
    pub timestamp: Timestamp,
    /// Binding value used by key manager.
    pub binding_value: KeymgrBindingValue,
    /// Maximum allowed version for keys at the next boot stage.
    pub max_key_version: u32,
    /// Offset of the start of the executable region in bytes.
    pub code_start: u32,
    /// Offset of the end of the executable region (exclusive) in bytes.
    pub code_end: u32,
    /// Offset of the entry point in bytes.
    pub entry_point: u32,
    /// Table of manifest extensions.
    pub extensions: ManifestExtTable,
}

impl Manifest {
    /// Validates the structure and offsets of the manifest.
    ///
    /// This function implements the checks from `manifest_check` in OpenTitan's
    /// `manifest.h`.
    /// TODO: consider evolving this function to add addtional heuristics.
    pub fn check(&self) -> Result<(), RomError> {
        if self.manifest_version.major != MANIFEST_VERSION_MAJOR_2 {
            return Err(RomError::ManifestBadVersionMajor);
        }

        if self.signed_region_end > self.length {
            return Err(RomError::ManifestBadSignedRegion);
        }

        if self.code_start >= self.code_end
            || (self.code_start as usize) < size_of::<Self>()
            || self.code_end > self.signed_region_end
            || (self.code_start & 0x3) != 0
            || (self.code_end & 0x3) != 0
        {
            return Err(RomError::ManifestBadCodeRegion);
        }

        if self.entry_point < self.code_start
            || self.entry_point >= self.code_end
            || (self.entry_point & 0x3) != 0
        {
            return Err(RomError::ManifestBadEntryPoint);
        }

        for i in 0..MANIFEST_EXT_TABLE_COUNT {
            if (self.extensions.entries[i].offset & 0x3) != 0 {
                return Err(RomError::ManifestBadExtension);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn test_manifest_layout() {
        assert_eq!(offset_of!(Manifest, ecdsa_signature), 0);
        assert_eq!(offset_of!(Manifest, reserved_signature), 64);
        assert_eq!(offset_of!(Manifest, reserved_unsigned), 224);
        assert_eq!(offset_of!(Manifest, usage_constraints), 384);
        assert_eq!(offset_of!(Manifest, ecdsa_public_key), 432);
        assert_eq!(offset_of!(Manifest, reserved_public_key), 496);
        assert_eq!(offset_of!(Manifest, reserved), 656);
        assert_eq!(offset_of!(Manifest, manifest_base_address), 812);
        assert_eq!(offset_of!(Manifest, address_translation), 816);
        assert_eq!(offset_of!(Manifest, identifier), 820);
        assert_eq!(offset_of!(Manifest, manifest_version), 824);
        assert_eq!(offset_of!(Manifest, signed_region_end), 828);
        assert_eq!(offset_of!(Manifest, length), 832);
        assert_eq!(offset_of!(Manifest, version_major), 836);
        assert_eq!(offset_of!(Manifest, version_minor), 840);
        assert_eq!(offset_of!(Manifest, security_version), 844);
        assert_eq!(offset_of!(Manifest, timestamp), 848);
        assert_eq!(offset_of!(Manifest, binding_value), 856);
        assert_eq!(offset_of!(Manifest, max_key_version), 888);
        assert_eq!(offset_of!(Manifest, code_start), 892);
        assert_eq!(offset_of!(Manifest, code_end), 896);
        assert_eq!(offset_of!(Manifest, entry_point), 900);
        assert_eq!(offset_of!(Manifest, extensions), 904);
        assert_eq!(size_of::<Manifest>(), CHIP_MANIFEST_SIZE);
    }

    #[test]
    fn test_isfb_layout() {
        assert_eq!(offset_of!(ManifestExtIsfb, header), 0);
        assert_eq!(offset_of!(ManifestExtIsfb, strike_mask), 8);
        assert_eq!(offset_of!(ManifestExtIsfb, product_expr_count), 24);
        assert_eq!(size_of::<ManifestExtIsfb>(), 28);
    }

    #[test]
    fn test_owner_transfer_blob_layout() {
        assert_eq!(offset_of!(ManifestExtOwnerTransferBlob, header), 0);
        assert_eq!(offset_of!(ManifestExtOwnerTransferBlob, owner_block), 8);
        assert_eq!(size_of::<ManifestExtOwnerTransferBlob>(), 2056);
    }
}
