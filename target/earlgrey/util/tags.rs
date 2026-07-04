// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Strongly-typed constants and magic tags used in the OpenTitan boot sequence.
//!
//! These types wrap `u32` values to provide type safety when parsing raw data
//! structures shared between the ROM, ROM_EXT, and application firmware.

#![allow(non_upper_case_globals)]

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};
use zfmt::Zfmt;

/// Identifies the type of firmware manifest.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct ManifestIdentifier(pub u32);

impl ManifestIdentifier {
    /// ROM Extension manifest identifier ('OTRE').
    pub const ROM_EXT: Self = Self(u32::from_le_bytes(*b"OTRE"));
    /// Application (BL0) manifest identifier ('OTB0').
    pub const APPLICATION: Self = Self(u32::from_le_bytes(*b"OTB0"));
}

/// A multi-bit representation of boolean values used for hardening.
///
/// Unlike standard single-bit booleans, hardened booleans use distinct multi-bit
/// values that are far apart in Hamming distance to reduce the chance of fault
/// attacks.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:04x}")]
#[repr(C)]
pub struct HardenedBool(pub u32);

impl HardenedBool {
    /// Hardened `true` value (`0x739`).
    pub const True: Self = Self(0x739);
    /// Hardened `false` value (`0x1d4`).
    pub const False: Self = Self(0x1d4);
}

/// Represents the chip's current ownership state.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct OwnershipState(pub u32);

impl OwnershipState {
    /// Recovery state (default/unowned).
    pub const Recovery: Self = Self(0);
    /// Locked to a specific owner.
    pub const LockedOwner: Self = Self(0x444e574f);
    /// Unlocked, allowing ownership to be claimed by this node.
    pub const UnlockedSelf: Self = Self(0x464c5355);
    /// Unlocked, allowing ownership to be claimed by any node.
    pub const UnlockedAny: Self = Self(0x594e4155);
    /// Unlocked, allowing ownership to be claimed by an endorsed owner.
    pub const UnlockedEndorsed: Self = Self(0x444e4555);
}

/// Identifies a boot slot (A or B).
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct BootSlot(pub u32);

impl BootSlot {
    /// Boot slot A ('AA__').
    pub const SlotA: Self = Self(u32::from_le_bytes(*b"AA__"));
    /// Boot slot B ('__BB').
    pub const SlotB: Self = Self(u32::from_le_bytes(*b"__BB"));
    /// Unspecified slot ('UUUU').
    pub const Unspecified: Self = Self(u32::from_le_bytes(*b"UUUU"));
}

impl BootSlot {
    /// Returns the opposite boot slot (A -> B, B -> A).
    ///
    /// Returns `None` if the slot is `Unspecified` or invalid.
    pub fn opposite(self) -> Option<Self> {
        match self {
            BootSlot::SlotA => Some(BootSlot::SlotB),
            BootSlot::SlotB => Some(BootSlot::SlotA),
            _ => None,
        }
    }

    /// Returns a string representation of the boot slot.
    pub fn as_str(self) -> &'static str {
        match self {
            BootSlot::SlotA => "SlotA",
            BootSlot::SlotB => "SlotB",
            BootSlot::Unspecified => "Unspecified",
            _ => "Invalid",
        }
    }
}

/// The unlock mode requested in the OwnershipUnlock command.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct UnlockMode(pub u32);

impl UnlockMode {
    /// Unlock the chip to accept any next owner.
    pub const Any: Self = Self(0x00594e41);
    /// Unlock the chip to accept only the endorsed next owner.
    pub const Endorsed: Self = Self(0x4f444e45);
    /// Unlock the chip to update the current owner configuration.
    pub const Update: Self = Self(0x00445055);
    /// Abort the unlock operation.
    pub const Abort: Self = Self(0x54524241);
}

/// Identifies the kind of Boot Service request or response.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct BootSvcKind(pub u32);

impl BootSvcKind {
    /// Empty request tag ('EMPT').
    pub const EmptyRequest: Self = Self(u32::from_le_bytes(*b"EMPT"));
    /// Empty response tag ('TPME').
    pub const EmptyResponse: Self = Self(u32::from_le_bytes(*b"TPME"));
    /// Request to set minimum BL0 security version ('MSEC').
    pub const MinBl0SecVerRequest: Self = Self(u32::from_le_bytes(*b"MSEC"));
    /// Response to minimum BL0 security version request ('CESM').
    pub const MinBl0SecVerResponse: Self = Self(u32::from_le_bytes(*b"CESM"));
    /// Request to set next BL0 boot slot ('NEXT').
    pub const NextBl0SlotRequest: Self = Self(u32::from_le_bytes(*b"NEXT"));
    /// Response to next BL0 boot slot request ('TXEN').
    pub const NextBl0SlotResponse: Self = Self(u32::from_le_bytes(*b"TXEN"));
    /// Request to unlock ownership ('UNLK').
    pub const OwnershipUnlockRequest: Self = Self(u32::from_le_bytes(*b"UNLK"));
    /// Response to ownership unlock request ('KLNU').
    pub const OwnershipUnlockResponse: Self = Self(u32::from_le_bytes(*b"KLNU"));
    /// Request to activate new ownership ('ACTV').
    pub const OwnershipActivateRequest: Self = Self(u32::from_le_bytes(*b"ACTV"));
    /// Response to ownership activate request ('VTCA').
    pub const OwnershipActivateResponse: Self = Self(u32::from_le_bytes(*b"VTCA"));
}

/// Identifies the cryptographic algorithm used for ownership keys.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct OwnershipKeyAlg(pub u32);

impl OwnershipKeyAlg {
    /// RSA algorithm ('RSA3').
    pub const Rsa: Self = Self(u32::from_le_bytes(*b"RSA3"));
    /// ECDSA P-256 algorithm ('P256').
    pub const EcdsaP256: Self = Self(u32::from_le_bytes(*b"P256"));
    /// Sphinx Pure signature scheme ('S+Pu').
    pub const SpxPure: Self = Self(u32::from_le_bytes(*b"S+Pu"));
    /// Sphinx Prehash signature scheme ('S+S2').
    pub const SpxPrehash: Self = Self(u32::from_le_bytes(*b"S+S2"));
    /// Hybrid Sphinx Pure scheme ('H+Pu').
    pub const HybridSpxPure: Self = Self(u32::from_le_bytes(*b"H+Pu"));
    /// Hybrid Sphinx Prehash scheme ('H+S2').
    pub const HybridSpxPrehash: Self = Self(u32::from_le_bytes(*b"H+S2"));
}

/// Identifies the version of the Retention RAM layout.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct RetRamVersion(pub u32);

impl RetRamVersion {
    /// Retention RAM layout version 3 ('RR03').
    pub const Version3: Self = Self(u32::from_le_bytes(*b"RR03"));
    /// Retention RAM layout version 4 ('RR04').
    pub const Version4: Self = Self(u32::from_le_bytes(*b"RR04"));
}

/// The ownership update mode configuration.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Zfmt)]
#[zfmt(format = "{0:c}")]
#[repr(C)]
pub struct OwnershipUpdateMode(pub u32);

impl OwnershipUpdateMode {
    /// Update mode open: `OPEN` (unlock key has full power).
    pub const Open: Self = Self(u32::from_le_bytes(*b"OPEN"));
    /// Update mode self: `SELF` (unlock key only unlocks to UnlockedSelf).
    pub const SelfMode: Self = Self(u32::from_le_bytes(*b"SELF"));
    /// Update mode NewVersion: `NEWV`
    /// (unlock key can't unlock; accept new owner configs from self-same owner
    /// if the config_version is newer).
    pub const NewVersion: Self = Self(u32::from_le_bytes(*b"NEWV"));
    /// Update mode SelfVersion: `SELV`
    /// (unlock key only unlocks to UnlockedSelf; accept new owner configs from
    /// self-same owner if the config_version is newer).
    pub const SelfVersion: Self = Self(u32::from_le_bytes(*b"SELV"));
    /// Update mode AnyVersion: `ANYV`
    /// (accept new owner configs as long as the config_version is newer,
    /// or any config_version if it is a new owner (transfer)).
    pub const AnyVersion: Self = Self(u32::from_le_bytes(*b"ANYV"));
}
