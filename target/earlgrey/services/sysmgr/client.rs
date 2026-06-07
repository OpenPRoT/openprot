// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes, KnownLayout};

use earlgrey_util::tags::{BootSlot, HardenedBool, OwnershipState};
use userspace::time::Instant;
use util_error::ErrorCode;
use util_ipc::IpcChannel;

use zfmt::Zfmt;

pub mod op {
    use util_types::Opcode;
    pub const SYSMGR_OP_GET_BOOT_INFO: Opcode = Opcode::new(*b"MGBI");
    pub const SYSMGR_OP_SET_BOOT_POLICY: Opcode = Opcode::new(*b"MGBP");
    pub const SYSMGR_OP_REQ_REBOOT: Opcode = Opcode::new(*b"MGRB");
}

pub struct SysmgrClient<IPC: IpcChannel> {
    ipc: IPC,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Zfmt)]
#[zfmt(format = "OpenTitan {creator_id:04x}-{product_id:04x}-{revision:02x}")]
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

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Zfmt)]
#[repr(C)]
#[zfmt(format = "ROM_EXT {major}.{minor} (slot={boot_slot:c})")]
pub struct RomExtInfo {
    pub boot_slot: BootSlot,
    pub major: u32,
    pub minor: u32,
    pub min_sec_ver: u32,
    pub size: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Zfmt)]
#[repr(C)]
#[zfmt(format = "App {firmware_domain:c} size={size} (slot={boot_slot:c}/pref={pref_slot:c})")]
pub struct ApplicationInfo {
    pub boot_slot: BootSlot,
    pub pref_slot: BootSlot,
    pub min_sec_ver: u32,
    pub size: u32,
    pub firmware_domain: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Zfmt)]
#[repr(C)]
#[zfmt(format = "owner={state:c}")]
pub struct OwnershipInfo {
    pub nonce: u64,
    pub state: OwnershipState,
    pub transfers: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Zfmt)]
#[repr(C)]
#[zfmt(format = "reset={reason:02x}")]
pub struct ResetInfo {
    pub reason: u32,
    pub retram_init: HardenedBool,
    pub hardware_straps: u32,
    pub software_straps: u32,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Zfmt)]
#[repr(C)]
#[zfmt(format = "{chip} {rom_ext} {app} {ownership} {reset}")]
pub struct BootInfo {
    pub chip: ChipInfo,
    pub rom_ext: RomExtInfo,
    pub app: ApplicationInfo,
    pub ownership: OwnershipInfo,
    pub reset: ResetInfo,
}

#[derive(Clone, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct BootPolicy {
    pub pref_slot: BootSlot,
    pub next_slot: BootSlot,
}

impl<IPC: IpcChannel> SysmgrClient<IPC> {
    pub const fn new(ipc: IPC) -> Self {
        SysmgrClient { ipc }
    }

    pub fn get_boot_info(&self) -> Result<BootInfo, ErrorCode> {
        let mut result = 0u32;
        let mut info = BootInfo::new_zeroed();
        self.ipc
            .transact(
                &[op::SYSMGR_OP_GET_BOOT_INFO.as_bytes()],
                &mut [result.as_mut_bytes(), info.as_mut_bytes()],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        ErrorCode::check_status(result)?;
        Ok(info)
    }

    pub fn set_boot_policy(&self, policy: BootPolicy) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc
            .transact(
                &[
                    op::SYSMGR_OP_SET_BOOT_POLICY.as_bytes(),
                    policy.pref_slot.as_bytes(),
                    policy.next_slot.as_bytes(),
                ],
                &mut [result.as_mut_bytes()],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        ErrorCode::check_status(result)
    }

    pub fn request_reboot(&self) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc
            .transact(
                &[op::SYSMGR_OP_REQ_REBOOT.as_bytes()],
                &mut [result.as_mut_bytes()],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        ErrorCode::check_status(result)
    }
}
