use ufmt::derive::uDebug;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::misc::UnalignedU64;
use crate::tags::{BootSlot, HardenedBool, OwnershipState};
use crate::CheckDigest;

/// The BootLog provides information about how the ROM and ROM_EXT
/// booted the chip.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout, uDebug)]
#[repr(C)]
pub struct BootLog {
    /// A SHA256 digest over all other fields in this struct.
    pub digest: [u8; 32],
    /// A tag that identifies this struct as the boot log ('BLOG').
    pub identifier: u32,
    /// The chip version (a git hash prefix from the ROM).
    pub chip_version: UnalignedU64,
    /// The boot slot the ROM chose to boot the ROM_EXT.
    pub rom_ext_slot: BootSlot,
    /// The ROM_EXT major version number.
    pub rom_ext_major: u32,
    /// The ROM_EXT minor version number.
    pub rom_ext_minor: u32,
    /// The ROM_EXT size in bytes.
    pub rom_ext_size: u32,
    /// The ROM_EXT nonce (a value used to prevent replay of signed commands).
    pub rom_ext_nonce: UnalignedU64,
    /// The boot slot the ROM_EXT chose to boot the owner firmware.
    pub bl0_slot: BootSlot,
    /// The chip's ownership state.
    pub ownership_state: OwnershipState,
    /// The number of ownership transfers performed on this chip.
    pub ownership_transfers: u32,
    /// Minimum security version permitted for ROM_EXT payloads.
    pub rom_ext_min_sec_ver: u32,
    /// Minimum security version permitted for application payloads.
    pub bl0_min_sec_ver: u32,
    /// The primary BL0 boot slot.
    pub primary_bl0_slot: BootSlot,
    /// Whether the retention RAM was initialized on this boot.
    pub retention_ram_initialized: HardenedBool,
    /// Reserved for future use.
    pub reserved: [u32; 8],
}

impl CheckDigest for BootLog {
    fn check_digest<F>(&self, f: F) -> bool
    where
        F: Fn(&[u8]) -> [u8; 32],
    {
        let digest = f(&self.as_bytes()[32..]);
        for (a, b) in self.digest.iter().zip(digest.iter().rev()) {
            if *a != *b {
                return false;
            }
        }
        return true;
    }

    fn set_digest<F>(&mut self, f: F)
    where
        F: Fn(&[u8]) -> [u8; 32],
    {
        let digest = f(&self.as_bytes()[32..]);
        for (a, b) in self.digest.iter_mut().zip(digest.iter().rev()) {
            *a = *b;
        }
    }
}
