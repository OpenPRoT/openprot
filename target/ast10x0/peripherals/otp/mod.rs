// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use self::common::{
    AspeedChipVersion, AspeedOtpRegion, Logger, OtpError, SessionInfo, StrapStatus,
};
use ast1060_pac::Secure;
use core::fmt::Write;
use embedded_hal::delay::DelayNs;

type SbRegBlock = ast1060_pac::secure::RegisterBlock;

pub mod common;

struct DummyDelay;

impl DelayNs for DummyDelay {
    // Timing calibrated for AST1060 at 200MHz; host builds spin_loop with no real delay.
    fn delay_ns(&mut self, ns: u32) {
        for _ in 0..(ns / 100) {
            #[cfg(target_arch = "arm")]
            cortex_m::asm::nop();
            #[cfg(not(target_arch = "arm"))]
            core::hint::spin_loop();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum OtpSoak {
    Default = 0,
    NormalProg = 1,
    SoakProg = 2,
}

pub struct OtpController<L: Logger> {
    sb: &'static SbRegBlock,
    scu_base: *const ast1060_pac::scu::RegisterBlock,
    locked: bool,
    pub logger: L,
}

macro_rules! otp_debug {
    ($logger:expr, $($arg:tt)*) => {
        let mut buf: heapless::String<64> = heapless::String::new();
        let _ = write!(buf, $($arg)*); // truncate rather than panic on overflow
        $logger.debug(buf.as_str());
    };
}

macro_rules! otp_error {
    ($logger:expr, $($arg:tt)*) => {
        let mut buf: heapless::String<64> = heapless::String::new();
        let _ = write!(buf, $($arg)*); // truncate rather than panic on overflow
        $logger.error(buf.as_str());
    };
}

//major minor build
const OTP_VER: &str = "2.1.1";

const ID0_AST1060A1: u32 = 0xA001_0000;
const ID1_AST1060A1: u32 = 0xA001_0000;
const ID0_AST1060A2: u32 = 0xA003_0000;
const ID1_AST1060A2: u32 = 0xA003_0000;
const ID0_AST1060A2_ENG: u32 = 0x8003_0000;
const ID1_AST1060A2_ENG: u32 = 0x8003_0000;
//const OTP_AST1060A1: u32 = 3;
//const OTP_AST1060A2: u32 = 4;
const OTP_PASSWD: u32 = 0x349f_e38a;
const OTP_READ_CMD: u32 = 0x23b1_e361;
const OTP_WRITE_CMD: u32 = 0x23b1_e362;
//const OTP_COMP_CMD: u32 = 0x23b1_e363;
const OTP_PROG_CMD: u32 = 0x23b1_e364;

pub const OTP_MEM_LIMIT: u32 = 2144; //67kbits
const OTP_MEM_LIMIT_DATA: usize = 2048;
//const OTP_MEM_ECC_OFFSET: u32 = 1792; //DWORD

/// timing
const OTP_TIMING_200US: u32 = 0x0419_1388;
const OTP_TIMING_600US: u32 = 0x0419_3a98;
const OTP_OP_RETRIES: u8 = 20;
///OTP memory layout
///
/// OTP region protection
pub const OTP_CONF_OFFSET: u32 = 0x800;
pub const OTP_MEM_LOCK_ENABLE: u32 = 1 << 31;
pub const OTP_KEY_PROT_ENABLE: u32 = 1 << 29;
pub const OTP_STRAP_PROT_ENABLE: u32 = 1 << 25;
pub const OTP_CONF_PROT_ENABLE: u32 = 1 << 24;
//data secure,user,ecc
pub const OTP_USER_ECC_PROT_ENABLE: u32 = 1 << 23;
const OTP_SECURE_PROT_ENABLE: u32 = 1 << 22;
pub const OTP_SECURE_SIZE_BIT_POS: u32 = 16;
const OTP_SECURE_SIZE_MASK: u32 = 0x3f;

#[derive(Debug, Clone, Copy)]
pub struct SoakProInfo {
    pub address: u32,
    pub data: u32,
}
/// Write MRA
/// Write MRB
/// Write MR
pub static SOAK_PROG_DEFAULT: &[SoakProInfo] = &[
    SoakProInfo {
        address: 0x3000,
        data: 0,
    },
    SoakProInfo {
        address: 0x5000,
        data: 0,
    },
    SoakProInfo {
        address: 0x1000,
        data: 0,
    },
];
pub static SOAK_PROG_NORMAL: &[SoakProInfo] = &[
    SoakProInfo {
        address: 0x3000,
        data: 0x1320,
    },
    SoakProInfo {
        address: 0x5000,
        data: 0x1008,
    },
    SoakProInfo {
        address: 0x1000,
        data: 0x0024,
    },
];
pub static SOAK_PROG_SOAK: &[SoakProInfo] = &[
    SoakProInfo {
        address: 0x3000,
        data: 0x1320,
    },
    SoakProInfo {
        address: 0x5000,
        data: 0x0007,
    },
    SoakProInfo {
        address: 0x1000,
        data: 0x0100,
    },
];

pub struct RegionInfo {
    pub region_type: AspeedOtpRegion,
    pub start: usize,
    pub cdw_size: usize,
    pub alignment: usize,
}

pub static REGION_INFO: &[RegionInfo] = &[
    RegionInfo {
        region_type: AspeedOtpRegion::Data,
        start: 0,
        cdw_size: OTP_MEM_LIMIT_DATA,
        alignment: 4,
    },
    RegionInfo {
        //[otpcfg0, otpcfg31]
        region_type: AspeedOtpRegion::Configuration,
        start: 0x800,
        cdw_size: 32,
        alignment: 4,
    },
    RegionInfo {
        //[otpcfg16, otpcfg27]
        region_type: AspeedOtpRegion::Strap,
        start: 0xC00,
        cdw_size: 2,
        alignment: 4,
    },
    RegionInfo {
        //[otpcfg28, otpcfg29]
        region_type: AspeedOtpRegion::ScuProtection,
        start: 0xE08,
        cdw_size: 2,
        alignment: 4,
    },
];
// INVARIANT: REGION_INFO must be ordered by AspeedOtpRegion discriminant (Data=0, Configuration=1,
// Strap=2, ScuProtection=3) because several functions index it directly by `region as usize`.

impl<L: Logger> OtpController<L> {
    /// # Safety
    /// `scu_base` must point to a valid SCU register block for the lifetime of this controller.
    pub unsafe fn new(scu_base: *const ast1060_pac::scu::RegisterBlock, logger: L) -> Self {
        Self {
            sb: unsafe { &*Secure::PTR },
            scu_base,
            locked: false,
            logger,
        }
    }

    fn scu_regs(&self) -> &ast1060_pac::scu::RegisterBlock {
        unsafe { &*self.scu_base }
    }
    pub fn chip_version(&self) -> AspeedChipVersion {
        let revid0 = self.scu_regs().scu004().read().bits();
        let revid1 = self.scu_regs().scu014().read().bits();

        if revid0 == ID0_AST1060A1 && revid1 == ID1_AST1060A1 {
            return AspeedChipVersion::Ast1060A1;
        } else if revid0 == ID0_AST1060A2 && revid1 == ID1_AST1060A2
            || revid0 == ID0_AST1060A2_ENG && revid1 == ID1_AST1060A2_ENG
        {
            return AspeedChipVersion::Ast1060A2;
        }
        AspeedChipVersion::Unknown
    }
    pub fn wait_complete(&self) -> bool {
        let mut tries: u32 = 1000;
        let mut delay = DummyDelay {};

        delay.delay_ns(100_000); // 100us

        //check if OTP controller is idle (1)
        //OTP memory is idle
        loop {
            let sts = self.sb.secure014().read();
            if sts.otpctrl_sts().bit() && sts.otpmemory_sts().bit() {
                return true;
            }
            tries -= 1;
            if tries == 0 {
                return false;
            }
            delay.delay_ns(1_000); // 1us between polls so 1000 retries span real time
        }
    }

    pub fn otp_write(&self, otp_addr: u32, data: u32) -> bool {
        self.sb.secure010().write(|w| unsafe { w.bits(otp_addr) });
        self.sb.secure020().write(|w| unsafe { w.bits(data) });
        self.sb
            .secure004()
            .write(|w| unsafe { w.bits(OTP_WRITE_CMD) });
        self.wait_complete()
    }

    pub fn otp_soak(&self, otp_soak: OtpSoak) {
        match otp_soak {
            OtpSoak::Default => {
                // MRA/MRB/MR register writes are best-effort timing setup; errors are not actionable
                let _ = self.otp_write(SOAK_PROG_DEFAULT[0].address, SOAK_PROG_DEFAULT[0].data);
                let _ = self.otp_write(SOAK_PROG_DEFAULT[1].address, SOAK_PROG_DEFAULT[1].data);
                let _ = self.otp_write(SOAK_PROG_DEFAULT[2].address, SOAK_PROG_DEFAULT[2].data);
            }
            OtpSoak::NormalProg => {
                let _ = self.otp_write(SOAK_PROG_NORMAL[0].address, SOAK_PROG_NORMAL[0].data);
                let _ = self.otp_write(SOAK_PROG_NORMAL[1].address, SOAK_PROG_NORMAL[1].data);
                let _ = self.otp_write(SOAK_PROG_NORMAL[2].address, SOAK_PROG_NORMAL[2].data);
                self.sb
                    .secure008()
                    .write(|w| unsafe { w.bits(OTP_TIMING_200US) });
            }
            OtpSoak::SoakProg => {
                let _ = self.otp_write(SOAK_PROG_SOAK[0].address, SOAK_PROG_SOAK[0].data);
                let _ = self.otp_write(SOAK_PROG_SOAK[1].address, SOAK_PROG_SOAK[1].data);
                let _ = self.otp_write(SOAK_PROG_SOAK[2].address, SOAK_PROG_SOAK[2].data);
                self.sb
                    .secure008()
                    .write(|w| unsafe { w.bits(OTP_TIMING_600US) });
            }
        }
        self.wait_complete();
    }
    /// Read 2 DWORD data
    pub fn otp_read_data(&self, otp_addr: u32, buffer: &mut [u32]) -> Result<(), OtpError> {
        if buffer.len() < 2 {
            return Err(OtpError::ReadFailed);
        }
        self.sb.secure010().write(|w| unsafe { w.bits(otp_addr) });
        self.sb
            .secure004()
            .write(|w| unsafe { w.bits(OTP_READ_CMD) });
        if !self.wait_complete() {
            return Err(OtpError::ReadFailed);
        }
        buffer[0] = self.sb.secure020().read().bits();
        buffer[1] = self.sb.secure024().read().bits();
        Ok(())
    }
    /// Read configuration register at raw address
    fn otp_read_conf(&self, addr: u32) -> Result<u32, OtpError> {
        self.sb.secure010().write(|w| unsafe { w.bits(addr) });
        self.sb
            .secure004()
            .write(|w| unsafe { w.bits(OTP_READ_CMD) });
        if !self.wait_complete() {
            return Err(OtpError::ReadFailed);
        }
        let data = self.sb.secure020().read().bits();
        Ok(data)
    }

    /// This function does OTPCONFGX read.
    /// # Arguments
    ///
    /// * `reg_idx` - OTPCFG register number eg. 1-OTPCFG1
    ///   The address offset: 0x800 (OTPFCG0),0x802 (OTPFCG1)
    ///   A00(OTPCFG8-15), C00(OTPCFG16-31)
    fn otp_read_conf_idx(&self, reg_idx: u32) -> Result<u32, OtpError> {
        let mut addr = OTP_CONF_OFFSET;

        addr |= (reg_idx / 8) * 0x200;
        addr |= (reg_idx % 8) * 0x2;

        self.otp_read_conf(addr)
    }

    fn otp_prog(&mut self, otp_addr: u32, prog_bit: u32) -> Result<(), OtpError> {
        if self.otp_write(0, prog_bit) {
            self.sb.secure010().write(|w| unsafe { w.bits(otp_addr) });
            self.sb.secure020().write(|w| unsafe { w.bits(prog_bit) });
            self.sb
                .secure004()
                .write(|w| unsafe { w.bits(OTP_PROG_CMD) });
            if self.wait_complete() {
                Ok(())
            } else {
                Err(OtpError::Timeout)
            }
        } else {
            otp_error!(self.logger, "otp_prog failed");
            Err(OtpError::WriteFailed)
        }
    }
    /// Program one bit, inverting for the even/odd address polarity convention
    fn otp_prog_bit_helper(
        &mut self,
        value: u32,
        address: u32,
        bit_offset: u32,
    ) -> Result<(), OtpError> {
        let mut prog_bit: u32 = 0;

        if address & 0x1 == 0 {
            //even address, default data is 0x0
            if value != 0 {
                prog_bit = !(0x1 << bit_offset);
            }
        } else {
            //odd address, default data is 0xffff_ffff
            if value == 0 {
                prog_bit = 0x1 << bit_offset;
            }
        }
        if prog_bit != 0 {
            self.otp_prog(address, prog_bit)
        } else {
            Ok(())
        }
    }
    //lock registers
    pub fn otp_lock_reg(&self) {
        self.sb
            .secure000()
            .write(|w| unsafe { w.prot_key().bits(1) });
    }
    pub fn otp_unlock_reg(&self) {
        self.sb
            .secure000()
            .write(|w| unsafe { w.prot_key().bits(OTP_PASSWD) });
    }
    fn verify_bit(&mut self, value: u32, otp_addr: u32, bit_offset: u32) -> Result<(), OtpError> {
        let mut ret: [u32; 2] = [0, 0];
        let mut success: bool = false;
        let addr: u32 = if otp_addr & 0x1 == 0 {
            otp_addr
        } else {
            //make it even
            otp_addr - 1
        };
        self.otp_read_data(addr, &mut ret)?;

        if otp_addr & 0x1 == 0 {
            if (ret[0] >> bit_offset) & 1 == value {
                success = true;
            }
        } else {
            //Odd address takes takes the 2nd Dword
            if (ret[1] >> bit_offset) & 1 == value {
                success = true;
            }
        }
        if !success {
            return Err(OtpError::VerificationFailed);
        }
        Ok(())
    }

    /// Program one bit with normal→soak retry loop
    pub(crate) fn otp_prog_dc_b(
        &mut self,
        value: u32,
        address: u32,
        bit_offset: u32,
    ) -> Result<(), OtpError> {
        let mut pass: bool = false;

        self.otp_soak(OtpSoak::NormalProg);
        self.otp_prog_bit_helper(value, address, bit_offset)?;
        for _i in 0..OTP_OP_RETRIES {
            if self.verify_bit(value, address, bit_offset).is_err() {
                self.otp_soak(OtpSoak::SoakProg);
                self.otp_prog_bit_helper(value, address, bit_offset)?;
                if self.verify_bit(value, address, bit_offset).is_ok() {
                    self.otp_soak(OtpSoak::NormalProg);
                } else {
                    break;
                }
            } else {
                pass = true;
                break;
            }
        }
        if !pass {
            return Err(OtpError::Timeout);
        }
        Ok(())
    }
    ///
    /// program a DWORD. will do verification after program a DWORD for efficiency
    /// * `ignore` - bit position mask. don't program the bits shown in the mask
    ///
    fn otp_prog_dw(&mut self, value: u32, ignore: u32, address: u32) -> Result<(), OtpError> {
        let mut bit_value: u32;
        let mut prog_bit: u32;
        //1-bit at a time
        for bit_pos in 0..32 {
            if (ignore >> bit_pos) & 0x1 == 0x1 {
                //don't do anything
                continue;
            }
            bit_value = (value >> bit_pos) & 0x1;
            //inverse
            if address & 0x1 == 0 {
                if bit_value == 0x1 {
                    prog_bit = !(0x1 << bit_pos);
                } else {
                    continue;
                }
            } else if bit_value == 0x1 {
                continue;
            } else {
                prog_bit = 0x1 << bit_pos;
            }
            self.otp_prog(address, prog_bit)?;
        }
        Ok(())
    }

    /// Verify one DWORD against `data`; `*compare` encoding differs by parity: even→0/xor, odd→!0/!xor.
    pub(crate) fn verify_dw(
        &self,
        address: u32,
        data: u32,
        ignore: u32,
        compare: &mut u32,
    ) -> bool {
        let mut ret: [u32; 2] = [0, 0];

        let otp_addr = address & !(1 << 15);

        let addr = if otp_addr & 0x1 == 0 {
            otp_addr
        } else {
            otp_addr - 1
        };
        if self.otp_read_data(addr, &mut ret).is_err() {
            return false;
        }
        if otp_addr & 0x1 == 0 {
            //retrieve 1st DWORD
            if (data & !ignore) == (ret[0] & !ignore) {
                *compare = 0;
                return true;
            }
            *compare = data ^ ret[0];
            false
        } else {
            //odd address: retrieve 2nd DWORD
            if (data & !ignore) == (ret[1] & !ignore) {
                *compare = !0;
                return true;
            }
            *compare = !(data ^ ret[1]);
            false
        }
    }

    /// Verify up to 2 DWORDs; delegates to `verify_dw` when `num_dw == 1`
    pub(crate) fn verify_2dw(
        &mut self,
        address: u32,
        value: &[u32],
        ignore: &[u32],
        num_dw: u32,
        compare: &mut [u32],
    ) -> bool {
        let mut ret: [u32; 2] = [0, 0];

        let otp_addr = address & !(1 << 15);

        if num_dw == 1 {
            return self.verify_dw(address, value[0], ignore[0], &mut compare[0]);
        } else if num_dw == 2 {
            //otp_addr should already be even
            if self.otp_read_data(otp_addr, &mut ret).is_err() {
                return false;
            }
            if (value[0] & !ignore[0]) == (ret[0] & !ignore[0])
                && (value[1] & !ignore[1]) == (ret[1] & !ignore[1])
            {
                compare[0] = 0;
                compare[1] = !0;
                return true;
            }
            compare[0] = value[0] ^ ret[0];
            compare[1] = !(value[1] ^ ret[1]);
        }
        false
    }

    /// Return false if `buffer_data` would require programming a bit in the wrong direction
    pub(crate) fn is_program_data_valid(addr: u32, otp_data: u32, buffer_data: u32) -> bool {
        for i in 0..32 {
            if addr & 0x1 == 0 {
                //even location, default is 0x0000_0000
                //only able to write b'1
                //it's already b'1, can't program it b'0
                if ((otp_data >> i) & 0x1) == 1 && ((buffer_data >> i) & 0x1) == 0 {
                    return false;
                }
            } else if ((otp_data >> i) & 0x1) == 0 && ((buffer_data >> i) & 0x1) == 1 {
                return false;
            }
        }
        true
    }
    /// Program and verify 2 DWORDs with normal→soak retry, skipping already-matching words
    pub fn otp_prog_verify_2dw(
        &mut self,
        address: u32,
        otp_data: &[u32],
        buffer: &[u32],
        ignore: &[u32],
    ) -> Result<(), OtpError> {
        let mut ignore_mask: [u32; 2] = [0, 0];
        let mut compare: [u32; 2] = [0, 0];
        let mut pass: bool;
        let mut verify_size = 0;
        ignore_mask[0] = ignore[0];
        ignore_mask[1] = ignore[1];
        let data0_masked = otp_data[0] & !ignore_mask[0];
        let buf0_masked = buffer[0] & !ignore_mask[0];
        let data1_masked = otp_data[1] & !ignore_mask[1];
        let buf1_masked = buffer[1] & !ignore_mask[1];
        //if bits to be programmed is the same as
        //already programmed bits, no need to program
        if data0_masked == buf0_masked {
            ignore_mask[0] = 0xffff_ffff;
        }
        if data1_masked == buf1_masked {
            ignore_mask[1] = 0xffff_ffff;
        }

        //check if data to be written is the same on otp
        if data0_masked == buf0_masked && data1_masked == buf1_masked {
            otp_debug!(
                self.logger,
                "otp_prog_verify_2dw: data is the same, no need to program"
            );
            return Ok(());
        }
        if ignore_mask[0] != 0xffff_ffff
            && !Self::is_program_data_valid(address, data0_masked, buf0_masked)
        {
            return Err(OtpError::WriteFailed);
        }
        if ignore_mask[1] != 0xffff_ffff
            && !Self::is_program_data_valid(address + 1, data1_masked, buf1_masked)
        {
            return Err(OtpError::WriteFailed);
        }
        self.otp_soak(OtpSoak::NormalProg);

        //ignore
        if ignore_mask[0] != 0xffff_ffff {
            self.otp_prog_dw(buffer[0], ignore_mask[0], address)?;
            verify_size += 1;
        }
        if ignore_mask[1] != 0xffff_ffff {
            self.otp_prog_dw(buffer[1], ignore_mask[1], address + 1)?;
            verify_size += 1;
        }
        pass = false;
        for _j in 0..OTP_OP_RETRIES {
            if self.verify_2dw(address, buffer, &ignore_mask, verify_size, &mut compare) {
                pass = true;
                break;
            }
            self.otp_soak(OtpSoak::SoakProg);
            if compare[0] != 0 {
                self.otp_prog_dw(compare[0], ignore_mask[0], address)?;
            }
            if verify_size == 2 && compare[1] != !0 {
                self.otp_prog_dw(compare[1], ignore_mask[1], address + 1)?;
            }
            if self.verify_2dw(address, buffer, &ignore_mask, verify_size, &mut compare) {
                pass = true;
                break;
            }
            self.otp_soak(OtpSoak::NormalProg);
        }
        if !pass {
            self.otp_soak(OtpSoak::Default);
            return Err(OtpError::WriteFailed);
        }
        Ok(())
    }

    /// Verify a DWORD then reprogram mismatched bits with soak retry; returns true on success
    pub fn otp_prog_verify_retry(&mut self, addr: u32, data: u32, ignore: u32) -> bool {
        let mut compare: u32 = 0;
        let mut pass: bool = false;

        for _j in 0..OTP_OP_RETRIES {
            if self.verify_dw(addr, data, ignore, &mut compare) {
                pass = true;
                break;
            }
            self.otp_soak(OtpSoak::SoakProg);
            if let Err(_e) = self.otp_prog_dw(compare, ignore, addr) {
                pass = false;
                break;
            }
            if self.verify_dw(addr, data, ignore, &mut compare) {
                pass = true;
                break;
            }
            self.otp_soak(OtpSoak::NormalProg);
        }
        pass
    }
    //lock otp memory
    pub fn otp_lock_mem(&mut self) -> Result<(), OtpError> {
        if self.is_otp_locked() {
            self.locked = true;
            return Ok(());
        }
        self.otp_unlock_reg();
        self.otp_soak(OtpSoak::NormalProg);
        self.otp_prog_dw(OTP_MEM_LOCK_ENABLE, 0, OTP_CONF_OFFSET)?;
        self.otp_soak(OtpSoak::Default);
        self.otp_lock_reg();
        if !self.is_otp_locked() {
            return Err(OtpError::LockFailed);
        }
        self.locked = true;
        Ok(())
    }
    pub fn is_otp_locked(&self) -> bool {
        let otp_conf: u32 = self.otp_read_conf_idx(0).unwrap_or_default();
        otp_conf & OTP_MEM_LOCK_ENABLE == OTP_MEM_LOCK_ENABLE
    }
    pub fn is_key_protected(&self) -> bool {
        let otp_conf: u32 = self.otp_read_conf_idx(0).unwrap_or_default();
        otp_conf & OTP_KEY_PROT_ENABLE == OTP_KEY_PROT_ENABLE
    }

    pub fn update_prot_info(&self, session: &mut SessionInfo) {
        session.chip_version = self.chip_version();
        match session.chip_version {
            AspeedChipVersion::Ast1060A1 => {
                session.version_name = *b"AST1060A1\0";
            }
            AspeedChipVersion::Ast1060A2 => {
                session.version_name = *b"AST1060A2\0";
            }
            _ => {
                session.version_name = *b"ASUnknown\0";
            }
        }
        let otp_conf: u32 = self.otp_read_conf_idx(0).unwrap_or_default();
        session.protection_status.memory_locked =
            otp_conf & OTP_MEM_LOCK_ENABLE == OTP_MEM_LOCK_ENABLE;
        session.protection_status.strap_protected =
            otp_conf & OTP_STRAP_PROT_ENABLE == OTP_STRAP_PROT_ENABLE;
        session.protection_status.user_ecc_protected =
            otp_conf & OTP_USER_ECC_PROT_ENABLE == OTP_USER_ECC_PROT_ENABLE;

        session.protection_status.security_protected =
            otp_conf & OTP_SECURE_PROT_ENABLE == OTP_SECURE_PROT_ENABLE;
        let mut secure_size = otp_conf >> OTP_SECURE_SIZE_BIT_POS;
        if secure_size != 0 {
            secure_size = (secure_size & OTP_SECURE_SIZE_MASK) << 5;
        }
        session.protection_status.security_size = secure_size;
    }
    pub fn get_sw_revision(&self, sw_rid: &mut [u32; 2]) {
        sw_rid[0] = self.sb.secure068().read().bits();
        sw_rid[1] = self.sb.secure06c().read().bits();
    }
    pub fn get_tool_version(&self) -> &str {
        OTP_VER
    }
    pub fn get_key_count(&self) -> u8 {
        self.sb.secure078().read().sec_boot_key_number_regs().bits()
    }
    #[allow(clippy::needless_range_loop)]
    pub fn otp_strap_status(&self, os: &mut [StrapStatus]) -> Result<(), OtpError> {
        let mut otpstrap_raw: [u32; 2] = [0; 2];

        for j in 0..64 {
            os[j].value = false;
            os[j].remaining_writes = 6;
            os[j].writable_option = 0xff;
            os[j].protected = false;
        }
        let strap_end: usize = 28; // Final strap address to process

        self.otp_soak(OtpSoak::Default);

        for i in (16..strap_end).step_by(2) {
            let option = u8::try_from((i - 16) / 2).unwrap();

            otpstrap_raw[0] = self.otp_read_conf_idx(i.try_into().unwrap())?;
            otpstrap_raw[1] = self.otp_read_conf_idx((i + 1).try_into().unwrap())?;
            for j in 0..32 {
                let bit_value = ((otpstrap_raw[0] >> j) & 0x1) as u8;

                if bit_value == 0 && os[j].writable_option == 0xff {
                    os[j].writable_option = option;
                }
                if bit_value == 1 {
                    os[j].remaining_writes -= 1;
                }
                os[j].value ^= bit_value != 0;
                os[j].options[option as usize] = bit_value;
            }

            for j in 32..64 {
                let bit_value = ((otpstrap_raw[1] >> (j - 32)) & 0x1) as u8;

                if bit_value == 0 && os[j].writable_option == 0xff {
                    os[j].writable_option = option;
                }
                if bit_value == 1 {
                    os[j].remaining_writes -= 1;
                }
                os[j].value ^= bit_value != 0;
                os[j].options[option as usize] = bit_value;
            }
        }
        otpstrap_raw[0] = self.otp_read_conf_idx(30)?;
        otpstrap_raw[1] = self.otp_read_conf_idx(31)?;

        for j in 0..32 {
            if (otpstrap_raw[0] >> j) & 0x1 == 1 {
                os[j].protected = true;
            }
        }

        for j in 32..64 {
            if (otpstrap_raw[1] >> (j - 32)) & 0x1 == 1 {
                os[j].protected = true;
            }
        }
        Ok(())
    }

    /// Read from the OTP data region; `offset` must be 4-DWORD aligned and `buffer` must have even length
    pub fn aspeed_otp_read_data(&self, offset: usize, buffer: &mut [u32]) -> Result<(), OtpError> {
        let mut temp: [u32; 2] = [0, 0];
        let cdw_len: usize = buffer.len();
        if cdw_len + offset > OTP_MEM_LIMIT_DATA {
            return Err(OtpError::BoundaryError);
        }
        if offset & 0x3 != 0 {
            return Err(OtpError::AlignmentError);
        }
        if cdw_len % 2 != 0 {
            return Err(OtpError::InvalidBufSize);
        }
        for i in (offset..offset + cdw_len).step_by(2) {
            let idx = i - offset;
            match self.otp_read_data(u32::try_from(i).unwrap(), &mut temp) {
                Ok(()) => {
                    buffer[idx] = temp[0];
                    buffer[idx + 1] = temp[1];
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Read from the OTP configuration region; `offset` is in DWORDs from OTPCFG0
    pub fn aspeed_otp_read_conf(&self, offset: usize, buffer: &mut [u32]) -> Result<(), OtpError> {
        let mut result: Result<(), OtpError> = Ok(());
        let cdw_len = buffer.len();
        if cdw_len + offset > 32 {
            return Err(OtpError::BoundaryError);
        }
        self.otp_unlock_reg();
        self.otp_soak(OtpSoak::Default);
        for i in offset..offset + cdw_len {
            let idx = i - offset;
            buffer[idx] = match self.otp_read_conf_idx(u32::try_from(i).unwrap()) {
                Ok(value) => value,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
        }
        self.otp_lock_reg();
        result
    }
    /// Read SCU protection region (OTPCFG28/29) into buffer
    pub fn aspeed_otp_read_scuprot(
        &self,
        offset: usize,
        buffer: &mut [u32],
    ) -> Result<(), OtpError> {
        let mut result: Result<(), OtpError> = Ok(());
        let cdw_len = buffer.len();

        if cdw_len + offset > 2 {
            return Err(OtpError::BoundaryError);
        }
        self.otp_unlock_reg();

        for i in offset..offset + cdw_len {
            let idx = i - offset;
            buffer[idx] = match self.otp_read_conf_idx(28 + u32::try_from(i).unwrap()) {
                Ok(value) => value,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
        }
        self.otp_lock_reg();
        result
    }

    /// Write `data` into the OTP data region starting at `offset`
    pub fn otp_prog_data(&mut self, offset: usize, data: &[u32]) -> Result<(), OtpError> {
        let mut result: Result<(), OtpError> = Ok(());
        let ignore: [u32; 2] = [0, 0];
        let cdw_len = data.len();

        if cdw_len + offset > OTP_MEM_LIMIT_DATA {
            return Err(OtpError::BoundaryError);
        }
        if offset & 0x3 != 0 {
            return Err(OtpError::AlignmentError);
        }
        self.otp_unlock_reg();

        let mut scratch: [u32; 2] = [0, 0];
        for i in (offset..offset + cdw_len).step_by(2) {
            let idx0 = i - offset;
            result = self.otp_read_data(u32::try_from(i).unwrap(), &mut scratch);
            if result.is_err() {
                otp_debug!(
                    self.logger,
                    "otp_prog_data: read fail {:?}",
                    result.unwrap_err()
                );
                break;
            }
            otp_debug!(
                self.logger,
                "otp_prog_data: idx0={:}, idx1={:}",
                idx0,
                idx0 + 1
            );
            result = self.otp_prog_verify_2dw(
                u32::try_from(i).unwrap(),
                &scratch,
                &data[idx0..idx0 + 2],
                &ignore,
            );
            if result.is_err() {
                break;
            }
        }

        self.otp_soak(OtpSoak::Default);
        self.otp_lock_reg();
        result
    }

    /// Extract the requested value of strap bit `i` from the caller's `strap` words.
    fn strap_target_bit(strap: &[u32], start_bit: usize, i: usize) -> u32 {
        if i < 32 {
            let offset = u32::try_from(i).unwrap();
            (strap[0] >> (offset - u32::try_from(start_bit).unwrap())) & 0x1
        } else {
            let offset = u32::try_from(i - 32).unwrap();
            if i - start_bit < 32 {
                (strap[0] >> u32::try_from(i - start_bit).unwrap()) & 0x1
            } else {
                (strap[1] >> (offset - u32::try_from(start_bit).unwrap())) & 0x1
            }
        }
    }

    /// Program strap bits; every unprotected bit that differs from its current value is burned.
    ///
    /// Fuses cannot be un-burned, so this validates the whole request first: if any
    /// bit that needs to change is protected or out of write attempts, it returns
    /// `RegionProtected`/`WriteExhausted` before burning anything. Once the burn pass
    /// starts, a `Timeout`/hardware verify failure can still leave a partial write.
    #[allow(clippy::needless_range_loop)]
    pub fn otp_prog_strap(&mut self, start_bit: usize, strap: &[u32]) -> Result<(), OtpError> {
        if start_bit > 63 {
            return Err(OtpError::InvalidAddress);
        }

        let mut os: [StrapStatus; 64] = [StrapStatus {
            value: false,
            protected: false,
            options: [0; 7],
            remaining_writes: 6,
            writable_option: 0xff,
        }; 64];
        self.otp_strap_status(&mut os)?;

        // Pre-flight: refuse the entire request before burning any fuse if a bit that
        // must change is protected or has no write attempts left. The strap snapshot is
        // read once and unaffected by the burns, so this decision is exact.
        let mut count_prot: u32 = 0;
        let mut count_cant_write: u32 = 0;
        for i in start_bit..64 {
            if Self::strap_target_bit(strap, start_bit, i) == u32::from(os[i].value) {
                continue;
            }
            if os[i].protected {
                count_prot += 1;
            } else if os[i].remaining_writes == 0 {
                count_cant_write += 1;
            }
        }
        if count_prot > 0 {
            return Err(OtpError::RegionProtected);
        }
        if count_cant_write > 0 {
            return Err(OtpError::WriteExhausted);
        }

        // Every differing bit is now known to be writable; burn them.
        for i in start_bit..64 {
            let bit = Self::strap_target_bit(strap, start_bit, i);
            if bit == u32::from(os[i].value) {
                otp_debug!(self.logger, "otp_prog_strap: bit {:} no need to program", i);
                continue;
            }
            otp_debug!(
                self.logger,
                "otp_prog_strap: program bit {:} from {:} to {:}",
                i,
                u32::from(os[i].value),
                bit
            );
            let offset = u32::try_from(i % 32).unwrap();
            let slot = if i < 32 {
                u32::from(os[i].writable_option) * 2 + 16
            } else {
                u32::from(os[i].writable_option) * 2 + 17
            };
            let prog_address = OTP_CONF_OFFSET | ((slot / 8) * 0x200) | ((slot % 8) * 0x2);
            self.otp_prog_dc_b(1, prog_address, offset)?;
        }
        self.otp_soak(OtpSoak::Default);
        Ok(())
    }

    /// Program OTP configuration registers starting at `start_conf`
    pub fn otp_prog_conf(&mut self, start_conf: usize, conf: &[u32]) -> Result<(), OtpError> {
        let mut result: Result<(), OtpError> = Ok(());
        let conf_ignore: u32 = 0;
        let mut otp_conf: u32;
        let mut pass: bool = true;
        let mut addr: usize;
        let mut data_masked: u32;
        let mut buf_masked: u32;
        let cdw_len = conf.len();

        if cdw_len + start_conf > 32 {
            return Err(OtpError::BoundaryError);
        }
        self.otp_unlock_reg();
        self.otp_soak(OtpSoak::Default);
        for i in start_conf..start_conf + cdw_len {
            //from 0
            let idx = i - start_conf;
            //read conf from OTP
            otp_conf = match self.otp_read_conf_idx(u32::try_from(i).unwrap()) {
                Ok(value) => value,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
            data_masked = otp_conf & !conf_ignore;
            buf_masked = conf[idx] & !conf_ignore;
            addr = REGION_INFO[AspeedOtpRegion::Configuration as usize].start;
            addr |= (i / 8) * 0x200;
            addr |= (i % 8) * 0x2;
            otp_debug!(self.logger, "otp_prog_conf: addr = {:#x}", addr);
            if data_masked == buf_masked {
                pass = true;
                continue;
            }
            self.otp_soak(OtpSoak::NormalProg);
            result = self.otp_prog_dw(conf[idx], conf_ignore, u32::try_from(addr).unwrap());
            if result.is_err() {
                break;
            }
            pass = self.otp_prog_verify_retry(u32::try_from(addr).unwrap(), conf[idx], conf_ignore);

            if !pass {
                break;
            }
        }
        self.otp_soak(OtpSoak::Default);
        self.otp_lock_reg();
        if !pass {
            return Err(OtpError::WriteFailed);
        }
        result
    }

    /// Program SCU protection registers
    #[allow(clippy::needless_range_loop)]
    pub fn otp_prog_scu_protect(&mut self, start: usize, otp_scu: &[u32]) -> Result<(), OtpError> {
        let mut scu_pro: [u32; 2] = [0; 2];
        let ignore: u32 = 0;
        let mut data_masked: u32;
        let mut buf_masked: u32;
        let mut addr: usize;
        let mut pass: bool = false;
        let scupro_start = REGION_INFO[AspeedOtpRegion::ScuProtection as usize].start;
        let cdw_size = otp_scu.len();
        let total_size = REGION_INFO[AspeedOtpRegion::ScuProtection as usize].cdw_size;

        if start + cdw_size > total_size {
            return Err(OtpError::BoundaryError);
        }
        scu_pro[0] = self.otp_read_conf_idx(28)?;
        scu_pro[1] = self.otp_read_conf_idx(29)?;
        self.otp_unlock_reg();
        self.otp_soak(OtpSoak::Default);

        for i in start..start + cdw_size {
            let idx = i - start;
            data_masked = scu_pro[idx] & !ignore;
            buf_masked = otp_scu[idx] & !ignore;
            addr = scupro_start + i * 2;
            if data_masked == buf_masked {
                pass = true;
                continue;
            }
            self.otp_soak(OtpSoak::Default);
            self.otp_prog_dw(otp_scu[idx], ignore, u32::try_from(addr).unwrap())?;

            pass = self.otp_prog_verify_retry(u32::try_from(addr).unwrap(), otp_scu[idx], ignore);

            if !pass {
                break;
            }
        }
        self.otp_soak(OtpSoak::Default);
        self.otp_lock_reg();
        if !pass {
            return Err(OtpError::WriteFailed);
        }
        Ok(())
    }

    pub fn total_capacity(&self) -> usize {
        let mut cdw_size: usize = 0;

        cdw_size += REGION_INFO[AspeedOtpRegion::Data as usize].cdw_size;
        cdw_size += REGION_INFO[AspeedOtpRegion::Configuration as usize].cdw_size;
        cdw_size << 2
    }

    pub fn region_capacity(&self, region: AspeedOtpRegion) -> usize {
        REGION_INFO[region as usize].cdw_size << 2
    }
    #[allow(clippy::unused_self)]
    pub fn region_alignment(&self, region: AspeedOtpRegion) -> usize {
        REGION_INFO[region as usize].alignment
    }
    #[allow(clippy::match_same_arms)]
    fn is_region_protected(&self, region: AspeedOtpRegion) -> Result<bool, OtpError> {
        let mut protected: bool = false;

        self.otp_unlock_reg();
        let otp_conf: u32 = self.otp_read_conf_idx(0)?;
        self.otp_lock_reg();
        match region {
            AspeedOtpRegion::Data => {
                if otp_conf & OTP_USER_ECC_PROT_ENABLE == OTP_USER_ECC_PROT_ENABLE
                    && otp_conf & OTP_SECURE_PROT_ENABLE == OTP_SECURE_PROT_ENABLE
                {
                    protected = true;
                }
            }
            AspeedOtpRegion::Configuration => {
                if otp_conf & OTP_CONF_PROT_ENABLE == OTP_CONF_PROT_ENABLE {
                    protected = true;
                }
            }
            AspeedOtpRegion::Strap => {
                if otp_conf & OTP_STRAP_PROT_ENABLE == OTP_STRAP_PROT_ENABLE {
                    protected = true;
                }
            }
            AspeedOtpRegion::ScuProtection => {}
        }
        Ok(protected)
    }

    /// Enable write protection for a specific OTP region
    pub fn enable_region_protection(&mut self, region: AspeedOtpRegion) -> Result<(), OtpError> {
        let mut value: [u32; 1] = [0; 1];

        if self.is_region_protected(region) == Ok(true) {
            return Ok(());
        }
        match region {
            AspeedOtpRegion::Data => {
                value[0] = OTP_USER_ECC_PROT_ENABLE | OTP_SECURE_PROT_ENABLE;
            }
            AspeedOtpRegion::Configuration => {
                value[0] = OTP_CONF_PROT_ENABLE;
            }
            AspeedOtpRegion::Strap => {
                value[0] = OTP_STRAP_PROT_ENABLE;
            }
            AspeedOtpRegion::ScuProtection => {}
        }
        self.otp_prog_conf(0, &value)
    }
}

#[cfg(test)]
mod tests {
    use super::common::{NoOpLogger, ProtectionStatus};
    use super::*;

    #[test]
    fn region_info_ordering_matches_discriminants() {
        assert_eq!(
            REGION_INFO[AspeedOtpRegion::Data as usize].region_type,
            AspeedOtpRegion::Data
        );
        assert_eq!(
            REGION_INFO[AspeedOtpRegion::Configuration as usize].region_type,
            AspeedOtpRegion::Configuration
        );
        assert_eq!(
            REGION_INFO[AspeedOtpRegion::Strap as usize].region_type,
            AspeedOtpRegion::Strap
        );
        assert_eq!(
            REGION_INFO[AspeedOtpRegion::ScuProtection as usize].region_type,
            AspeedOtpRegion::ScuProtection
        );
    }

    // Tests only touch the first 10 words; buffer must cover the full struct for the pointer cast to be valid.
    const SECURE_WORDS: usize = core::mem::size_of::<SbRegBlock>().div_ceil(4);
    static FAKE_SECURE: [u32; SECURE_WORDS] = [0u32; SECURE_WORDS];
    static FAKE_SCU: [u32; 16] = [0u32; 16];

    // Read-only paths only; writing the non-mut FAKE_SECURE static is UB.
    unsafe fn make_ctrl() -> OtpController<NoOpLogger> {
        OtpController {
            sb: unsafe { &*(FAKE_SECURE.as_ptr() as *const SbRegBlock) },
            scu_base: FAKE_SCU.as_ptr() as *const _,
            locked: false,
            logger: NoOpLogger,
        }
    }

    // Secure register buffer indices (byte offset / 4):
    //   buf[5]  = secure014: status bits — bit1=otpmemory_sts, bit2=otpctrl_sts
    //   buf[8]  = secure020: data word 0 returned by otp_read_data
    const STATUS_IDX: usize = 5; // secure014
    const DATA0_IDX: usize = 8; // secure020
    const STATUS_IDLE: u32 = 0x6; // otpmemory_sts(bit1)=1, otpctrl_sts(bit2)=1

    // Returns a fresh mutable 'static buffer and a controller pointing at it.
    // Uses Box::leak so the buffer is 'static (required by the sb field type)
    // without sharing state between tests. Memory is reclaimed when the test
    // process exits.
    unsafe fn make_ctrl_buf() -> (OtpController<NoOpLogger>, &'static mut [u32; SECURE_WORDS]) {
        let buf: &'static mut [u32; SECURE_WORDS] = Box::leak(Box::new([0u32; SECURE_WORDS]));
        buf[STATUS_IDX] = STATUS_IDLE;
        let ctrl = OtpController {
            sb: unsafe { &*(buf.as_ptr() as *const SbRegBlock) },
            scu_base: FAKE_SCU.as_ptr() as *const _,
            locked: false,
            logger: NoOpLogger,
        };
        (ctrl, buf)
    }

    // otp_prog_verify_2dw -----------------------------------------------------

    #[test]
    fn otp_prog_verify_2dw_skip_when_already_matches() {
        // otp_data == buffer under the mask → early Ok(()) before any soak.
        // Both dwords equal: data0_masked==buf0_masked AND data1_masked==buf1_masked.
        let mut ctrl = unsafe { make_ctrl() };
        assert_eq!(
            ctrl.otp_prog_verify_2dw(0, &[0xABCD, 0x1234], &[0xABCD, 0x1234], &[0, 0]),
            Ok(())
        );
    }

    #[test]
    fn otp_prog_verify_2dw_rejects_invalid_direction() {
        // otp_data[0]=1, buffer[0]=0 at even addr=0 → is_program_data_valid(0,1,0)=false.
        // Returns Err(WriteFailed) before touching any soak register.
        let mut ctrl = unsafe { make_ctrl() };
        assert_eq!(
            ctrl.otp_prog_verify_2dw(0, &[1, 0], &[0, 0], &[0, 0]),
            Err(OtpError::WriteFailed)
        );
    }

    #[test]
    fn otp_prog_verify_2dw_one_dword_ok() {
        // dw1 needs programming (otp_data[1]=0xFFFF_FFFF→buffer[1]=0, valid on odd addr=1).
        // dw0 matches → ignore_mask[0]=0xFFFF_FFFF, verify_size=1.
        // verify_2dw(num_dw=1) delegates to verify_dw(address=0, buffer[0],
        // ignore_mask[0]=0xFFFF_FFFF, compare[0]): masking with !0xFFFF_FFFF=0 makes both
        // sides 0 regardless of what otp_prog_dw wrote to buf[8]. Trivially returns true.
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(
            ctrl.otp_prog_verify_2dw(0, &[7, 0xFFFF_FFFF], &[7, 0], &[0, 0]),
            Ok(())
        );
    }

    #[test]
    fn otp_prog_verify_2dw_verify_always_fails_returns_write_failed() {
        // Both dwords differ, both need programming (verify_size=2).
        // otp_prog_dw writes prog_bit to buf[8]; verify reads buf[8] expecting buffer[0].
        // prog_bit != buffer[0], so every verify iteration fails. After OTP_OP_RETRIES
        // exhausted, the method soaks to Default and returns Err(WriteFailed).
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(
            ctrl.otp_prog_verify_2dw(0, &[0, 0], &[1, 0], &[0, 0]),
            Err(OtpError::WriteFailed)
        );
    }

    // chip_version ------------------------------------------------------------
    // scu004 is at byte offset 4 → scu_buf[1]; scu014 at offset 0x14 → scu_buf[5].
    // Stack-local buffers are used so parallel tests don't race on a shared static.

    // otp_strap_status --------------------------------------------------------
    // All conf reads go through otp_read_conf_idx → secure020 (buf[8]).
    // Every option row (indices 16..28) and the protection rows (30, 31) read
    // the same buf[8]. Options 0–5 each see identical data.

    fn blank_straps() -> [StrapStatus; 64] {
        core::array::from_fn(|_| StrapStatus {
            value: false,
            options: [0u8; 7],
            remaining_writes: 6,
            writable_option: 0xff,
            protected: false,
        })
    }

    #[test]
    fn strap_status_all_zero_conf_sets_defaults() {
        // buf[8]=0: every bit is 0 across all 6 option rows.
        // remaining_writes stays 6 (never decremented).
        // writable_option for strap 0 → set to option 0 on first row (bit is 0).
        // value stays false (XOR of six 0s = 0). protected stays false.
        let (ctrl, _buf) = unsafe { make_ctrl_buf() };
        let mut os = blank_straps();
        assert_eq!(ctrl.otp_strap_status(&mut os), Ok(()));
        assert!(!os[0].value);
        assert_eq!(os[0].remaining_writes, 6);
        assert_eq!(os[0].writable_option, 0); // first option where bit was 0
        assert!(!os[0].protected);
    }

    // aspeed_otp_read_data / aspeed_otp_read_conf ----------------------------

    #[test]
    fn read_data_boundary_error() {
        let ctrl = unsafe { make_ctrl() };
        let mut buf = [0u32; 2];
        // offset=2047 + len=2 = 2049 > OTP_MEM_LIMIT_DATA(2048)
        assert_eq!(
            ctrl.aspeed_otp_read_data(2047, &mut buf),
            Err(OtpError::BoundaryError)
        );
    }

    #[test]
    fn read_data_alignment_error_offset_1() {
        let ctrl = unsafe { make_ctrl() };
        let mut buf = [0u32; 2];
        assert_eq!(
            ctrl.aspeed_otp_read_data(1, &mut buf),
            Err(OtpError::AlignmentError)
        );
    }

    #[test]
    fn read_data_aligned_offset_passes_boundary() {
        let (ctrl, _buf) = unsafe { make_ctrl_buf() };
        let mut out = [0u32; 2];
        // offset=4 is 4-byte aligned and within bounds; read succeeds
        assert_eq!(ctrl.aspeed_otp_read_data(4, &mut out), Ok(()));
    }

    #[test]
    fn read_conf_boundary_error() {
        let ctrl = unsafe { make_ctrl() };
        let mut buf = [0u32; 2];
        // offset=31 + len=2 = 33 > 32
        assert_eq!(
            ctrl.aspeed_otp_read_conf(31, &mut buf),
            Err(OtpError::BoundaryError)
        );
    }

    #[test]
    fn read_conf_success_fills_every_word() {
        // soak(Default) writes 0 into secure020, so each conf read returns 0;
        // a valid in-bounds read must overwrite every sentinel slot and return Ok.
        let (ctrl, _buf) = unsafe { make_ctrl_buf() };
        let mut out = [0xFFFF_FFFFu32; 2];
        assert_eq!(ctrl.aspeed_otp_read_conf(0, &mut out), Ok(()));
        assert_eq!(out, [0, 0]);
    }

    // otp_prog_scu_protect ----------------------------------------------------

    #[test]
    fn scu_protect_boundary_error() {
        let mut ctrl = unsafe { make_ctrl() };
        assert_eq!(
            ctrl.otp_prog_scu_protect(2, &[0]),
            Err(OtpError::BoundaryError)
        );
        assert_eq!(
            ctrl.otp_prog_scu_protect(0, &[0, 0, 0]),
            Err(OtpError::BoundaryError)
        );
    }

    #[test]
    fn scu_protect_already_matches_returns_ok() {
        // scu_pro[0] and [1] both read 0 from buf[8] after soak clears it.
        // otp_scu=[0, 0] matches → pass=true for both, Ok(()) without programming.
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(ctrl.otp_prog_scu_protect(0, &[0, 0]), Ok(()));
    }

    #[test]
    fn scu_protect_needs_programming_verify_fails() {
        // scu_pro[0]=0 (from buf[8] after soak), otp_scu[0]=1 → mismatch, prog runs.
        // Fake buffer: verify always fails → Err(WriteFailed).
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(
            ctrl.otp_prog_scu_protect(0, &[1]),
            Err(OtpError::WriteFailed)
        );
    }

    // otp_lock_mem / is_otp_locked -------------------------------------------
    // is_otp_locked reads otp_read_conf_idx(0) → secure020 (buf[8]).
    // OTP_MEM_LOCK_ENABLE = 1<<31.

    #[test]
    fn otp_lock_mem_already_locked_returns_ok() {
        // Pre-set lock bit → is_otp_locked() true on first call → fast-path Ok(()).
        let (mut ctrl, buf) = unsafe { make_ctrl_buf() };
        buf[DATA0_IDX] = OTP_MEM_LOCK_ENABLE;
        assert_eq!(ctrl.otp_lock_mem(), Ok(()));
        assert!(ctrl.locked);
    }

    #[test]
    fn otp_lock_mem_latch_fails_returns_lock_failed() {
        // buf[8] has no lock bit. otp_prog_dw writes to secure020 then otp_soak(Default)
        // zeroes it, so is_otp_locked() reads 0 → false → Err(LockFailed).
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(ctrl.otp_lock_mem(), Err(OtpError::LockFailed));
        assert!(!ctrl.locked);
    }

    #[test]
    fn chip_version_ast1060a1() {
        let mut scu_buf = [0u32; 16];
        scu_buf[1] = ID0_AST1060A1;
        scu_buf[5] = ID1_AST1060A1;
        let ctrl = unsafe {
            OtpController {
                sb: &*(FAKE_SECURE.as_ptr() as *const SbRegBlock),
                scu_base: scu_buf.as_ptr() as *const _,
                locked: false,
                logger: NoOpLogger,
            }
        };
        assert_eq!(ctrl.chip_version(), AspeedChipVersion::Ast1060A1);
    }

    #[test]
    fn chip_version_ast1060a2() {
        let mut scu_buf = [0u32; 16];
        scu_buf[1] = ID0_AST1060A2;
        scu_buf[5] = ID1_AST1060A2;
        let ctrl = unsafe {
            OtpController {
                sb: &*(FAKE_SECURE.as_ptr() as *const SbRegBlock),
                scu_base: scu_buf.as_ptr() as *const _,
                locked: false,
                logger: NoOpLogger,
            }
        };
        assert_eq!(ctrl.chip_version(), AspeedChipVersion::Ast1060A2);
    }

    #[test]
    fn chip_version_unknown() {
        let ctrl = unsafe { make_ctrl() }; // FAKE_SCU all-zeros → no ID match
        assert_eq!(ctrl.chip_version(), AspeedChipVersion::Unknown);
    }

    // update_prot_info --------------------------------------------------------
    // otp_read_conf_idx(0) reads secure020 (buf[8]) after a wait_complete on buf[5].

    #[test]
    fn update_prot_info_sets_version_name_and_flags() {
        let (ctrl, buf) = unsafe { make_ctrl_buf() };
        // otp_conf: bit31=mem_locked, bit25=strap_prot, bit23=user_ecc_prot,
        //           bit22=sec_prot, bits[21:16]=secure_size_field
        // Set mem_locked and strap_protected; leave others clear.
        buf[DATA0_IDX] = OTP_MEM_LOCK_ENABLE | OTP_STRAP_PROT_ENABLE;
        let mut session = SessionInfo {
            chip_version: AspeedChipVersion::Unknown,
            version_name: [0u8; 10],
            protection_status: ProtectionStatus::default(),
            tool_version: [0u8; 32],
            software_revision: 0,
            key_count: 0,
        };
        ctrl.update_prot_info(&mut session);
        assert_eq!(session.chip_version, AspeedChipVersion::Unknown);
        assert_eq!(&session.version_name, b"ASUnknown\0");
        assert!(session.protection_status.memory_locked);
        assert!(session.protection_status.strap_protected);
        assert!(!session.protection_status.user_ecc_protected);
        assert!(!session.protection_status.security_protected);
        assert_eq!(session.protection_status.security_size, 0);
    }

    // otp_prog_data ----------------------------------------------------------

    #[test]
    fn prog_data_boundary_error() {
        let mut ctrl = unsafe { make_ctrl() };
        assert_eq!(
            ctrl.otp_prog_data(OTP_MEM_LIMIT_DATA, &[0, 0]),
            Err(OtpError::BoundaryError)
        );
    }

    #[test]
    fn prog_data_alignment_error() {
        let mut ctrl = unsafe { make_ctrl() };
        assert_eq!(
            ctrl.otp_prog_data(1, &[0, 0]),
            Err(OtpError::AlignmentError)
        );
    }

    #[test]
    fn prog_data_verify_always_fails_returns_write_failed() {
        // word 0 needs a valid 0→1 burn; against the fake buffer verify never
        // matches, so retries exhaust and the whole program returns WriteFailed.
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(ctrl.otp_prog_data(0, &[1, 0]), Err(OtpError::WriteFailed));
    }

    #[test]
    fn update_prot_info_security_size_nonzero() {
        let (ctrl, buf) = unsafe { make_ctrl_buf() };
        // secure_size is the 6-bit field bits[21:16]. Set it to 0x05.
        // Expected: (0x05 & 0x3f) << 5 = 0xA0.
        buf[DATA0_IDX] = 0x05 << OTP_SECURE_SIZE_BIT_POS;
        let mut session = SessionInfo {
            chip_version: AspeedChipVersion::Unknown,
            version_name: [0u8; 10],
            protection_status: ProtectionStatus::default(),
            tool_version: [0u8; 32],
            software_revision: 0,
            key_count: 0,
        };
        ctrl.update_prot_info(&mut session);
        assert_eq!(session.protection_status.security_size, 0xA0);
    }

    // Error/boundary paths for the program helpers not otherwise exercised.

    #[test]
    fn prog_strap_invalid_address() {
        let mut ctrl = unsafe { make_ctrl() };
        assert_eq!(
            ctrl.otp_prog_strap(64, &[0, 0]),
            Err(OtpError::InvalidAddress)
        );
    }

    #[test]
    fn prog_strap_no_change_burns_nothing() {
        // Fake straps read all-zero; requesting all-zero means no bit differs, so the
        // burn pass never runs and the call succeeds without touching a fuse.
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(ctrl.otp_prog_strap(0, &[0, 0]), Ok(()));
    }

    #[test]
    fn prog_strap_needs_burn_reaches_prog_and_times_out() {
        // Bit 0 must change 0→1; it is neither protected nor exhausted, so pre-flight
        // passes and the burn runs. The fake buffer can't satisfy verify, so
        // otp_prog_dc_b exhausts retries → Timeout (proves the burn pass executed).
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(ctrl.otp_prog_strap(0, &[1, 0]), Err(OtpError::Timeout));
    }

    #[test]
    fn prog_strap_nonzero_start_reads_correct_word_bit() {
        // start_bit=16, strap packed relative to it: strap[0] bit 16 is absolute
        // strap 32, which must be burned. Against the fake buffer the burn can't
        // verify, so a correct driver reaches the burn pass and returns Timeout.
        // The old i-32 shift read strap[0] bit 0 (zero) for absolute bit 32, saw
        // nothing to change, and wrongly returned Ok — this test catches that.
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(
            ctrl.otp_prog_strap(16, &[1 << 16, 0]),
            Err(OtpError::Timeout)
        );
    }

    #[test]
    fn prog_conf_boundary_error() {
        let mut ctrl = unsafe { make_ctrl() };
        // start_conf=31 + len=2 = 33 > 32
        assert_eq!(
            ctrl.otp_prog_conf(31, &[0, 0]),
            Err(OtpError::BoundaryError)
        );
    }

    #[test]
    fn prog_conf_already_matches_returns_ok() {
        // soak(Default) leaves secure020=0, so each conf reads back 0; requesting 0
        // matches → no burn, pass stays true → Ok(()).
        let (mut ctrl, _buf) = unsafe { make_ctrl_buf() };
        assert_eq!(ctrl.otp_prog_conf(0, &[0, 0]), Ok(()));
    }

    #[test]
    fn read_scuprot_boundary_error() {
        let ctrl = unsafe { make_ctrl() };
        let mut buf = [0u32; 2];
        // offset=1 + len=2 = 3 > 2
        assert_eq!(
            ctrl.aspeed_otp_read_scuprot(1, &mut buf),
            Err(OtpError::BoundaryError)
        );
    }

    #[test]
    fn read_scuprot_success_returns_conf28_29() {
        // No soak on this path, so secure020 keeps its preset; OTPCFG28/29 both read it.
        let (ctrl, buf) = unsafe { make_ctrl_buf() };
        buf[DATA0_IDX] = 0xDEAD_BEEF;
        let mut out = [0u32; 2];
        assert_eq!(ctrl.aspeed_otp_read_scuprot(0, &mut out), Ok(()));
        assert_eq!(out, [0xDEAD_BEEF, 0xDEAD_BEEF]);
    }

    #[test]
    fn enable_region_protection_already_protected_returns_ok() {
        // Configuration region already protected → early Ok(()) before any program.
        let (mut ctrl, buf) = unsafe { make_ctrl_buf() };
        buf[DATA0_IDX] = OTP_CONF_PROT_ENABLE;
        assert_eq!(
            ctrl.enable_region_protection(AspeedOtpRegion::Configuration),
            Ok(())
        );
    }
}
