// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![allow(deprecated)]
use zerocopy::{FromBytes, IntoBytes};

use earlgrey_sysmgr_client::*;
use earlgrey_util::boot_svc::NextBl0SlotRequest;
use earlgrey_util::error as eg_error;
use earlgrey_util::ret_ram::RetRam;
use earlgrey_util::CheckDigest;
use earlgrey_util::GetData;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
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

        util_zfmt::info!(info.clone());
        Ok(Self { info, retram })
    }

    fn handle_get_boot_info<'a>(
        &self,
        data: &'a mut [u8],
        _reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        let info = self.info.as_bytes();
        data[..info.len()].copy_from_slice(info);
        Ok(&data[..info.len()])
    }

    fn handle_req_reboot<'a>(
        &self,
        data: &'a mut [u8],
        _reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        pw_log::info!("RESET REQUESTED!");
        let mut rstmgr = unsafe { RstmgrAon::new() };
        rstmgr.regs_mut().reset_req().write(|_| 6u32.into());
        Ok(&data[0..0])
    }

    fn handle_set_boot_policy<'a>(
        &mut self,
        data: &'a mut [u8],
        reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        let policy_bytes = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let policy =
            BootPolicy::read_from_bytes(policy_bytes).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let request: &mut NextBl0SlotRequest = self.retram.boot_svc.get_mut();
        request.next_bl0_slot = policy.next_slot;
        request.primary_bl0_slot = policy.pref_slot;
        self.retram
            .boot_svc
            .set_digest(|data| Sha256::digest(data).into());
        Ok(&data[0..0])
    }

    fn handle_op<'a>(
        &mut self,
        opcode: Opcode,
        data: &'a mut [u8],
        reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        match opcode {
            op::SYSMGR_OP_GET_BOOT_INFO => self.handle_get_boot_info(data, reqsz),
            op::SYSMGR_OP_REQ_REBOOT => self.handle_req_reboot(data, reqsz),
            op::SYSMGR_OP_SET_BOOT_POLICY => self.handle_set_boot_policy(data, reqsz),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }

    pub fn handle_one(&mut self, ipc: &impl IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
        let len = ipc.read(0, data).map_err(ErrorCode::kernel_error)?;
        let (opcode, reqrsp) = data.split_at_mut(core::mem::size_of::<Opcode>());
        let opcode = Opcode::read_from_bytes(opcode).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let len = len.saturating_sub(core::mem::size_of::<Opcode>());
        let mut status = 0u32;
        let result = match self.handle_op(opcode, reqrsp, len) {
            Ok(result) => result,
            Err(e) => {
                status = e.0.get();
                &[]
            }
        };
        ipc.respond(&[status.as_bytes(), result])
            .map_err(ErrorCode::kernel_error)?;
        Ok(())
    }
}
