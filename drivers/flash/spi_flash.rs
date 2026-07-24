// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use bitfield_struct::bitfield;
use core::cmp::min;
use core::convert::TryFrom;
use core::num::NonZero;
use core::prelude::v1::*;
use util_sfdp::{QuadEnableRequirements, SfdpReader};
use util_types::PowerOf2Usize;
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes};

use embedded_hal::spi::Operation;
use hal_flash::{Flash as FlashTrait, FlashAddress};
use util_error::{self as error, ErrorCode};
use util_io::RandomRead;

// TODO(b/481400917): Replace with stronger "byte count" type.
const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;

// The maximum number of bytes the opcode + address + dummy bytes at the start
// of a transaction will need.
const MAX_PREFIX_LEN: usize = 6;
const MAX_3B_SIZE: usize = 16 * MIB;
const SECTOR_SIZE: usize = 4096;
const BLOCK_SIZE: usize = 64 * KIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiTxnWidth {
    STANDARD = 0,
    DUAL = 1,
    QUAD = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SfCmd {
    pub opcode: u8,
    pub width: SpiTxnWidth,
    pub addr_mode: AddressingMode,
}

impl SfCmd {
    pub const READ: Self = Self {
        opcode: OP_READ,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_3Byte,
    };
    pub const PROGRAM: Self = Self {
        opcode: OP_PROGRAM,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_3Byte,
    };
    pub const ERASE: Self = Self {
        opcode: OP_ERASE_4K,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_3Byte,
    };
    pub const READ4B: Self = Self {
        opcode: OP_READ4B,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const PROGRAM4B: Self = Self {
        opcode: OP_PROGRAM4B,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const ERASE4B: Self = Self {
        opcode: OP_ERASE4B_4K,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const ERASE64K: Self = Self {
        opcode: OP_ERASE_64K,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_3Byte,
    };
    pub const ERASE4B_64K: Self = Self {
        opcode: OP_ERASE4B_64K,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const QREAD: Self = Self {
        opcode: OP_QREAD,
        width: SpiTxnWidth::QUAD,
        addr_mode: AddressingMode::_3ByteWithDummy,
    };
    pub const QREAD4B: Self = Self {
        opcode: OP_QREAD4B,
        width: SpiTxnWidth::QUAD,
        addr_mode: AddressingMode::_4ByteWithDummy,
    };
    pub const QPROGRAM: Self = Self {
        opcode: OP_QPROGRAM,
        width: SpiTxnWidth::QUAD,
        addr_mode: AddressingMode::_3Byte,
    };
    pub const QPROGRAM4B: Self = Self {
        opcode: OP_QPROGRAM4B,
        width: SpiTxnWidth::QUAD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const READ4B_GLOBAL: Self = Self {
        opcode: OP_READ,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const PROGRAM4B_GLOBAL: Self = Self {
        opcode: OP_PROGRAM,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const ERASE4B_GLOBAL: Self = Self {
        opcode: OP_ERASE_4K,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
    pub const ERASE64K_GLOBAL: Self = Self {
        opcode: OP_ERASE_64K,
        width: SpiTxnWidth::STANDARD,
        addr_mode: AddressingMode::_4Byte,
    };
}

pub struct SpiFlashConfig {
    pub read: SfCmd,
    pub program: SfCmd,
    pub erase4k: SfCmd,
    pub erase64k: SfCmd,

    /// The total size of the flash in bytes
    pub size: NonZero<usize>,
    pub quad_enable_req: Option<QuadEnableRequirements>,
}

impl SpiFlashConfig {
    pub fn from_sfdp_conservative<R: RandomRead<Error = ErrorCode>>(
        sfdp_bytes: R,
    ) -> Result<Self, ErrorCode> {
        let mut sfdp = SfdpReader::new(sfdp_bytes)?;
        let table = sfdp.basic_flash_parameters()?;

        let size = usize::try_from(table.table_jesd216().memory_density.byte_len()?).unwrap();
        let size = NonZero::new(size).ok_or(error::FLASH_GENERIC_INVALID_SIZE)?;
        let config = if size.get() <= MAX_3B_SIZE {
            SpiFlashConfig {
                size,
                read: SfCmd::READ,
                erase4k: SfCmd::ERASE,
                erase64k: SfCmd::ERASE64K,
                program: SfCmd::PROGRAM,
                quad_enable_req: None,
            }
        } else {
            SpiFlashConfig {
                size,
                read: SfCmd::READ4B_GLOBAL,
                erase4k: SfCmd::ERASE4B_GLOBAL,
                erase64k: SfCmd::ERASE64K_GLOBAL,
                program: SfCmd::PROGRAM4B_GLOBAL,
                quad_enable_req: None,
            }
        };
        Ok(config)
    }

    pub fn from_sfdp<R: RandomRead<Error = ErrorCode>>(sfdp_bytes: R) -> Result<Self, ErrorCode> {
        let mut sfdp = SfdpReader::new(sfdp_bytes)?;
        let table = sfdp.basic_flash_parameters()?;

        let size = usize::try_from(table.table_jesd216().memory_density.byte_len()?).unwrap();
        let size = NonZero::new(size).ok_or(error::FLASH_GENERIC_INVALID_SIZE)?;

        // TODO: Restore Quad SPI (QSPI) support once earlgrey_spi_host implements
        // DUAL/QUAD transaction widths.
        let config = if size.get() <= MAX_3B_SIZE {
            SpiFlashConfig {
                size,
                read: SfCmd::READ,
                erase4k: SfCmd::ERASE,
                erase64k: SfCmd::ERASE64K,
                program: SfCmd::PROGRAM,
                quad_enable_req: None,
            }
        } else {
            SpiFlashConfig {
                size,
                read: SfCmd::READ4B_GLOBAL,
                erase4k: SfCmd::ERASE4B_GLOBAL,
                erase64k: SfCmd::ERASE64K_GLOBAL,
                program: SfCmd::PROGRAM4B_GLOBAL,
                quad_enable_req: None,
            }
        };
        Ok(config)
    }
}

/// "Driver" for SPI NOR flash.
pub struct SpiFlash<S: embedded_hal::spi::SpiDevice> {
    spi: S,
    config: SpiFlashConfig,
    initialized: bool,
}

impl<S: embedded_hal::spi::SpiDevice> SpiFlash<S> {
    pub fn new(spi: S) -> Self {
        Self {
            spi,
            config: SpiFlashConfig {
                read: SfCmd::READ,
                program: SfCmd::PROGRAM,
                erase4k: SfCmd::ERASE,
                erase64k: SfCmd::ERASE64K,
                size: NonZero::new(1).unwrap(),
                quad_enable_req: None,
            },
            initialized: false,
        }
    }

    pub fn init(&mut self) -> Result<(), ErrorCode> {
        let sfdp_bytes = SfdpRandRead { spi: &mut self.spi };
        let config = SpiFlashConfig::from_sfdp(sfdp_bytes)?;
        let qer = config.quad_enable_req;

        let mut status = Status::new_zeroed();
        self.transfer_req_resp(&[OP_STATUS], status.as_mut_bytes())?;

        if status.busy() {
            self.wait_for_busy_to_clear()?;
        }

        if let Some(qer) = qer {
            match qer {
                QuadEnableRequirements::NoQeBit => {}
                QuadEnableRequirements::QeBit6SR1 => {
                    if !status.maybe_quad_en() {
                        status.set_maybe_quad_en(true);
                        self.transfer_req_resp(&[OP_WRITE_EN], &mut [])?;
                        self.transfer_req_resp(&[OP_WR_STATUS, status.into()], &mut [])?;
                    }
                }
                _ => {
                    self.config.read = SfCmd::READ4B;
                }
            }
        }

        // TODO: Consider different options:
        // Enter 4-byte mode and use READ4B_GLOBAL vs Use READ4B if available.
        if config.size.get() > MAX_3B_SIZE {
            self.enter_4byte_mode()?;
        }

        self.config = config;
        self.initialized = true;
        Ok(())
    }

    fn enter_4byte_mode(&mut self) -> Result<(), ErrorCode> {
        self.transfer_req_resp(&[OP_WRITE_EN], &mut [])?;
        self.spi
            .write(&[OP_ENTER_4B_ADDR_MODE])
            .map_err(|_| error::FLASH_GENERIC_BUSY)?;
        Ok(())
    }

    pub fn reset_device(&mut self) -> Result<(), ErrorCode> {
        self.transfer_req_resp(&[OP_RESET_ENABLE], &mut [])?;
        self.transfer_req_resp(&[OP_RESET], &mut [])?;
        Ok(())
    }

    pub fn read_jedec_id(&mut self, buf: &mut [u8]) -> Result<(), ErrorCode> {
        self.transfer_req_resp(&[OP_READ_JEDEC_ID], buf)
    }

    pub fn config(&self) -> &SpiFlashConfig {
        &self.config
    }

    pub fn set_ear(&mut self, bank: u8) -> Result<(), ErrorCode> {
        self.transfer_req_resp(&[OP_WRITE_EN], &mut [])?;
        self.transfer_req_resp(&[OP_WR_EAR, bank], &mut [])?;
        Ok(())
    }

    /// Erase the entire chip.
    pub fn erase_all(&mut self) -> Result<(), ErrorCode> {
        self.transfer_req_resp(&[OP_WRITE_EN], &mut [])?;
        self.transfer_req_resp(&[OP_CHIP_ERASE], &mut [])?;
        self.wait_for_busy_to_clear()
    }

    fn wait_for_busy_to_clear(&mut self) -> Result<(), ErrorCode> {
        let mut status = Status::new_zeroed();
        loop {
            self.transfer_req_resp(&[OP_STATUS], status.as_mut_bytes())?;
            if !status.busy() {
                return Ok(());
            }
        }
    }

    fn erase_cmd(&mut self, start_addr: usize, cmd: SfCmd) -> Result<(), ErrorCode> {
        self.transfer_req_resp(&[OP_WRITE_EN], &mut [])?;

        let mut buf = [0_u8; MAX_PREFIX_LEN];
        let op = cmd
            .addr_mode
            .write_prefix(&mut buf, cmd.opcode, start_addr)?;
        self.transfer_req_resp(op, &mut [])?;

        self.wait_for_busy_to_clear()
    }

    fn transfer_req_resp(&mut self, req: &[u8], resp: &mut [u8]) -> Result<(), ErrorCode> {
        if resp.is_empty() {
            self.spi.write(req).map_err(|_| error::FLASH_GENERIC_BUSY)
        } else {
            let mut ops = [Operation::Write(req), Operation::Read(resp)];
            self.spi
                .transaction(&mut ops)
                .map_err(|_| error::FLASH_GENERIC_BUSY)
        }
    }

    fn check_valid_size(&self, start_addr: usize, len: usize) -> Result<(), ErrorCode> {
        let Some(end_addr) = start_addr.checked_add(len) else {
            return Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
        };
        if end_addr > self.config.size.get() {
            return Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
        }
        Ok(())
    }

    pub fn is_busy(&mut self) -> bool {
        let mut status = Status::new_zeroed();
        match self.transfer_req_resp(&[OP_STATUS], status.as_mut_bytes()) {
            Ok(_) => status.busy(),
            Err(_) => true,
        }
    }

    pub fn complete_op(&mut self) -> Result<(), ErrorCode> {
        Ok(())
    }
}

impl<S: embedded_hal::spi::SpiDevice> FlashTrait for SpiFlash<S> {
    type Error = ErrorCode;

    fn geometry(&mut self) -> Result<(NonZero<usize>, PowerOf2Usize, u32), ErrorCode> {
        if !self.initialized {
            return Err(error::FLASH_GENERIC_NOT_INITIALIZED);
        }
        // Support 4KB and 64KB erases
        let bitmap = (1 << SECTOR_SIZE.trailing_zeros()) | (1 << BLOCK_SIZE.trailing_zeros());
        let page_size = PowerOf2Usize::new(SECTOR_SIZE).unwrap();
        Ok((self.config.size, page_size, bitmap))
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        if !self.initialized {
            return Err(error::FLASH_GENERIC_NOT_INITIALIZED);
        }
        read_common(
            &mut self.spi,
            start_addr.offset() as usize,
            buf,
            self.config.read.opcode,
            self.config.read.width,
            self.config.read.addr_mode,
            self.config.size.get(),
        )
    }

    fn program(&mut self, start_address: FlashAddress, mut data: &[u8]) -> Result<(), ErrorCode> {
        if !self.initialized {
            return Err(error::FLASH_GENERIC_NOT_INITIALIZED);
        }
        // A single program transaction must not span 256-byte pages; writes
        // that span multiple pages must be split into multiple chunks.
        const PROGRAM_PAGE_LEN: usize = 256;

        let start_addr = start_address.offset() as usize;
        self.check_valid_size(start_addr, data.len())?;

        // TODO: Eliminate this buffer once the drivers support vectored I/O.
        let mut buf = [0_u8; MAX_PREFIX_LEN + PROGRAM_PAGE_LEN];

        let mut addr = start_addr;
        while !data.is_empty() {
            self.transfer_req_resp(&[OP_WRITE_EN], &mut [])?;
            let prefix_len = self
                .config
                .program
                .addr_mode
                .write_prefix(
                    <&mut [u8; MAX_PREFIX_LEN]>::try_from(&mut buf[..MAX_PREFIX_LEN]).unwrap(),
                    self.config.program.opcode,
                    addr,
                )?
                .len();

            let chunk_len = min(data.len(), PROGRAM_PAGE_LEN - (addr % PROGRAM_PAGE_LEN));
            buf[prefix_len..][..chunk_len].copy_from_slice(&data[..chunk_len]);
            self.transfer_req_resp(&buf[..prefix_len + chunk_len], &mut [])?;

            self.wait_for_busy_to_clear()?;
            data = &data[chunk_len..];
            addr += chunk_len;
        }
        Ok(())
    }

    fn erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
        if !self.initialized {
            return Err(error::FLASH_GENERIC_NOT_INITIALIZED);
        }
        let mut addr = start_addr.offset() as usize;
        let mut len = size.get();

        if addr % SECTOR_SIZE != 0 {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR);
        }
        if len % SECTOR_SIZE != 0 {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        }
        self.check_valid_size(addr, len)?;

        while len > 0 {
            let can_erase_block = (addr % BLOCK_SIZE == 0) && (len >= BLOCK_SIZE);
            let (cmd, erased) = if can_erase_block {
                (self.config.erase64k, BLOCK_SIZE)
            } else {
                (self.config.erase4k, SECTOR_SIZE)
            };
            self.erase_cmd(addr, cmd)?;
            addr += erased;
            len -= erased;
        }
        Ok(())
    }
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable)]
pub struct Status {
    busy: bool,
    write_en: bool,
    bp0: bool,
    bp1: bool,
    bp2: bool,
    bp3: bool,
    maybe_quad_en: bool,
    reserved7: bool,
}

const OP_STATUS: u8 = 0x05;
const OP_WRITE_EN: u8 = 0x06;
const OP_WR_STATUS: u8 = 0x01;
const OP_WR_EAR: u8 = 0xC5;
const OP_READ: u8 = 0x03;
const OP_QREAD: u8 = 0x6B;
const OP_READ4B: u8 = 0x13;
const OP_QREAD4B: u8 = 0x6C;
const OP_CHIP_ERASE: u8 = 0xC7;
const OP_ERASE_4K: u8 = 0x20;
const OP_ERASE4B_4K: u8 = 0x21;
const OP_ERASE_64K: u8 = 0xD8;
const OP_ERASE4B_64K: u8 = 0xDC;
const OP_PROGRAM: u8 = 0x02;
const OP_QPROGRAM: u8 = 0x32;
const OP_PROGRAM4B: u8 = 0x12;
const OP_QPROGRAM4B: u8 = 0x34;
const OP_SFDP_READ: u8 = 0x5a;
const OP_RESET_ENABLE: u8 = 0x66;
const OP_RESET: u8 = 0x99;
const OP_READ_JEDEC_ID: u8 = 0x9f;
const OP_ENTER_4B_ADDR_MODE: u8 = 0xB7;

/// A RandomRead implementation that can be used to access SFDP bytes.
pub struct SfdpRandRead<'a, S: embedded_hal::spi::SpiDevice> {
    spi: &'a mut S,
}

impl<'a, S: embedded_hal::spi::SpiDevice> SfdpRandRead<'a, S> {
    pub fn new(spi: &'a mut S) -> Self {
        Self { spi }
    }
}

const SFDP_MEM_SIZE: usize = 1 << 24;

impl<S: embedded_hal::spi::SpiDevice> RandomRead for SfdpRandRead<'_, S> {
    type Error = ErrorCode;
    fn read(&mut self, start_addr: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        read_common(
            self.spi,
            start_addr,
            buf,
            OP_SFDP_READ,
            SpiTxnWidth::STANDARD,
            AddressingMode::_3ByteWithDummy,
            SFDP_MEM_SIZE,
        )
    }
    fn size(&mut self) -> Result<usize, Self::Error> {
        Ok(SFDP_MEM_SIZE)
    }
}

fn read_common<S: embedded_hal::spi::SpiDevice>(
    spi: &mut S,
    start_addr: usize,
    buf: &mut [u8],
    opcode: u8,
    _width: SpiTxnWidth,
    addr_size: AddressingMode,
    src_total_len: usize,
) -> Result<(), ErrorCode> {
    let Some(end_addr) = start_addr.checked_add(buf.len()) else {
        return Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
    };
    if end_addr > src_total_len {
        return Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
    }
    let mut cmd_buf = [0; MAX_PREFIX_LEN];
    let cmd_buf = addr_size.write_prefix(&mut cmd_buf, opcode, start_addr)?;

    let mut ops = [Operation::Write(cmd_buf), Operation::Read(buf)];
    spi.transaction(&mut ops)
        .map_err(|_| error::FLASH_GENERIC_BUSY)
}

const _: () = assert!(
    size_of::<usize>() >= size_of::<u32>(),
    "on supported platforms, usize must be at least 32-bits"
);

/// Describes how addresses should be formatting on the wire
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    _3Byte = 0,
    _4Byte = 1,
    _3ByteWithDummy = 2,
    _4ByteWithDummy = 3,
}

impl AddressingMode {
    /// Writes the opcode, address and (if needed) dummy byte to `buf`, and
    /// returns a slice to the part of `buf` that should be sent as the initial
    /// bytes of the transaction.
    #[inline]
    fn write_prefix(
        self,
        buf: &mut [u8; MAX_PREFIX_LEN],
        opcode: u8,
        addr: usize,
    ) -> Result<&[u8], ErrorCode> {
        buf[0] = opcode;
        if !self.is_valid_addr(addr) {
            return Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
        }
        let shift = if matches!(self, Self::_3Byte | Self::_3ByteWithDummy) {
            8
        } else {
            0
        };
        let addr_bytes = u32::try_from(addr << shift).unwrap().to_be_bytes();
        *<&mut [u8; 4]>::try_from(&mut buf[1..5]).unwrap() = addr_bytes;
        let len = match self {
            Self::_3Byte => 4,
            Self::_4Byte => 5,
            Self::_3ByteWithDummy => 5,
            Self::_4ByteWithDummy => {
                buf[5] = 0;
                6
            }
        };
        Ok(&buf[..len])
    }

    /// Returns true if `addr` can be represented fully by this address mode.
    #[inline]
    fn is_valid_addr(self, addr: usize) -> bool {
        if addr < MAX_3B_SIZE {
            return true;
        }
        match self {
            Self::_3Byte | Self::_3ByteWithDummy => addr < MAX_3B_SIZE,
            Self::_4Byte | Self::_4ByteWithDummy => u32::try_from(addr).is_ok(),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::*;
    use drivers_mock_spi_device_fake::*;
    use std::vec;
    use std::vec::Vec;
    use util_sfdp::*;

    const GIB: usize = 1024 * MIB;

    const STATUS_WIP_WEL: u8 = 0x03;
    const STATUS_WIP: u8 = 0x01;
    const STATUS_READY: u8 = 0x00;
    // Note: Some parts use a different bit position.
    // See "6.4.18 JEDEC Basic Flash Parameter Table: 15th DWORD" of JESD216 and
    // `QuadEnableRequirements`.
    // FIXME: STATUS_QE is currently unused because QSPI dynamic initialization is disabled.
    #[allow(dead_code)]
    const STATUS_QE: u8 = 0x40;

    fn preprogram_init(fake_spi: &FakeSpiDevice, size_bytes: usize) {
        let sfdp = gen_sfdp(size_bytes);
        preprogram_sfdp(fake_spi, &sfdp);
        fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        if size_bytes > 16 * MIB {
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            fake_spi.preprogram_data_response(vec![OP_ENTER_4B_ADDR_MODE].into(), vec![].into());
        }
    }

    /// Generates some generic SFDP for a flash of a specific size (in bytes).
    fn gen_sfdp(flash_total_len: usize) -> Vec<u8> {
        const SFDP_HEADER: SfdpHeader = SfdpHeader {
            sig: SfdpSignature::EXPECTED_VALUE,
            major_rev: 1,
            minor_rev: 0,
            access_protocol: AccessProtocol::LEGACY,
            num_parameter_header: 0, // 0-based => 0 means 1 header
        };
        const SFDP_PARAMETER_HEADER: ParameterHeader = ParameterHeader {
            parameter_id_lsb: 0x00,
            major_rev: 1,
            minor_rev: 0,
            len_in_dwords: 23,
            ptr: U24::new(16),
            parameter_id_msb: 0xff,
        };
        let mut result = vec![];
        result.extend_from_slice(SFDP_HEADER.as_bytes());
        result.extend_from_slice(SFDP_PARAMETER_HEADER.as_bytes());
        let mut bpt = BasicFlashParameterTable::new_zeroed();
        bpt.table_jesd216.memory_density =
            MemoryDensity::from_byte_len(u32::try_from(flash_total_len).unwrap()).unwrap();
        //FIXME: Add test cases for the rest of the SFDP Quad support variations
        bpt.table_jesd216.word1.set_supports_1s_1s_4s_read(true);
        bpt.table_jesd216a
            .word15
            .set_quad_enable_requirements(QuadEnableRequirements::QeBit6SR1);
        result.extend_from_slice(bpt.as_bytes());
        result
    }

    fn preprogram_sfdp(fake_spi: &FakeSpiDevice, sfdp_bytes: &[u8]) {
        fake_spi.preprogram_data_response(
            vec![OP_SFDP_READ, 0, 0, 0, 0].into(),
            sfdp_bytes[..8].to_vec().into(),
        );
        fake_spi.preprogram_data_response(
            vec![OP_SFDP_READ, 0, 0, 8, 0].into(),
            sfdp_bytes[8..16].to_vec().into(),
        );
        fake_spi.preprogram_data_response(
            vec![OP_SFDP_READ, 0, 0, 16, 0].into(),
            sfdp_bytes[16..].to_vec().into(),
        );
    }

    #[test]
    fn test_size_8mb() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 8 * MIB);
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        assert_eq!(8 * MIB, flash.geometry().unwrap().0.get());
        assert_eq!(8 * MIB, flash.random_reader().size().unwrap());
        fake_spi.assert_all_expectations_met();
    }
    #[test]
    fn test_size_128mb() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 128 * MIB);
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        assert_eq!(128 * MIB, flash.geometry().unwrap().0.get());
        assert_eq!(128 * MIB, flash.random_reader().size().unwrap());
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_read_3b() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response(
            (&[OP_READ, 0x12, 0x34, 0x56]).into(),
            b"Hello World!".into(),
        );
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        let mut buf = [0x55_u8; 12];
        flash
            .read(FlashAddress::new(0x12_3456_u32), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello World!");
        assert_eq!(
            flash.read(FlashAddress::new(0x1234_5678_u32), &mut buf),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        assert_eq!(
            flash.read(FlashAddress::new((8 * MIB) as u32), &mut buf),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_read_4b() {
        let fake_spi = FakeSpiDevice::new();
        // 32 MiB flash
        preprogram_init(&fake_spi, 32 * MIB);
        fake_spi
            .preprogram_data_response((&[OP_READ, 0x01, 0x23, 0x45, 0x67]).into(), b"World".into());
        fake_spi
            .preprogram_data_response((&[OP_READ, 0x00, 0x12, 0x34, 0x56]).into(), b"Hello".into());
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        let mut buf = [0x55_u8; 5];
        flash
            .read(FlashAddress::new(0x0012_3456_u32), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello");
        flash
            .read(FlashAddress::new(0x0123_4567_u32), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"World");
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_read_4b_qspi() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 32 * MIB);

        // Expect OP_QREAD4B + 4B Address + 1 Dummy Byte (0x00)
        fake_spi.preprogram_data_response(
            vec![OP_QREAD4B, 0x00, 0x12, 0x34, 0x56, 0x00].into(),
            b"Hello".to_vec().into(),
        );

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.read = SfCmd::QREAD4B;

        let mut buf = [0u8; 5];
        flash
            .read(FlashAddress::new(0x0012_3456), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello");
        fake_spi.assert_all_expectations_met();
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_qread_3b() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response(
            (&[OP_QREAD, 0x12, 0x34, 0x56, 0x00]).into(),
            b"Hello World!".into(),
        );
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.read = SfCmd::QREAD;
        let mut buf = [0x55_u8; 12];
        flash
            .read(FlashAddress::new(0x12_3456_u32), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello World!");
        assert_eq!(
            flash.read(FlashAddress::new(0x1234_5678_u32), &mut buf),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        assert_eq!(
            flash.read(FlashAddress::new((8 * MIB) as u32), &mut buf),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_qread_4b() {
        let fake_spi = FakeSpiDevice::new();
        // 32 MiB flash
        preprogram_init(&fake_spi, 32 * MIB);
        fake_spi.preprogram_data_response(
            (&[OP_QREAD4B, 0x01, 0x23, 0x45, 0x67, 0x00]).into(),
            b"World".into(),
        );
        fake_spi.preprogram_data_response(
            (&[OP_QREAD4B, 0x00, 0x12, 0x34, 0x56, 0x00]).into(),
            b"Hello".into(),
        );
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.read = SfCmd::QREAD4B;
        let mut buf = [0x55_u8; 5];
        flash
            .read(FlashAddress::new(0x0012_3456_u32), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello");
        flash
            .read(FlashAddress::new(0x0123_4567_u32), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"World");
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_3b() {
        let fake_spi = FakeSpiDevice::new();
        // 16 MiB flash
        preprogram_init(&fake_spi, 16 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_ERASE_4K, 0xba, 0x10, 0x00]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP_WEL]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(0xba_1000_u32),
                PowerOf2Usize::new(4096).unwrap(),
            )
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 5..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // erase
                    tx: vec![OP_ERASE_4K, 0xba, 0x10, 0b00],
                    rx: vec![0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=1, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP_WEL],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new(0xba_1001_u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new((16 * MIB) as u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_4b() {
        let fake_spi = FakeSpiDevice::new();
        // 1 GiB flash
        preprogram_init(&fake_spi, GIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_ERASE_4K, 0x1a, 0x5e, 0xb0, 0x00]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP_WEL]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(0x1a5e_b000_u32),
                PowerOf2Usize::new(4096).unwrap(),
            )
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 4..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // erase (4-byte addr)
                    tx: vec![OP_ERASE_4K, 0x1a, 0x5e, 0xb0, 0x00],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=1, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP_WEL],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new(0xba_1001_u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new((GIB) as u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_last_page() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_ERASE_4K, 0x7f, 0xf0, 0x00]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((8 * MIB - 4096) as u32),
                PowerOf2Usize::new(4096).unwrap(),
            )
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // erase (4-byte addr)
                    tx: vec![OP_ERASE_4K, 0x7f, 0xf0, 0x00],
                    rx: vec![0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );

        assert_eq!(
            flash.erase(
                FlashAddress::new((8 * MIB - 1) as u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new((8 * MIB) as u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_single_page_3b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);

        const ADDRESS: usize = 0x1000;
        const LEN: usize = 4 * KIB;

        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_ERASE_4K, 0x00, 0x10, 0x00]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((ADDRESS) as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x00, 0x10, 0x00],
                    rx: vec![0, 0, 0, 0]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, STATUS_READY]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_single_page_4b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 32 * MIB);

        const ADDRESS: usize = 0x1000;
        const LEN: usize = 4 * KIB;

        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_ERASE_4K, 0x00, 0x00, 0x10, 0x00]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((ADDRESS) as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x00, 0x00, 0x10, 0x00],
                    rx: vec![0, 0, 0, 0, 0]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, STATUS_READY]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_single_block_64k_3b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);

        const ADDRESS: usize = 0x10000;
        const LEN: usize = 64 * KIB;

        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_ERASE_64K, 0x01, 0x00, 0x00]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((ADDRESS) as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_64K, 0x01, 0x00, 0x00],
                    rx: vec![0, 0, 0, 0]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, STATUS_READY]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_single_block_64k_4b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 32 * MIB);

        const ADDRESS: usize = 0x01230000;
        const LEN: usize = 64 * KIB;

        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_ERASE_64K, 0x01, 0x23, 0x00, 0x00]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((ADDRESS) as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_64K, 0x01, 0x23, 0x00, 0x00],
                    rx: vec![0, 0, 0, 0, 0]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, STATUS_READY]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_mixed_granularity_3b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);

        // Erase sequence: 15 pages, 1 block, 1 page.
        const ADDRESS: usize = 4 * KIB;
        const LEN: usize = 128 * KIB;

        let mut expectations = Vec::new();
        for i in 1..=15 {
            expectations.push((OP_ERASE_4K, 0x00, (i * 0x10) as u8, 0x00));
        }
        expectations.push((OP_ERASE_64K, 0x01, 0x00, 0x00));
        expectations.push((OP_ERASE_4K, 0x02, 0x00, 0x00));

        for (op, a1, a2, a3) in expectations.clone() {
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            fake_spi.preprogram_data_response(vec![op, a1, a2, a3].into(), vec![].into());
            fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        }

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(ADDRESS as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        let log = fake_spi.log();
        let log = &log[log.len() - 3 * expectations.len()..];
        for (i, (op, a1, a2, a3)) in expectations.iter().enumerate() {
            let offset = i * 3;
            assert_eq!(log[offset].tx, vec![OP_WRITE_EN]);
            assert_eq!(log[offset + 1].tx, vec![*op, *a1, *a2, *a3]);
            assert_eq!(log[offset + 2].tx, vec![OP_STATUS, 0x00]);
        }
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_mixed_granularity_4b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 32 * MIB);

        // Erase sequence: 1 page, 1 block, 15 pages.
        const ADDRESS: usize = MAX_3B_SIZE - 4 * KIB;
        const LEN: usize = 128 * KIB;

        let mut expectations = Vec::new();
        expectations.push((OP_ERASE_4K, 0x00, 0xff, 0xf0, 0x00));
        expectations.push((OP_ERASE_64K, 0x01, 0x00, 0x00, 0x00));
        for i in 0..15 {
            expectations.push((OP_ERASE_4K, 0x01, 0x01, (i * 0x10) as u8, 0x00));
        }

        for (op, a1, a2, a3, a4) in expectations.clone() {
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            fake_spi.preprogram_data_response(vec![op, a1, a2, a3, a4].into(), vec![].into());
            fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        }

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(ADDRESS as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        let log = fake_spi.log();
        let log = &log[log.len() - 3 * expectations.len()..];
        for (i, (op, a1, a2, a3, a4)) in expectations.iter().enumerate() {
            let offset = i * 3;
            assert_eq!(log[offset].tx, vec![OP_WRITE_EN]);
            assert_eq!(log[offset + 1].tx, vec![*op, *a1, *a2, *a3, *a4]);
            assert_eq!(log[offset + 2].tx, vec![OP_STATUS, 0x00]);
        }
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_alignment_errors() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();

        // Start address not page aligned
        assert_eq!(
            flash.erase(FlashAddress::new(1_u32), PowerOf2Usize::new(4096).unwrap()),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );

        // Length not page aligned
        assert_eq!(
            flash.erase(FlashAddress::new(0_u32), PowerOf2Usize::new(1).unwrap()),
            Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_out_of_bounds() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();

        assert_eq!(
            flash.erase(
                FlashAddress::new((16 * MIB - 4 * KIB) as u32),
                PowerOf2Usize::new(8 * KIB).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );

        // 4B flash (32 MiB)
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 32 * MIB);
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();

        assert_eq!(
            flash.erase(
                FlashAddress::new((32 * MIB) as u32),
                PowerOf2Usize::new(4 * KIB).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_entire_flash_3b() {
        let fake_spi = FakeSpiDevice::new();
        const SIZE: usize = 128 * KIB;
        preprogram_init(&fake_spi, SIZE);

        let expectations = [
            (OP_ERASE_64K, vec![0x00, 0x00, 0x00]),
            (OP_ERASE_64K, vec![0x01, 0x00, 0x00]),
        ];

        for (op, addr_bytes) in &expectations {
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            let mut tx = vec![*op];
            tx.extend_from_slice(addr_bytes);
            fake_spi.preprogram_data_response(tx.into(), vec![].into());
            fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        }

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(FlashAddress::new(0_u32), PowerOf2Usize::new(SIZE).unwrap())
            .unwrap();

        let log = fake_spi.log();
        let log = &log[log.len() - 3 * expectations.len()..];
        for (i, (op, addr_bytes)) in expectations.iter().enumerate() {
            let offset = i * 3;
            assert_eq!(log[offset].tx, vec![OP_WRITE_EN]);
            let mut expected_tx = vec![*op];
            expected_tx.extend_from_slice(addr_bytes);
            assert_eq!(log[offset + 1].tx, expected_tx);
            assert_eq!(log[offset + 2].tx, vec![OP_STATUS, 0x00]);
        }
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_all() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 128 * KIB);

        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_CHIP_ERASE]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.erase_all().unwrap();

        let log = fake_spi.log();
        let log = &log[(log.len() - 4)..];
        assert_eq!(
            log,
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_CHIP_ERASE],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, 0x01]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, 0x00]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_entire_flash_4b() {
        let fake_spi = FakeSpiDevice::new();
        const SIZE: usize = 32 * MIB;
        preprogram_init(&fake_spi, SIZE);

        const NUM_BLOCKS: usize = SIZE / (64 * KIB);
        for i in 0..NUM_BLOCKS {
            let addr = (i * 64 * KIB) as u32;
            let addr_bytes = addr.to_be_bytes();
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            let mut tx = vec![OP_ERASE_64K];
            tx.extend_from_slice(&addr_bytes);
            fake_spi.preprogram_data_response(tx.into(), vec![].into());
            fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        }

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(FlashAddress::new(0_u32), PowerOf2Usize::new(SIZE).unwrap())
            .unwrap();

        let log = fake_spi.log();
        let log = &log[log.len() - 3 * NUM_BLOCKS..];
        for i in 0..NUM_BLOCKS {
            let addr = (i * 64 * KIB) as u32;
            let addr_bytes = addr.to_be_bytes();
            let offset = i * 3;
            assert_eq!(log[offset].tx, vec![OP_WRITE_EN]);
            let mut expected_tx = vec![OP_ERASE_64K];
            expected_tx.extend_from_slice(&addr_bytes);
            assert_eq!(log[offset + 1].tx, expected_tx);
            assert_eq!(log[offset + 2].tx, vec![OP_STATUS, 0x00]);
        }
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_multiple_4k_pages_3b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);

        // Erase sequence: four 4K pages.
        const ADDRESS: usize = 0x1000;
        const LEN: usize = 16 * KIB;

        for i in 1..=4 {
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            fake_spi.preprogram_data_response(
                vec![OP_ERASE_4K, 0x00, (i * 0x10) as u8, 0x00].into(),
                vec![].into(),
            );
            fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        }

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(ADDRESS as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 12..],
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x00, 0x10, 0x00],
                    rx: vec![0x00, 0x00, 0x00, 0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY]
                },
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x00, 0x20, 0x00],
                    rx: vec![0x00, 0x00, 0x00, 0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY]
                },
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x00, 0x30, 0x00],
                    rx: vec![0x00, 0x00, 0x00, 0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY]
                },
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x00, 0x40, 0x00],
                    rx: vec![0x00, 0x00, 0x00, 0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_erase_block_aligned_small_len_3b() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 16 * MIB);

        // Address is 64K aligned, but length is < 64K. Should use 4K erases.
        const ADDRESS: usize = 0x10000;
        const LEN: usize = 8 * KIB;

        for i in 0..2 {
            fake_spi.preprogram_data_response(vec![OP_WRITE_EN].into(), vec![].into());
            fake_spi.preprogram_data_response(
                vec![OP_ERASE_4K, 0x01, (i * 0x10) as u8, 0x00].into(),
                vec![].into(),
            );
            fake_spi.preprogram_data_response(vec![OP_STATUS].into(), vec![STATUS_READY].into());
        }

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((ADDRESS) as u32),
                PowerOf2Usize::new(LEN).unwrap(),
            )
            .unwrap();

        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 6..],
            &[
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x01, 0x00, 0x00],
                    rx: vec![0, 0, 0, 0]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, STATUS_READY]
                },
                FakeSpiTransfer {
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00]
                },
                FakeSpiTransfer {
                    tx: vec![OP_ERASE_4K, 0x01, 0x10, 0x00],
                    rx: vec![0, 0, 0, 0]
                },
                FakeSpiTransfer {
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0, STATUS_READY]
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_program_3b() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_PROGRAM, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x74_1140_u32), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 4..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program
                    tx: vec![OP_PROGRAM, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_program_4b() {
        let fake_spi = FakeSpiDevice::new();
        // 128 MiB flash
        preprogram_init(&fake_spi, 128 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_PROGRAM, 0x02, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .program(
                FlashAddress::new(0x0274_1140_u32),
                &[0xba, 0x5e, 0xba, 0x11],
            )
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 4..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program
                    tx: vec![OP_PROGRAM, 0x02, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_qprogram_3b() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_QPROGRAM, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM;
        flash
            .program(FlashAddress::new(0x74_1140_u32), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 4..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program
                    tx: vec![OP_QPROGRAM, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_qprogram_4b() {
        let fake_spi = FakeSpiDevice::new();
        // 128 MiB flash
        preprogram_init(&fake_spi, 128 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[
                OP_QPROGRAM4B,
                0x02,
                0x74,
                0x11,
                0x40,
                0xba,
                0x5e,
                0xba,
                0x11,
            ])
                .into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM4B;
        flash
            .program(
                FlashAddress::new(0x0274_1140_u32),
                &[0xba, 0x5e, 0xba, 0x11],
            )
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 4..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program
                    tx: vec![
                        OP_QPROGRAM4B,
                        0x02,
                        0x74,
                        0x11,
                        0x40,
                        0xba,
                        0x5e,
                        0xba,
                        0x11
                    ],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_program_last_byte() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi
            .preprogram_data_response((&[OP_PROGRAM, 0x7f, 0xff, 0xff, 0x42]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x7f_ffff_u32), &[0x42])
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program
                    tx: vec![OP_PROGRAM, 0x7f, 0xff, 0xff, 0x42],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        // Programming past the end of the flash should result in an error
        assert_eq!(
            flash.program(FlashAddress::new(0x7f_ffff_u32), &[0x42, 0x42]),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_program_spans_pages() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_PROGRAM, 0x74, 0x11, 0xfd, 0xba, 0x5e, 0xba]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi
            .preprogram_data_response((&[OP_PROGRAM, 0x74, 0x12, 0x00, 0x11]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x74_11fd_u32), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 8..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program end of first page
                    tx: vec![OP_PROGRAM, 0x74, 0x11, 0xfd, 0xba, 0x5e, 0xba],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program start of next page
                    tx: vec![OP_PROGRAM, 0x74, 0x12, 0x00, 0x11],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_qprogram_last_byte() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_QPROGRAM, 0x7f, 0xff, 0xff, 0x42]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM;
        flash
            .program(FlashAddress::new(0x7f_ffff_u32), &[0x42])
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 3..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program
                    tx: vec![OP_QPROGRAM, 0x7f, 0xff, 0xff, 0x42],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        // Programming past the end of the flash should result in an error
        assert_eq!(
            flash.program(FlashAddress::new(0x7f_ffff_u32), &[0x42, 0x42]),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_qprogram_spans_pages() {
        let fake_spi = FakeSpiDevice::new();
        // 8 MiB flash
        preprogram_init(&fake_spi, 8 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_QPROGRAM, 0x74, 0x11, 0xfd, 0xba, 0x5e, 0xba]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response(
            (&[OP_QPROGRAM, 0x74, 0x12, 0x00, 0x11]).into(),
            (&[]).into(),
        );
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_WIP]).into());
        fake_spi.preprogram_data_response((&[OP_STATUS]).into(), (&[STATUS_READY]).into());

        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM;
        flash
            .program(FlashAddress::new(0x74_11fd_u32), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        assert_eq!(
            &fake_spi.log()[fake_spi.log().len() - 8..],
            &[
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program end of first page
                    tx: vec![OP_QPROGRAM, 0x74, 0x11, 0xfd, 0xba, 0x5e, 0xba],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
                FakeSpiTransfer {
                    // write-enable
                    tx: vec![OP_WRITE_EN],
                    rx: vec![0x00],
                },
                FakeSpiTransfer {
                    // program start of next page
                    tx: vec![OP_QPROGRAM, 0x74, 0x12, 0x00, 0x11],
                    rx: vec![0x00, 0x00, 0x00, 0x00, 0x00],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0, WIP=1
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_WIP],
                },
                FakeSpiTransfer {
                    // get-status: WRITE_EN=0,WIP=0
                    tx: vec![OP_STATUS, 0x00],
                    rx: vec![0x00, STATUS_READY],
                },
            ]
        );
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn test_set_ear() {
        let fake_spi = FakeSpiDevice::new();
        preprogram_init(&fake_spi, 128 * MIB);
        fake_spi.preprogram_data_response((&[OP_WRITE_EN]).into(), (&[]).into());
        fake_spi.preprogram_data_response((&[OP_WR_EAR, 0x02]).into(), (&[]).into());
        let mut flash = SpiFlash::new(fake_spi.clone());
        flash.init().unwrap();
        flash.set_ear(0x02).unwrap();
        fake_spi.assert_all_expectations_met();
    }

    #[test]
    fn address_size_is_valid_addr() {
        assert!(AddressingMode::_3Byte.is_valid_addr(0));
        assert!(AddressingMode::_3Byte.is_valid_addr(MAX_3B_SIZE - 1));
        assert!(!AddressingMode::_3Byte.is_valid_addr(MAX_3B_SIZE));

        assert!(AddressingMode::_4Byte.is_valid_addr(MAX_3B_SIZE));
        assert!(AddressingMode::_4Byte.is_valid_addr(0xffff_ffff));

        #[cfg(target_pointer_width = "8")]
        assert!(!AddressingMode::_4Byte.is_valid_addr(0x1_0000_0000));

        #[cfg(target_pointer_width = "8")]
        assert!(!AddressingMode::_4Byte.is_valid_addr(0xffff_ffff_ffff_ffff));
    }

    #[test]
    fn address_size_write_prefix() {
        let mut buf = [0xdd; MAX_PREFIX_LEN];
        assert_eq!(
            Ok([OP_READ, 0x12, 0x34, 0x56].as_slice()),
            AddressingMode::_3Byte.write_prefix(&mut buf, OP_READ, 0x12_3456)
        );

        let mut buf = [0xdd; MAX_PREFIX_LEN];
        assert_eq!(
            Ok([OP_READ, 0x12, 0x34, 0x56, 0x00].as_slice()),
            AddressingMode::_3ByteWithDummy.write_prefix(&mut buf, OP_READ, 0x12_3456)
        );

        let mut buf = [0xdd; MAX_PREFIX_LEN];
        assert_eq!(
            Ok([OP_READ, 0x12, 0x34, 0x56, 0x78].as_slice()),
            AddressingMode::_4Byte.write_prefix(&mut buf, OP_READ, 0x1234_5678)
        );

        let mut buf = [0xdd; MAX_PREFIX_LEN];
        assert_eq!(
            Ok([OP_READ, 0x12, 0x34, 0x56, 0x78, 0x00].as_slice()),
            AddressingMode::_4ByteWithDummy.write_prefix(&mut buf, OP_READ, 0x1234_5678)
        );

        assert_eq!(
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS),
            AddressingMode::_3Byte.write_prefix(&mut buf, OP_READ, 0x1234_5678)
        );
        assert_eq!(
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS),
            AddressingMode::_3ByteWithDummy.write_prefix(&mut buf, OP_READ, 0x1234_5678)
        );
    }
}
