use core::mem::size_of;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unalign};

use crate::rom_error::RomError;
use crate::tags::{BootSlot, BootSvcKind, HardenedBool, OwnershipKeyAlg, UnlockMode};
use crate::{CheckDigest, GetData};

#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
/// The Boot Services header common to all boot services commands and responses.
pub struct BootSvc {
    /// A SHA256 digest over the rest of the boot services message.
    pub digest: [u8; 32],
    /// A tag that identifies this struct as a boot services message ('BSVC').
    pub identifier: u32,
    /// The type of boot services message that follows this header.
    pub kind: BootSvcKind,
    /// The length of the boot services message in bytes (including the header).
    pub length: u32,
    /// The message data.
    pub data: [u8; 212],
}

impl BootSvc {
    const HEADER_LEN: usize = 44;
    const TAG: u32 = u32::from_le_bytes(*b"BSVC");
}

impl CheckDigest for BootSvc {
    fn check_digest<F>(&self, f: F) -> bool
    where
        F: Fn(&[u8]) -> [u8; 32],
    {
        let digest = f(&self.as_bytes()[32..self.length as usize]);
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
        let digest = f(&self.as_bytes()[32..self.length as usize]);
        for (a, b) in self.digest.iter_mut().zip(digest.iter().rev()) {
            *a = *b;
        }
    }
}

macro_rules! impl_getdata {
    ($t:ident, $tag:ident) => {
        impl GetData<$t> for BootSvc {
            fn get(&self) -> Option<&$t> {
                if self.identifier != BootSvc::TAG
                    || self.length as usize != BootSvc::HEADER_LEN + size_of::<$t>()
                    || self.kind != BootSvcKind::$tag
                {
                    return None;
                }
                let (result, _) = <$t>::ref_from_prefix(&self.data).unwrap();
                Some(result)
            }
            fn get_mut(&mut self) -> &mut $t {
                self.identifier = BootSvc::TAG;
                self.length = (BootSvc::HEADER_LEN + size_of::<$t>()) as u32;
                self.kind = BootSvcKind::$tag;
                let (result, _) = <$t>::mut_from_prefix(&mut self.data).unwrap();
                result
            }
        }
    };
    ($t:ident) => {
        impl_getdata!($t, $t);
    };
}

/// An empty boot services message.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct Empty {
    pub payload: [u8; 212],
}
impl_getdata!(Empty, EmptyRequest);

/// Request to set the minimum owner stage firmware version.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct MinBl0SecVerRequest {
    /// The desired minimum BL0 version.
    pub ver: u32,
}
impl_getdata!(MinBl0SecVerRequest);

/// Response to the minimum version request.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct MinBl0SecVerResponse {
    /// The current minimum BL0 version.
    pub ver: u32,
    /// The status response to the request.
    pub status: RomError,
}
impl_getdata!(MinBl0SecVerResponse);

/// Request to set the next (one-time) owner stage boot slot.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct NextBl0SlotRequest {
    /// The slot to boot.
    pub next_bl0_slot: BootSlot,
    /// The slot to configure as primary.
    pub primary_bl0_slot: BootSlot,
}
impl_getdata!(NextBl0SlotRequest);

/// Response to the set next boot slot request.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct NextBl0SlotResponse {
    /// The status response to the request.
    pub status: RomError,
    /// The current primary slot.
    pub primary_bl0_slot: BootSlot,
}
impl_getdata!(NextBl0SlotResponse);

/// Request to unlock ownership of the chip.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipUnlockRequest {
    /// The desired unlock mode.
    pub unlock_mode: UnlockMode,
    /// The Device Identification Number of the chip.
    pub din: Unalign<u64>,
    /// Reserved for future use.
    pub reserved: [u32; 7],
    /// The algorithm of next owner's key (for unlock Endorsed mode).
    pub next_owner_alg: OwnershipKeyAlg,
    /// The ROM_EXT nonce.
    pub nonce: Unalign<u64>,
    /// The next owner's key (for unlock Endorsed mode).
    pub next_owner_key: [u8; 96],
    /// A signature over [unlock_mode..next_owner_key] with the current owner unlock key.
    pub signature: [u8; 64],
}
impl_getdata!(OwnershipUnlockRequest);

/// Response to the ownership unlock command.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipUnlockResponse {
    /// The status response to the request.
    pub status: RomError,
}
impl_getdata!(OwnershipUnlockResponse);

/// Request to activate ownership of the chip.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipActivateRequest {
    /// The new primary boot slot after activating ownership.
    pub primary_bl0_slot: BootSlot,
    /// The Device Identification Number of the chip.
    pub din: Unalign<u64>,
    /// Whether to erase the previous owner's data during activation.
    pub erase_previous: HardenedBool,
    /// Reserved for future use.
    pub reserved: [u32; 31],
    /// The ROM_EXT nonce.
    pub nonce: Unalign<u64>,
    /// A signature over [primary_bl0_slot..nonce] with the next owner's activate key.
    pub signature: [u8; 64],
}
impl_getdata!(OwnershipActivateRequest);

/// Response to the ownership activate command.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipActivateResponse {
    /// The status response to the request.
    pub status: RomError,
}
impl_getdata!(OwnershipActivateResponse);
