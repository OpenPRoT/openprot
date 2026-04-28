// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
use zerocopy::{FromBytes, IntoBytes};

use earlgrey_sysmgr_client::*;
use earlgrey_util::boot_svc::NextBl0SlotRequest;
use earlgrey_util::error as eg_error;
use earlgrey_util::ret_ram::RetRam;
use earlgrey_util::tags::BootSlot;
use earlgrey_util::CheckDigest;
use earlgrey_util::GetData;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use util_types::fourcc::FourCC;
use util_types::Opcode;

use lc_ctrl::LcCtrl;
use rstmgr::RstmgrAon;
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub struct SysmgrServer {
    info: BootInfo,
    retram: &'static mut RetRam,
}

impl SysmgrServer {
    pub fn new() -> Result<Self, ErrorCode> {
        let lc_ctrl = unsafe { LcCtrl::new() };
        let lcreg = lc_ctrl.regs();

        let retram = unsafe { RetRam::mut_ref() };
        if !retram
            .boot_log
            .check_digest(|data| Sha256::digest(data).into())
        {
            return Err(eg_error::EG_ERROR_BAD_BOOT_LOG);
        }

        let info = BootInfo {
            chip: ChipInfo {
                git_hash: retram.boot_log.chip_version.get(),
                lc_state: u32::from(lcreg.lc_state().read()),
                device_id: lcreg.device_id().read().into(),
                creator_id: lcreg.hw_revision0().read().silicon_creator_id() as u16,
                product_id: lcreg.hw_revision0().read().product_id() as u16,
                revision: lcreg.hw_revision1().read().revision_id() as u8,
                _pad: Default::default(),
            },
            rom_ext: RomExtInfo {
                boot_slot: retram.boot_log.rom_ext_slot,
                major: retram.boot_log.rom_ext_major,
                minor: retram.boot_log.rom_ext_minor,
                min_sec_ver: retram.boot_log.rom_ext_min_sec_ver,
                size: retram.boot_log.rom_ext_size,
            },
            app: ApplicationInfo {
                boot_slot: retram.boot_log.bl0_slot,
                pref_slot: retram.boot_log.primary_bl0_slot,
                min_sec_ver: retram.boot_log.bl0_min_sec_ver,
                // TODO: get from config?
                size: 400 * 1024,
                // TODO: read from keymgr.
                firmware_domain: 0,
            },
            ownership: OwnershipInfo {
                nonce: retram.boot_log.rom_ext_nonce.get(),
                state: retram.boot_log.ownership_state,
                transfers: retram.boot_log.ownership_transfers,
            },
            reset: ResetInfo {
                reason: retram.reset_reasons,
                retram_init: retram.boot_log.retention_ram_initialized,
                // TODO: read gpio strapping value.
                hardware_straps: 0,
                // TODO: get from config?
                software_straps: 0,
            },
        };

        pw_log::info!("Earlgrey System Manager");
        pw_log::info!(
            "chip: {:04x}-{:04x}-{:02x} / {:016x}",
            info.chip.creator_id as u16,
            info.chip.product_id as u16,
            info.chip.revision as u8,
            info.chip.git_hash as u64,
        );
        pw_log::info!(
            "ROM_EXT: {}.{} in {}",
            info.rom_ext.major as u32,
            info.rom_ext.minor as u32,
            info.rom_ext.boot_slot.as_str() as &str,
        );
        pw_log::info!(
            "Application in {} (prefer {})",
            info.app.boot_slot.as_str() as &str,
            info.app.pref_slot.as_str() as &str,
        );
        pw_log::info!("Reset reasons: {:08x}", info.reset.reason as u32);

        Ok(Self { info, retram })
    }

    fn handle_get_boot_info<'a>(&self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let info = self.info.as_bytes();
        data[..info.len()].copy_from_slice(info);
        Ok(&data[..info.len()])
    }

    fn handle_req_reboot<'a>(&self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        pw_log::info!("RESET REQUESTED!");
        let mut rstmgr = unsafe { RstmgrAon::new() };
        rstmgr.regs_mut().reset_req().write(|_| 6u32.into());
        Ok(&data[0..0])
    }

    fn handle_set_boot_policy<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let (pref, rest) =
            BootSlot::ref_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let (next, _rest) =
            BootSlot::ref_from_prefix(rest).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let request: &mut NextBl0SlotRequest = self.retram.boot_svc.get_mut();
        request.next_bl0_slot = *next;
        request.primary_bl0_slot = *pref;
        self.retram
            .boot_svc
            .set_digest(|data| Sha256::digest(data).into());
        Ok(&data[0..0])
    }

    fn handle_op<'a>(&mut self, opcode: Opcode, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        match opcode {
            op::SYSMGR_OP_GET_BOOT_INFO => self.handle_get_boot_info(data),
            op::SYSMGR_OP_REQ_REBOOT => self.handle_req_reboot(data),
            op::SYSMGR_OP_SET_BOOT_POLICY => self.handle_set_boot_policy(data),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }

    fn handle_one(&mut self, ipc: &IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
        ipc.wait_readable()?;
        let len = ipc.read(0, data)?;
        if len < 4 {
            return Err(error::IPC_ERROR_BAD_REQ_LEN);
        }
        let (op_status, reqrsp) = data.split_at_mut(4);
        let opcode = Opcode::read_from_bytes(op_status).unwrap();
        let len = match self.handle_op(opcode, reqrsp) {
            Ok(result) => {
                op_status.copy_from_slice((0u32).as_bytes());
                result.len()
            }
            Err(e) => {
                op_status.copy_from_slice(e.0.as_bytes());
                0
            }
        };
        ipc.respond(&data[..4 + len])?;
        Ok(())
    }

    pub fn run(&mut self, ipc: &IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
        loop {
            self.handle_one(ipc, data)?;
        }
    }
}
