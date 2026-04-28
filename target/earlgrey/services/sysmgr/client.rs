// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, FromZeros};

use earlgrey_util::tags::{BootSlot, OwnershipState, HardenedBool};
use userspace::time::Instant;
use util_error::ErrorCode;
use util_ipc::IpcChannel;
use ufmt::derive::uDebug;

pub mod op {
use util_types::Opcode;
pub const SYSMGR_OP_GET_BOOT_INFO: Opcode = Opcode::new(*b"MGBI");
pub const SYSMGR_OP_SET_BOOT_POLICY: Opcode = Opcode::new(*b"MGBP");
pub const SYSMGR_OP_REQ_REBOOT: Opcode = Opcode::new(*b"MGRB");
}

pub struct SysmgrClient {
    ipc: IpcChannel,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct ChipInfo {
    pub git_hash: u64,
    pub lc_state: u32,
    pub device_id: [u32; 8],
    pub creator_id: u16,
    pub product_id: u16,
    pub revision: u8,
    pub _pad: [u8; 7],
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct RomExtInfo {
    pub boot_slot: BootSlot,
    pub major: u32,
    pub minor: u32,
    pub min_sec_ver: u32,
    pub size: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct ApplicationInfo {
    pub boot_slot: BootSlot,
    pub pref_slot: BootSlot,
    pub min_sec_ver: u32,
    pub size: u32,
    pub firmware_domain: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct OwnershipInfo {
    pub nonce: u64,
    pub state: OwnershipState,
    pub transfers: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct ResetInfo {
    pub reason: u32,
    pub retram_init: HardenedBool,
    pub hardware_straps: u32,
    pub software_straps: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct BootInfo {
    pub chip: ChipInfo,
    pub rom_ext: RomExtInfo,
    pub app: ApplicationInfo,
    pub ownership: OwnershipInfo,
    pub reset: ResetInfo,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, uDebug)]
#[repr(C)]
pub struct BootPolicy {
    pub pref_slot: BootSlot,
    pub next_slot: BootSlot,
}

impl SysmgrClient {
    pub const fn new(ipc: IpcChannel) -> Self {
        SysmgrClient { ipc }
    }

    pub fn get_boot_info(&self) -> Result<BootInfo, ErrorCode> {
        let mut result = 0u32;
        let mut info = BootInfo::new_zeroed();
        self.ipc.transaction::<256>(
            &[op::SYSMGR_OP_GET_BOOT_INFO.as_bytes()],
            &mut [result.as_mut_bytes(), info.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)?;
        Ok(info)
    }

    pub fn set_boot_policy(&self, pref: BootSlot, next: BootSlot) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc.transaction::<256>(
            &[op::SYSMGR_OP_SET_BOOT_POLICY.as_bytes(),
            pref.as_bytes(),
            next.as_bytes(),
            ],
            &mut [result.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }

    pub fn request_reboot(&self) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc.transaction::<256>(
            &[op::SYSMGR_OP_REQ_REBOOT.as_bytes(),
            ],
            &mut [result.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }

}
