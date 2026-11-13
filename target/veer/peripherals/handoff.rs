// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Reader for the handoff table that the caliptra-mcu-sw ROM leaves for the
//! runtime image.
//!
//! The ROM reserves the last 1 KiB of DCCM for the table (`handoff_offset` in
//! the caliptra-mcu-sw memory map) and writes it during a cold boot, before it
//! loads the runtime image. The layout is `HandoffData` in
//! `romtime/src/handoff.rs` of caliptra-mcu-sw:
//!
//! | Offset | Size | Contents                                             |
//! |-------:|-----:|------------------------------------------------------|
//! |      0 |   64 | ROM table: marker, version, boot type, capabilities  |
//! |     64 |   64 | Runtime table, reserved for the runtime image        |
//! |    128 |  132 | Stable owner key: encrypted CMK and a validity marker |
//!
//! The major version changes when a field moves. The minor version increases
//! when a field is added, so each accessor checks that the ROM's minor version
//! has the field. This reader knows the fields of version 1.3.
//!
//! [`Handoff::at`] validates the ROM table and refers to it where it is in
//! DCCM. The kernel can access DCCM. A service needs a mapping of the table's
//! first [`ROM_TABLE_SIZE`] bytes in its system config. [`RomTable`] is
//! `Copy`, so code that needs the table after DCCM is put to another use
//! copies it out of [`Handoff::rom_table`] and passes the copy to
//! [`Handoff::new`].
//!
//! Bytes 8 to 19 of the ROM table hold the OCP LOCK HEK state when the ROM is
//! built with its `ocp-lock` feature. This reader does not decode them.

#![cfg_attr(not(test), no_std)]

use core::mem::{offset_of, size_of};
use core::ptr::with_exposed_provenance;

/// Value of the marker at the start of a handoff table: "MCUH".
pub const MARKER: u32 = 0x4855_434d;

/// Major version of the layout that this reader knows.
pub const MAJOR_VERSION: u16 = 1;

/// Size of the ROM table at the start of the handoff region.
pub const ROM_TABLE_SIZE: usize = size_of::<RomTable>();

/// Offset of the stable owner key from the start of the handoff region.
pub const STABLE_OWNER_KEY_OFFSET: usize = 128;

/// Size of the stable owner key table: the CMK and its validity marker.
pub const STABLE_OWNER_KEY_SIZE: usize = size_of::<StableOwnerKeyTable>();

/// Size of an encrypted Caliptra Cryptographic Manager key (CMK).
pub const CMK_SIZE: usize = 128;

/// Minor version that added the stable owner key.
const STABLE_OWNER_KEY_MINOR_VERSION: u16 = 1;

/// Minor version that added the firmware boot type.
const FIRMWARE_BOOT_TYPE_MINOR_VERSION: u16 = 2;

/// Minor version that added the ROM capabilities.
const ROM_CAPABILITIES_MINOR_VERSION: u16 = 3;

/// Value of the stable owner key's validity marker: "SOKV".
const STABLE_OWNER_KEY_VALID: u32 = 0x564b_4f53;

/// The ROM table as the ROM lays it out.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RomTable {
    /// [`MARKER`] when the ROM has written the table.
    pub marker: u32,
    pub major_version: u16,
    pub minor_version: u16,
    /// OCP LOCK HEK state, or zero when the ROM has no OCP LOCK support.
    pub ocp_lock: [u32; 3],
    /// A [`FirmwareBootType`] value.
    pub firmware_boot_type: u8,
    pub reserved: [u8; 3],
    /// [`RomCapabilities`] bits.
    pub rom_capabilities: u32,
    pub padding: [u8; 36],
}

const _: () = assert!(size_of::<RomTable>() == 64);
const _: () = assert!(offset_of!(RomTable, ocp_lock) == 8);
const _: () = assert!(offset_of!(RomTable, firmware_boot_type) == 20);
const _: () = assert!(offset_of!(RomTable, rom_capabilities) == 24);

/// The stable owner key as the ROM lays it out.
#[repr(C)]
struct StableOwnerKeyTable {
    cmk: [u8; CMK_SIZE],
    /// The ROM writes [`STABLE_OWNER_KEY_VALID`] here after it has written
    /// the whole CMK.
    valid_marker: u32,
}

const _: () = assert!(size_of::<StableOwnerKeyTable>() == 132);

/// Where the ROM got the runtime image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirmwareBootType {
    /// The ROM did not record the source.
    Unknown,
    /// The ROM loaded the image from flash.
    Flash,
    /// The image was streamed through the OCP recovery interface.
    Streaming,
    /// The ROM loaded the image over the network.
    Network,
}

impl FirmwareBootType {
    /// A name for logs.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Flash => "flash",
            Self::Streaming => "streaming",
            Self::Network => "network",
        }
    }
}

bitflags::bitflags! {
    /// What the ROM image supports. A set bit does not mean that this boot
    /// used the feature.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct RomCapabilities: u32 {
        /// Streaming boot over I3C.
        const STREAMING_BOOT_I3C = 1 << 0;
        /// Boot from flash.
        const FLASH_BOOT = 1 << 1;
        /// Boot over the network.
        const NETWORK_BOOT = 1 << 2;
        /// OCP LOCK key management.
        const OCP_LOCK = 1 << 3;
        /// Device Ownership Transfer (DOT) firmware manifests.
        const FW_MANIFEST_DOT = 1 << 4;
        /// Component SVN manifests.
        const COMPONENT_SVN_MANIFEST = 1 << 5;
        /// The ROM derives a stable owner key and hands it off.
        const STABLE_OWNER_KEY = 1 << 6;
        /// DOT blob authentication during boot.
        const DOT_BOOT = 1 << 7;
        /// A recovery path for the DOT locked state.
        const DOT_LOCKED_RECOVERY = 1 << 8;
        /// DOT recovery over I3C.
        const I3C_DOT_RECOVERY = 1 << 9;
    }
}

/// Why there is no usable handoff table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The table does not start with [`MARKER`]: the ROM has not written it.
    NoTable,
    /// The table has a major version other than [`MAJOR_VERSION`].
    UnsupportedVersion(u16),
}

/// Why the handoff table has no firmware boot type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootTypeError {
    /// The ROM's handoff version does not have the field.
    Unsupported,
    /// The field holds a value that is not a [`FirmwareBootType`].
    Invalid(u8),
}

/// A ROM table with a valid marker and major version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handoff<'a> {
    rom: &'a RomTable,
}

impl Handoff<'static> {
    /// Refers to the ROM table at the start of the handoff region.
    ///
    /// # Safety
    ///
    /// `base` must be the address of the handoff region and a multiple of 4.
    /// The first [`ROM_TABLE_SIZE`] bytes of the region must stay readable by
    /// the caller, and no code may write them from this call on.
    pub unsafe fn at(base: usize) -> Result<Self, Error> {
        let table = with_exposed_provenance::<RomTable>(base);
        // SAFETY: the caller guarantees that the table is aligned, stays
        // readable and is not written again. Every bit pattern is a valid
        // `RomTable`.
        Self::new(unsafe { &*table })
    }
}

impl<'a> Handoff<'a> {
    /// Validates a ROM table.
    pub const fn new(rom: &'a RomTable) -> Result<Self, Error> {
        if rom.marker != MARKER {
            return Err(Error::NoTable);
        }
        if rom.major_version != MAJOR_VERSION {
            return Err(Error::UnsupportedVersion(rom.major_version));
        }
        Ok(Self { rom })
    }

    /// The table as the ROM wrote it.
    pub const fn rom_table(&self) -> &'a RomTable {
        self.rom
    }

    /// Major version of the table, which is [`MAJOR_VERSION`].
    pub const fn major_version(&self) -> u16 {
        self.rom.major_version
    }

    /// Minor version of the table.
    pub const fn minor_version(&self) -> u16 {
        self.rom.minor_version
    }

    /// Where the ROM got the runtime image.
    pub const fn firmware_boot_type(&self) -> Result<FirmwareBootType, BootTypeError> {
        if self.rom.minor_version < FIRMWARE_BOOT_TYPE_MINOR_VERSION {
            return Err(BootTypeError::Unsupported);
        }
        match self.rom.firmware_boot_type {
            0 => Ok(FirmwareBootType::Unknown),
            1 => Ok(FirmwareBootType::Flash),
            2 => Ok(FirmwareBootType::Streaming),
            3 => Ok(FirmwareBootType::Network),
            other => Err(BootTypeError::Invalid(other)),
        }
    }

    /// What the ROM image supports, or `None` if the ROM's handoff version
    /// does not have the field. Bits that [`RomCapabilities`] does not name
    /// are kept.
    pub const fn rom_capabilities(&self) -> Option<RomCapabilities> {
        if self.rom.minor_version < ROM_CAPABILITIES_MINOR_VERSION {
            return None;
        }
        Some(RomCapabilities::from_bits_retain(self.rom.rom_capabilities))
    }

    /// Refers to the stable owner key in the handoff region: an encrypted CMK
    /// that only the Caliptra core can use. Returns `None` if the ROM's
    /// handoff version does not have the key or the ROM did not write one.
    ///
    /// # Safety
    ///
    /// `base` must be the address of the handoff region that holds this
    /// table, and a multiple of 4. The [`STABLE_OWNER_KEY_SIZE`] bytes at
    /// [`STABLE_OWNER_KEY_OFFSET`] in the region must stay readable by the
    /// caller, and no code may write them from this call on.
    pub unsafe fn stable_owner_key(&self, base: usize) -> Option<&'static [u8; CMK_SIZE]> {
        if self.rom.minor_version < STABLE_OWNER_KEY_MINOR_VERSION {
            return None;
        }
        let table = with_exposed_provenance::<StableOwnerKeyTable>(
            base.wrapping_add(STABLE_OWNER_KEY_OFFSET),
        );
        // SAFETY: the caller guarantees that the table stays readable and
        // is not written again. `base` and the offset are multiples of 4.
        // Every bit pattern is a valid `StableOwnerKeyTable`.
        let table: &'static StableOwnerKeyTable = unsafe { &*table };
        (table.valid_marker == STABLE_OWNER_KEY_VALID).then_some(&table.cmk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Size of `HandoffData` in caliptra-mcu-sw.
    const HANDOFF_SIZE: usize = 260;

    /// A handoff region. The tests write it at the byte offsets that the
    /// caliptra-mcu-sw handoff documentation lists.
    #[repr(C, align(4))]
    struct Region([u8; HANDOFF_SIZE]);

    impl Region {
        fn put(&mut self, offset: usize, bytes: &[u8]) {
            self.0[offset..offset + bytes.len()].copy_from_slice(bytes);
        }

        /// Writes a valid stable owner key.
        fn put_stable_owner_key(&mut self) {
            self.put(STABLE_OWNER_KEY_OFFSET, &[0x5a; CMK_SIZE]);
            self.put(
                STABLE_OWNER_KEY_OFFSET + CMK_SIZE,
                &STABLE_OWNER_KEY_VALID.to_le_bytes(),
            );
        }

        fn base(&self) -> usize {
            (&raw const self.0).expose_provenance()
        }

        fn handoff(&'static self) -> Result<Handoff<'static>, Error> {
            // SAFETY: the region is aligned, and it stays readable and is
            // not written again because it is leaked behind a shared
            // reference.
            unsafe { Handoff::at(self.base()) }
        }

        fn stable_owner_key(&'static self) -> Option<&'static [u8; CMK_SIZE]> {
            let handoff = self.handoff().unwrap();
            // SAFETY: as for `handoff`, and the region holds the whole
            // stable owner key table.
            unsafe { handoff.stable_owner_key(self.base()) }
        }
    }

    /// A region as the ROM writes it at handoff version 1.3 for a streaming
    /// boot, without a stable owner key, and then changed by `edit`. The
    /// region is leaked, as the handoff region in DCCM lasts for the whole
    /// program.
    fn region(edit: impl FnOnce(&mut Region)) -> &'static Region {
        let mut region = Box::new(Region([0; HANDOFF_SIZE]));
        region.put(0, &MARKER.to_le_bytes());
        region.put(4, &1u16.to_le_bytes());
        region.put(6, &3u16.to_le_bytes());
        region.put(20, &[2]);
        region.put(24, &0x43u32.to_le_bytes());
        edit(&mut region);
        Box::leak(region)
    }

    #[test]
    fn reads_the_fields_at_the_documented_offsets() {
        let handoff = region(|_| {}).handoff().unwrap();
        assert_eq!(handoff.major_version(), 1);
        assert_eq!(handoff.minor_version(), 3);
        assert_eq!(
            handoff.firmware_boot_type(),
            Ok(FirmwareBootType::Streaming)
        );
        assert_eq!(
            handoff.rom_capabilities(),
            Some(
                RomCapabilities::STREAMING_BOOT_I3C
                    | RomCapabilities::FLASH_BOOT
                    | RomCapabilities::STABLE_OWNER_KEY
            )
        );
        assert_eq!(handoff.rom_table().ocp_lock, [0; 3]);
    }

    #[test]
    fn refers_to_the_table_in_the_region() {
        let region = region(|_| {});
        let handoff = region.handoff().unwrap();
        assert_eq!(
            (&raw const *handoff.rom_table()).addr(),
            (&raw const region.0).addr()
        );
    }

    #[test]
    fn a_copy_of_the_table_decodes_the_same() {
        let handoff = region(|_| {}).handoff().unwrap();
        let copy = *handoff.rom_table();
        let from_copy = Handoff::new(&copy).unwrap();
        assert_eq!(from_copy, handoff);
        assert_eq!(
            from_copy.firmware_boot_type(),
            Ok(FirmwareBootType::Streaming)
        );
    }

    #[test]
    fn rejects_a_region_without_the_marker() {
        let region = region(|r| r.put(0, &0u32.to_le_bytes()));
        assert_eq!(region.handoff(), Err(Error::NoTable));
    }

    #[test]
    fn rejects_another_major_version() {
        let region = region(|r| r.put(4, &2u16.to_le_bytes()));
        assert_eq!(region.handoff(), Err(Error::UnsupportedVersion(2)));
    }

    #[test]
    fn accepts_a_later_minor_version() {
        let handoff = region(|r| r.put(6, &9u16.to_le_bytes())).handoff().unwrap();
        assert_eq!(handoff.minor_version(), 9);
        assert_eq!(
            handoff.firmware_boot_type(),
            Ok(FirmwareBootType::Streaming)
        );
    }

    #[test]
    fn decodes_each_boot_type() {
        let cases = [
            (0, Ok(FirmwareBootType::Unknown)),
            (1, Ok(FirmwareBootType::Flash)),
            (2, Ok(FirmwareBootType::Streaming)),
            (3, Ok(FirmwareBootType::Network)),
            (4, Err(BootTypeError::Invalid(4))),
            (0xff, Err(BootTypeError::Invalid(0xff))),
        ];
        for (value, expected) in cases {
            let handoff = region(|r| r.put(20, &[value])).handoff().unwrap();
            assert_eq!(handoff.firmware_boot_type(), expected);
        }
    }

    #[test]
    fn boot_type_needs_minor_version_2() {
        let handoff = region(|r| r.put(6, &1u16.to_le_bytes())).handoff().unwrap();
        assert_eq!(
            handoff.firmware_boot_type(),
            Err(BootTypeError::Unsupported)
        );
    }

    #[test]
    fn rom_capabilities_need_minor_version_3() {
        let handoff = region(|r| r.put(6, &2u16.to_le_bytes())).handoff().unwrap();
        assert_eq!(handoff.rom_capabilities(), None);
        assert_eq!(
            handoff.firmware_boot_type(),
            Ok(FirmwareBootType::Streaming)
        );
    }

    #[test]
    fn rom_capabilities_keep_unnamed_bits() {
        let handoff = region(|r| r.put(24, &0x8000_0001u32.to_le_bytes()))
            .handoff()
            .unwrap();
        let capabilities = handoff.rom_capabilities().unwrap();
        assert!(capabilities.contains(RomCapabilities::STREAMING_BOOT_I3C));
        assert_eq!(capabilities.bits(), 0x8000_0001);
    }

    #[test]
    fn stable_owner_key_needs_its_marker() {
        assert_eq!(region(|_| {}).stable_owner_key(), None);

        let without_marker = region(|r| r.put(STABLE_OWNER_KEY_OFFSET, &[0x5a; CMK_SIZE]));
        assert_eq!(without_marker.stable_owner_key(), None);

        let with_marker = region(Region::put_stable_owner_key);
        assert_eq!(with_marker.stable_owner_key(), Some(&[0x5a; CMK_SIZE]));
    }

    #[test]
    fn stable_owner_key_needs_minor_version_1() {
        let region = region(|r| {
            r.put_stable_owner_key();
            r.put(6, &0u16.to_le_bytes());
        });
        assert_eq!(region.stable_owner_key(), None);
    }
}
