// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot Services protocol structures for OpenTitan.
//!
//! Boot Services allow the running application or ROM_EXT to request actions
//! from the early boot stages (ROM or ROM_EXT) that are executed upon the next
//! reboot. Requests and responses are passed via the Retention SRAM
//! (`RetRam.boot_svc` field).
//!
//! Every boot service message consists of a common `BootSvc` header containing
//! a SHA256 digest for integrity, followed by a command-specific payload.

use core::mem::size_of;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unalign};

use crate::rom_error::RomError;
use crate::tags::{BootSlot, BootSvcKind, HardenedBool, OwnershipKeyAlg, UnlockMode};
use crate::{CheckDigest, GetData};

/// The Boot Services message container.
///
/// This structure holds the common header (digest, identifier, kind, length)
/// and a generic byte payload area (`data`) that is cast to specific command
/// or response structures.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct BootSvc {
    /// A SHA256 digest over the rest of the boot services message (from `identifier` to the end of the payload).
    pub digest: [u8; 32],
    /// A tag that identifies this struct as a boot services message. Must be `'BSVC'`.
    pub identifier: u32,
    /// The type of boot services message (identifies request or response payload).
    pub kind: BootSvcKind,
    /// The total length of the boot services message in bytes (including this header).
    pub length: u32,
    /// The message payload data area.
    pub data: [u8; 212],
}

impl BootSvc {
    /// Length of the header before the `data` field (digest + identifier + kind + length) = 44 bytes.
    const HEADER_LEN: usize = 44;
    /// Expected value for `identifier` field ('BSVC').
    const TAG: u32 = u32::from_le_bytes(*b"BSVC");
}

impl CheckDigest for BootSvc {
    /// Validates the integrity digest of the BootSvc message.
    ///
    /// The digest is calculated over the message bytes starting from the `identifier`
    /// field up to the configured `length`.
    fn check_digest<F>(&self, f: F) -> bool
    where
        F: Fn(&[u8]) -> [u8; 32],
    {
        if self.length as usize > size_of::<Self>() {
            return false;
        }
        if let Some(data) = self.as_bytes().get(32..self.length as usize) {
            let digest = f(data);
            // OpenTitan digest is stored in reverse byte order
            for (a, b) in self.digest.iter().zip(digest.iter().rev()) {
                if *a != *b {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Computes and sets the integrity digest on the BootSvc message.
    fn set_digest<F>(&mut self, f: F)
    where
        F: Fn(&[u8]) -> [u8; 32],
    {
        let length = self.length as usize;
        if let Some(data) = self.as_bytes().get(32..length) {
            let digest = f(data);
            // OpenTitan digest is stored in reverse byte order
            for (a, b) in self.digest.iter_mut().zip(digest.iter().rev()) {
                *a = *b;
            }
        }
    }
}

/// Helper macro to implement `GetData<T>` for `BootSvc`, providing type-safe
/// accessors to the underlying command payload stored in the `data` buffer.
macro_rules! impl_getdata {
    ($t:ident, $tag:ident) => {
        impl GetData<$t> for BootSvc {
            /// Attempts to get a reference to the payload `$t` if the header
            /// matches the expected format, size, and service kind.
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
            /// Prepares the header fields and returns a mutable reference to the
            /// payload `$t`.
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

/// An empty boot services message (used for clearing requests).
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct Empty {
    pub payload: [u8; 212],
}
impl_getdata!(Empty, EmptyRequest);

/// Request to set the minimum owner stage firmware security version (BL0).
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct MinBl0SecVerRequest {
    /// The desired minimum security version.
    pub ver: u32,
}
impl_getdata!(MinBl0SecVerRequest);

/// Response to the minimum version request.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct MinBl0SecVerResponse {
    /// The current minimum security version configured in the chip.
    pub ver: u32,
    /// Status code of the operation returned by the bootloader.
    pub status: RomError,
}
impl_getdata!(MinBl0SecVerResponse);

/// Request to set the next owner stage boot slot (one-time boot slot configuration).
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct NextBl0SlotRequest {
    /// The slot to boot on the next reset (SlotA or SlotB).
    pub next_bl0_slot: BootSlot,
    /// The slot to configure as primary (persisted choice).
    pub primary_bl0_slot: BootSlot,
}
impl_getdata!(NextBl0SlotRequest);

/// Response to the next boot slot request.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct NextBl0SlotResponse {
    /// Status code of the operation.
    pub status: RomError,
    /// The current active primary boot slot.
    pub primary_bl0_slot: BootSlot,
}
impl_getdata!(NextBl0SlotResponse);

/// Request to unlock ownership of the chip, enabling ownership transition.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipUnlockRequest {
    /// The desired unlock mode (Any, Endorsed, Update, Abort).
    pub unlock_mode: UnlockMode,
    /// The Device Identification Number (DIN) of the target chip.
    pub din: Unalign<u64>,
    /// Reserved for future use.
    pub reserved: [u32; 7],
    /// The key algorithm of the next owner (used for Endorsed mode).
    pub next_owner_alg: OwnershipKeyAlg,
    /// Nonce retrieved from the ROM_EXT to prevent replay attacks.
    pub nonce: Unalign<u64>,
    /// The public key of the next owner (used for Endorsed mode).
    pub next_owner_key: [u8; 96],
    /// Cryptographic signature over the fields [unlock_mode..next_owner_key]
    /// generated by the current owner's unlock key.
    pub signature: [u8; 64],
}
impl_getdata!(OwnershipUnlockRequest);

/// Response to the ownership unlock command.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipUnlockResponse {
    /// Status code of the unlock operation.
    pub status: RomError,
}
impl_getdata!(OwnershipUnlockResponse);

/// Request to activate new ownership on the chip.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipActivateRequest {
    /// The primary boot slot configuration for the new owner.
    pub primary_bl0_slot: BootSlot,
    /// The Device Identification Number (DIN) of the target chip.
    pub din: Unalign<u64>,
    /// Hardened boolean instructing whether to erase the previous owner's flash data.
    pub erase_previous: HardenedBool,
    /// Reserved for future use.
    pub reserved: [u32; 31],
    /// Nonce retrieved from the ROM_EXT to prevent replay.
    pub nonce: Unalign<u64>,
    /// Cryptographic signature over [primary_bl0_slot..nonce] generated with
    /// the new owner's activate key.
    pub signature: [u8; 64],
}
impl_getdata!(OwnershipActivateRequest);

/// Response to the ownership activate command.
#[derive(Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct OwnershipActivateResponse {
    /// Status code of the activation operation.
    pub status: RomError,
}
impl_getdata!(OwnershipActivateResponse);
