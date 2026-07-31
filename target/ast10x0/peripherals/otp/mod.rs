// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::otp::common::{
    AspeedChipVersion, AspeedOtpRegion, Logger, OtpError, SessionInfo, StrapStatus,
};
use ast1060_pac::{Scu, Secure};
use core::fmt::Write;
use embedded_hal::delay::DelayNs;

type SbRegBlock = ast1060_pac::secure::RegisterBlock;

pub mod common;

struct DummyDelay;

impl DelayNs for DummyDelay {
    fn delay_ns(&mut self, ns: u32) {
        for _ in 0..(ns / 100) {
            cortex_m::asm::nop();
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
    pub sb: &'static SbRegBlock,
    pub scu: Scu,
    pub locked: bool,
    pub session_active: bool,
    pub logger: L,
}

macro_rules! otp_debug {
    ($logger:expr, $($arg:tt)*) => {
        let mut buf: heapless::String<64> = heapless::String::new();
        write!(buf, $($arg)*).unwrap();
        $logger.debug(buf.as_str());
    };
}

macro_rules! otp_error {
    ($logger:expr, $($arg:tt)*) => {
        let mut buf: heapless::String<64> = heapless::String::new();
        write!(buf, $($arg)*).unwrap();
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
pub const OTP_MEM_LOCK_ENBLE: u32 = 1 << 31;
pub const OTP_KEY_PROT_ENBLE: u32 = 1 << 29;
pub const OTP_STRAP_PROT_ENBLE: u32 = 1 << 25;
pub const OTP_CONF_PROT_ENBLE: u32 = 1 << 24;
//data secure,user,ecc
pub const OTP_USER_ECC_PROT_ENBLE: u32 = 1 << 23;
const OTP_SECURE_PROT_ENBLE: u32 = 1 << 22;
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
    region_type: AspeedOtpRegion,
    start: usize,
    cdw_size: usize,
    alignment: usize, //data aligment
}
pub static REGION_IDS: &[AspeedOtpRegion] = &[
    AspeedOtpRegion::Data,
    AspeedOtpRegion::Configuration,
    AspeedOtpRegion::Strap,
    AspeedOtpRegion::ScuProtection,
];

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

impl<L: Logger> OtpController<L> {
    pub fn new(scu: Scu, logger: L) -> Self {
        let locked: bool = false;
        let session_active = false;
        let sb = unsafe { &*Secure::PTR };
        Self {
            sb,
            scu,
            locked,
            session_active,
            logger,
        }
    }
    pub fn chip_version(&self) -> AspeedChipVersion {
        let revid0 = self.scu.scu004().read().bits();
        let revid1 = self.scu.scu014().read().bits();

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
        while !self.sb.secure014().read().otpctrl_sts().bit()
            || !self.sb.secure014().read().otpmemory_sts().bit()
        {
            tries -= 1;
            if tries == 0 {
                break;
            }
        }
        if self.sb.secure014().read().otpctrl_sts().bit()
            && self.sb.secure014().read().otpmemory_sts().bit()
        {
            return true;
        }
        false
    }

    pub fn otp_write(&self, otp_addr: u32, data: u32) -> bool {
        self.sb.secure010().write(|w| unsafe { w.bits(otp_addr) });
        self.sb.secure020().write(|w| unsafe { w.bits(data) });
        self.sb
            .secure004()
            .write(|w| unsafe { w.bits(OTP_WRITE_CMD) });
        self.wait_complete()
    }

    pub fn otp_soak(&self, otp_soak: OtpSoak) -> bool {
        match otp_soak {
            OtpSoak::Default => {
                self.otp_write(SOAK_PROG_DEFAULT[0].address, SOAK_PROG_DEFAULT[0].data);
                self.otp_write(SOAK_PROG_DEFAULT[1].address, SOAK_PROG_DEFAULT[1].data);
                self.otp_write(SOAK_PROG_DEFAULT[2].address, SOAK_PROG_DEFAULT[2].data);
            }
            OtpSoak::NormalProg => {
                self.otp_write(SOAK_PROG_NORMAL[0].address, SOAK_PROG_NORMAL[0].data);
                self.otp_write(SOAK_PROG_NORMAL[1].address, SOAK_PROG_NORMAL[1].data);
                self.otp_write(SOAK_PROG_NORMAL[2].address, SOAK_PROG_NORMAL[2].data);
                self.sb
                    .secure008()
                    .write(|w| unsafe { w.bits(OTP_TIMING_200US) });
            }
            OtpSoak::SoakProg => {
                self.otp_write(SOAK_PROG_SOAK[0].address, SOAK_PROG_SOAK[0].data);
                self.otp_write(SOAK_PROG_SOAK[1].address, SOAK_PROG_SOAK[1].data);
                self.otp_write(SOAK_PROG_SOAK[2].address, SOAK_PROG_SOAK[2].data);
                self.sb
                    .secure008()
                    .write(|w| unsafe { w.bits(OTP_TIMING_600US) });
            }
        }
        self.wait_complete()
    }
    ///
    /// Read 2 DWORD data
    ///
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
    ///
    /// Read configruation
    ///
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
    ///
    /// Inverse the data
    ///
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
        if prog_bit > 0 {
            self.otp_prog(address, prog_bit)
        } else {
            Err(OtpError::Timeout)
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

    ///
    /// bit program
    ///
    pub fn otp_prog_dc_b(
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
                    pass = true;
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
        let mut result: Result<(), OtpError>;
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
            result = self.otp_prog(address, prog_bit);
            if result != Ok(()) {
                return result;
            }
        }
        Ok(())
    }

    ///
    /// verify 1 DWORD from odd/even address
    ///
    pub fn verify_dw(&self, address: u32, data: u32, ignore: u32, compare: &mut u32) -> bool {
        let mut ret: [u32; 2] = [0, 0];

        let otp_addr = address & !(1 << 15);

        let addr = if otp_addr & 0x1 == 0 {
            otp_addr
        } else {
            otp_addr - 1
        };
        if self.otp_read_data(addr, &mut ret) != Ok(()) {
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

    ///
    /// verify 2 DWORD
    ///
    pub fn verify_2dw(
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
            if self.otp_read_data(otp_addr, &mut ret) != Ok(()) {
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

    ///
    /// Check if prorammed data is valid
    ///
    pub fn is_program_data_valid(&mut self, addr: u32, otp_data: u32, buffer_data: u32) -> bool {
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
    ///
    /// Program 2 DWORD and verify in data region
    ///
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
            && !self.is_program_data_valid(address, data0_masked, buf0_masked)
        {
            return Err(OtpError::WriteFailed);
        }
        if ignore_mask[1] != 0xffff_ffff
            && !self.is_program_data_valid(address + 1, data1_masked, buf1_masked)
        {
            return Err(OtpError::WriteFailed);
        }
        if !self.otp_soak(OtpSoak::NormalProg) {
            return Err(OtpError::Timeout);
        }

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

    ///
    /// bit programing retry
    ///
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
    ///
    /// Program OTP data region
    /// starts from "address" with "buffer" contents
    ///
    #[allow(clippy::needless_range_loop)]
    pub fn aspeed_otp_prog_data(
        &mut self,
        address: usize,
        buffer: &mut [u32],
    ) -> Result<(), OtpError> {
        let mut result = Ok(());
        let ignore: u32 = 0; //ignore bits mask
        let mut addr: u32;
        let len: usize = buffer.len();
        let mut pass: bool;

        if address + len > OTP_MEM_LIMIT_DATA || address & 0x3 != 0 {
            return Err(OtpError::InvalidAddress);
        }
        self.otp_unlock_reg();

        for i in 0..len {
            addr = u32::try_from(address + i).unwrap();
            self.otp_soak(OtpSoak::NormalProg);
            result = self.otp_prog_dw(buffer[i], ignore, addr);
            if result != Ok(()) {
                return result;
            }
            pass = self.otp_prog_verify_retry(buffer[i], ignore, addr);
            if !pass {
                self.otp_soak(OtpSoak::Default);
                result = Err(OtpError::WriteFailed);
                break;
            }
        }
        self.otp_lock_reg();
        result
    }
    //lock otp memory
    pub fn otp_lock_mem(&mut self) -> Result<(), OtpError> {
        if self.is_otp_locked() {
            self.locked = true;
            return Ok(());
        }
        self.otp_prog(0, 31)?;
        if !self.is_otp_locked() {
            return Err(OtpError::LockFailed);
        }
        self.locked = true;
        Ok(())
    }
    pub fn is_otp_locked(&self) -> bool {
        let otp_conf: u32 = self.otp_read_conf_idx(0).unwrap_or_default();
        otp_conf & OTP_MEM_LOCK_ENBLE == OTP_MEM_LOCK_ENBLE
    }
    pub fn is_key_protected(&self) -> bool {
        let otp_conf: u32 = self.otp_read_conf_idx(0).unwrap_or_default();
        otp_conf & OTP_KEY_PROT_ENBLE == OTP_KEY_PROT_ENBLE
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
            otp_conf & OTP_MEM_LOCK_ENBLE == OTP_MEM_LOCK_ENBLE;
        session.protection_status.strap_protected =
            otp_conf & OTP_STRAP_PROT_ENBLE == OTP_STRAP_PROT_ENBLE;
        session.protection_status.user_ecc_protected =
            otp_conf & OTP_USER_ECC_PROT_ENBLE == OTP_USER_ECC_PROT_ENBLE;

        session.protection_status.security_protected =
            otp_conf & OTP_SECURE_PROT_ENBLE == OTP_SECURE_PROT_ENBLE;
        let mut secure_size = otp_conf >> OTP_SECURE_SIZE_BIT_POS;
        if secure_size != 0 {
            secure_size = (secure_size & (OTP_SECURE_SIZE_MASK + 1)) << 5;
        }
        session.protection_status.security_size = secure_size;
    }
    pub fn get_sw_revision(&self, sw_rid: &mut [u32; 2]) {
        sw_rid[0] = self.sb.secure068().read().bits();
        sw_rid[1] = self.sb.secure06c().read().bits();
    }
    pub fn get_tool_verion(&self) -> &[u8] {
        return OTP_VER.as_bytes();
    }
    pub fn get_key_count(&self) -> u8 {
        let key_num = self.sb.secure078().read().sec_boot_key_number_regs().bits();

        key_num
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

    ///
    /// Read from data region
    ///
    pub fn aspeed_otp_read_data(&self, offset: usize, buffer: &mut [u32]) -> Result<(), OtpError> {
        let mut temp: [u32; 2] = [0, 0];
        let cdw_len: usize = buffer.len();
        if cdw_len + offset > OTP_MEM_LIMIT_DATA {
            return Err(OtpError::BoundaryError);
        }
        if offset & 0x4 != 0 {
            return Err(OtpError::AlignmentError);
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

    ///
    /// Read from configuration region
    ///
    pub fn aspeed_otp_read_conf(&self, offset: u32, buffer: &mut [u32]) -> Result<(), OtpError> {
        let mut result: Result<(), OtpError> = Ok(());
        let cdw_len = u32::try_from(buffer.len()).unwrap();
        if cdw_len + offset > 32 {
            return Err(OtpError::BoundaryError);
        }
        self.otp_unlock_reg();
        self.otp_soak(OtpSoak::Default);
        for i in offset..offset + cdw_len {
            let idx = (i - offset) as usize;
            buffer[idx] = match self.otp_read_conf_idx(i) {
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
    ///
    ///Read OTP strap into buffer.
    ///buf: output OTP strap into buffer.
    ///
    #[allow(dead_code)]
    fn aspeed_otp_read_strap(&self, offset: usize, buf: &mut [u32]) -> Result<(), OtpError> {
        let mut strap_status: [StrapStatus; 64] = [StrapStatus {
            value: false,
            protected: false,
            options: [0; 7],
            remaining_writes: 6,
            writable_option: 0xff,
        }; 64];
        let cdw_len = buf.len();

        if cdw_len + offset > 2 {
            return Err(OtpError::BoundaryError);
        }
        self.otp_unlock_reg();
        let result = self.otp_strap_status(&mut strap_status);
        if result == Ok(()) {
            for i in offset..offset + cdw_len {
                let idx = i - offset;
                buf[idx] = 0;
                for j in 0..32 {
                    buf[idx] |= u32::from(strap_status[i * 32 + j].value) << j;
                }
            }
        }
        self.otp_lock_reg();
        result
    }

    ///
    ///Read OTP protect into buffer.
    ///offset-offset to scu protect region
    ///OTPCFG28,OTPCFG29
    ///
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

        if result == Ok(()) {
            for i in offset..offset + cdw_len {
                let idx = i - offset;
                buffer[idx] = match self.otp_read_conf_idx(28 + u32::try_from(offset).unwrap()) {
                    Ok(value) => value,
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                };
            }
        }
        self.otp_lock_reg();
        result
    }

    ///
    /// Write data to data region
    ///
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
            let idx1 = i;
            result = self.otp_read_data(u32::try_from(idx1).unwrap(), &mut scratch);
            if result != Ok(()) {
                otp_debug!(
                    self.logger,
                    "otp_prog_data: read fail {:?}",
                    result.unwrap_err()
                );
                break;
            }
            otp_debug!(self.logger, "otp_prog_data: idx0={:}, idx1={:}", idx0, idx1);
            result = self.otp_prog_verify_2dw(
                u32::try_from(i).unwrap(),
                &scratch,
                &data[idx0..idx0 + 2],
                &ignore,
            );
            if result != Ok(()) {
                break;
            }
        }

        self.otp_soak(OtpSoak::Default);
        self.otp_lock_reg();
        result
    }

    ///
    /// Program strap bits
    /// All non proteced bits will be programmed
    ///
    #[allow(clippy::needless_range_loop)]
    pub fn otp_prog_strap(&mut self, start_bit: usize, strap: &[u32]) -> Result<(), OtpError> {
        let mut prog_address: u32;
        let mut bit: u32;
        let mut offset: u32;
        let mut prog_flag: u32;
        let mut count_prot: u32 = 0;
        let mut count_cant_write: u32 = 0;

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
        //all strap bits
        for i in start_bit..64 {
            prog_address = OTP_CONF_OFFSET;
            if i < 32 {
                offset = u32::try_from(i).unwrap();
                bit = (strap[0] >> (offset - u32::try_from(start_bit).unwrap())) & 0x1;
                prog_address |= ((u32::from(os[i].writable_option) * 2 + 16) / 8) * 0x200;
                prog_address |= ((u32::from(os[i].writable_option) * 2 + 16) % 8) * 0x2;
            } else {
                offset = u32::try_from(i - 32).unwrap();
                if i - start_bit < 32 {
                    bit = (strap[0] >> offset) & 0x1;
                } else {
                    bit = (strap[1] >> (offset - u32::try_from(start_bit).unwrap())) & 0x1;
                }
                prog_address |= ((u32::from(os[i].writable_option) * 2 + 17) / 8) * 0x200;
                prog_address |= ((u32::from(os[i].writable_option) * 2 + 17) % 8) * 0x2;
            }
            //check if program bit value is the same as the programmed bit value
            if bit == u32::from(os[i].value) {
                prog_flag = 0; //no need to proram
                otp_debug!(self.logger, "otp_prog_strap: bit {:} no need to program", i);
            } else {
                prog_flag = 1;
                otp_debug!(
                    self.logger,
                    "otp_prog_strap: program bit {:} from {:} to {:}",
                    i,
                    u32::from(os[i].value),
                    bit
                );
            }
            //bit to be prgrammed is protected
            if os[i].protected && prog_flag == 1 {
                count_prot += 1;
                continue;
            }
            if os[i].remaining_writes == 0 && prog_flag == 1 {
                count_cant_write += 1;
                continue;
            }
            if prog_flag == 1 {
                match self.otp_prog_dc_b(1, prog_address, offset) {
                    Ok(()) => {}
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
        self.otp_soak(OtpSoak::Default);
        if count_prot > 0 || count_cant_write > 0 {
            //return Err()
        }
        Ok(())
    }

    ///
    /// OTP conf
    ///
    pub fn otp_prog_conf(&mut self, start_conf: usize, conf: &[u32]) -> Result<(), OtpError> {
        let mut result: Result<(), OtpError> = Ok(());
        let conf_ignore: u32 = 0;
        let mut otp_conf: u32;
        let mut pass: bool = false;
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
            if result != Ok(()) {
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

    ///
    /// SCU protect
    ///
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
            data_masked = scu_pro[i] & !ignore;
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
                if otp_conf & OTP_USER_ECC_PROT_ENBLE == OTP_USER_ECC_PROT_ENBLE
                    && otp_conf & OTP_SECURE_PROT_ENBLE == OTP_SECURE_PROT_ENBLE
                {
                    protected = true;
                }
            }
            AspeedOtpRegion::Configuration => {
                if otp_conf & OTP_CONF_PROT_ENBLE == OTP_CONF_PROT_ENBLE {
                    protected = true;
                }
            }
            AspeedOtpRegion::Strap => {
                if otp_conf & OTP_STRAP_PROT_ENBLE == OTP_STRAP_PROT_ENBLE {
                    protected = true;
                }
            }
            AspeedOtpRegion::ScuProtection => {}
        }
        Ok(protected)
    }

    /// Enable protection for a specific region
    ///
    /// # Parameters
    /// - `region`: The region to protect
    ///
    /// # Returns
    /// - `Ok(())`: Protection enabled successfully
    /// - `Err(Self::Error)`: Failed to enable protection
    pub fn enable_region_protection(&mut self, region: AspeedOtpRegion) -> Result<(), OtpError> {
        let mut value: [u32; 1] = [0; 1];

        if self.is_region_protected(region) == Ok(true) {
            return Ok(());
        }
        match region {
            AspeedOtpRegion::Data => {
                value[0] = OTP_USER_ECC_PROT_ENBLE | OTP_SECURE_PROT_ENBLE;
            }
            AspeedOtpRegion::Configuration => {
                value[0] = OTP_CONF_PROT_ENBLE;
            }
            AspeedOtpRegion::Strap => {
                value[0] = OTP_STRAP_PROT_ENBLE;
            }
            AspeedOtpRegion::ScuProtection => {}
        }
        self.otp_prog_conf(0, &value)
    }
    #[allow(clippy::unused_self)]
    #[allow(clippy::unnecessary_wraps)]
    pub fn is_feature_supported(&self, _feature: &str) -> Result<bool, OtpError> {
        Ok(false)
    }
    #[allow(clippy::unused_self)]
    #[allow(clippy::unnecessary_wraps)]
    pub fn list_regions(&self) -> Result<&[AspeedOtpRegion], OtpError> {
        Ok(REGION_IDS)
    }
    #[allow(clippy::unused_self)]
    pub fn get_region_info(&self, region: AspeedOtpRegion) -> Result<(usize, usize, usize), OtpError> {
        for each in REGION_INFO {
            if each.region_type == region {
                return Ok((each.start, each.cdw_size, each.alignment));
            }
        }
        Err(OtpError::Unknown)
    }
}
