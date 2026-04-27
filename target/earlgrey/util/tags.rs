#![allow(non_upper_case_globals)]

use ufmt::{uDebug, uDisplay};
use util_types::impl_fourcc;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct ManifestIdentifier(pub u32);
impl ManifestIdentifier {
    pub const ROM_EXT: Self = Self(u32::from_le_bytes(*b"OTRE"));
    pub const APPLICATION: Self = Self(u32::from_le_bytes(*b"OTB0"));
}

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct HardenedBool(pub u32);
impl HardenedBool {
    pub const True: Self = Self(0x739);
    pub const False: Self = Self(0x1d4);
}

impl uDisplay for HardenedBool {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match *self {
            HardenedBool::True => ufmt::uwrite!(f, "True"),
            HardenedBool::False => ufmt::uwrite!(f, "False"),
            unk => ufmt::uwrite!(f, "HardenedBool({:08x})", unk.0),
        }
    }
}

impl uDebug for HardenedBool {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        ufmt::uDisplay::fmt(self, f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct OwnershipState(pub u32);
impl OwnershipState {
    pub const Recovery: Self = Self(0);
    pub const LockedOwner: Self = Self(0x444e574f);
    pub const UnlockedSelf: Self = Self(0x464c5355);
    pub const UnlockedAny: Self = Self(0x594e4155);
    pub const UnlockedEndorsed: Self = Self(0x444e4555);
}
impl_fourcc!(OwnershipState);

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct BootSlot(pub u32);
impl BootSlot {
    pub const SlotA: Self = Self(u32::from_le_bytes(*b"AA__"));
    pub const SlotB: Self = Self(u32::from_le_bytes(*b"__BB"));
    pub const Unspecified: Self = Self(u32::from_le_bytes(*b"UUUU"));
}
impl_fourcc!(BootSlot);

impl BootSlot {
    pub fn opposite(self) -> Option<Self> {
        match self {
            BootSlot::SlotA => Some(BootSlot::SlotB),
            BootSlot::SlotB => Some(BootSlot::SlotA),
            _ => None,
        }
    }
}

/// The unlock mode for the OwnershipUnlock command.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
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
impl_fourcc!(UnlockMode);

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct BootSvcKind(pub u32);
impl BootSvcKind {
    pub const EmptyRequest: Self = Self(u32::from_le_bytes(*b"EMPT"));
    pub const EmptyResponse: Self = Self(u32::from_le_bytes(*b"TPME"));
    pub const MinBl0SecVerRequest: Self = Self(u32::from_le_bytes(*b"MSEC"));
    pub const MinBl0SecVerResponse: Self = Self(u32::from_le_bytes(*b"CESM"));
    pub const NextBl0SlotRequest: Self = Self(u32::from_le_bytes(*b"NEXT"));
    pub const NextBl0SlotResponse: Self = Self(u32::from_le_bytes(*b"TXEN"));
    pub const OwnershipUnlockRequest: Self = Self(u32::from_le_bytes(*b"UNLK"));
    pub const OwnershipUnlockResponse: Self = Self(u32::from_le_bytes(*b"KLNU"));
    pub const OwnershipActivateRequest: Self = Self(u32::from_le_bytes(*b"ACTV"));
    pub const OwnershipActivateResponse: Self = Self(u32::from_le_bytes(*b"VTCA"));
}
impl_fourcc!(BootSvcKind);

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct OwnershipKeyAlg(pub u32);
impl OwnershipKeyAlg {
    pub const Rsa: Self = Self(u32::from_le_bytes(*b"RSA3"));
    pub const EcdsaP256: Self = Self(u32::from_le_bytes(*b"P256"));
    pub const SpxPure: Self = Self(u32::from_le_bytes(*b"S+Pu"));
    pub const SpxPrehash: Self = Self(u32::from_le_bytes(*b"S+S2"));
    pub const HybridSpxPure: Self = Self(u32::from_le_bytes(*b"H+Pu"));
    pub const HybridSpxPrehash: Self = Self(u32::from_le_bytes(*b"H+S2"));
}
impl_fourcc!(OwnershipKeyAlg);

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct RetRamVersion(pub u32);
impl RetRamVersion {
    pub const Version3: Self = Self(u32::from_le_bytes(*b"RR03"));
    pub const Version4: Self = Self(u32::from_le_bytes(*b"RR04"));
}
impl_fourcc!(RetRamVersion);
