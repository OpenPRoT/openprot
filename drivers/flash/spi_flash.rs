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

        if config.size.get() > 16 * MIB {
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
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum FakeSpiOp {
        Write(Vec<u8>),
        Read(usize),
        Transaction(Vec<FakeSpiOp>),
    }

    pub struct FakeSpiDevice {
        expected_ops: Vec<(FakeSpiOp, Vec<u8>)>,
        next_op_idx: usize,
    }

    impl FakeSpiDevice {
        pub fn new(expected: Vec<(FakeSpiOp, Vec<u8>)>) -> Self {
            Self {
                expected_ops: expected,
                next_op_idx: 0,
            }
        }

        pub fn verify(&self) {
            assert_eq!(
                self.next_op_idx,
                self.expected_ops.len(),
                "Not all expected SPI operations were executed"
            );
        }
    }

    impl embedded_hal::spi::ErrorType for FakeSpiDevice {
        type Error = core::convert::Infallible;
    }

    impl embedded_hal::spi::SpiDevice for FakeSpiDevice {
        fn transaction(
            &mut self,
            operations: &mut [embedded_hal::spi::Operation<'_, u8>],
        ) -> Result<(), Self::Error> {
            let mut transaction_ops = Vec::new();
            for op in operations.iter() {
                match op {
                    embedded_hal::spi::Operation::Write(buf) => {
                        transaction_ops.push(FakeSpiOp::Write(buf.to_vec()));
                    }
                    embedded_hal::spi::Operation::Read(buf) => {
                        transaction_ops.push(FakeSpiOp::Read(buf.len()));
                    }
                    _ => unimplemented!(),
                }
            }

            assert!(
                self.next_op_idx < self.expected_ops.len(),
                "Unexpected SPI transaction"
            );
            let (expected_op, response) = &self.expected_ops[self.next_op_idx];
            self.next_op_idx += 1;

            if let FakeSpiOp::Transaction(expected_sub_ops) = expected_op {
                assert_eq!(
                    transaction_ops.len(),
                    expected_sub_ops.len(),
                    "Transaction sub-operations count mismatch"
                );
                let mut resp_idx = 0;
                for (i, op) in operations.iter_mut().enumerate() {
                    match op {
                        embedded_hal::spi::Operation::Write(buf) => {
                            if let FakeSpiOp::Write(expected_buf) = &expected_sub_ops[i] {
                                assert_eq!(buf, expected_buf, "Transaction write content mismatch");
                            } else {
                                panic!("Expected Write op, got {:?}", expected_sub_ops[i]);
                            }
                        }
                        embedded_hal::spi::Operation::Read(buf) => {
                            if let FakeSpiOp::Read(expected_len) = expected_sub_ops[i] {
                                assert_eq!(
                                    buf.len(),
                                    expected_len,
                                    "Transaction read length mismatch"
                                );
                                let chunk = &response[resp_idx..resp_idx + expected_len];
                                buf.copy_from_slice(chunk);
                                resp_idx += expected_len;
                            } else {
                                panic!("Expected Read op, got {:?}", expected_sub_ops[i]);
                            }
                        }
                        _ => unimplemented!(),
                    }
                }
            } else {
                panic!("Expected transaction op, got {:?}", expected_op);
            }

            Ok(())
        }

        fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
            assert!(
                self.next_op_idx < self.expected_ops.len(),
                "Unexpected SPI write"
            );
            let (expected_op, _) = &self.expected_ops[self.next_op_idx];
            self.next_op_idx += 1;

            if let FakeSpiOp::Write(expected_buf) = expected_op {
                assert_eq!(buf, expected_buf, "Write content mismatch");
            } else {
                panic!("Expected Write, got {:?}", expected_op);
            }
            Ok(())
        }

        fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
            assert!(
                self.next_op_idx < self.expected_ops.len(),
                "Unexpected SPI read"
            );
            let (expected_op, response) = &self.expected_ops[self.next_op_idx];
            self.next_op_idx += 1;

            if let FakeSpiOp::Read(expected_len) = expected_op {
                assert_eq!(buf.len(), *expected_len, "Read length mismatch");
                buf.copy_from_slice(response);
            } else {
                panic!("Expected Read, got {:?}", expected_op);
            }
            Ok(())
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

    /// Configure the expected operations to playback the expected SFDP read pattern
    /// when requested.
    fn preprogram_sfdp(expected_ops: &mut Vec<(FakeSpiOp, Vec<u8>)>, size_bytes: usize) {
        let sfdp = gen_sfdp(size_bytes);
        expected_ops.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_SFDP_READ, 0, 0, 0, 0]),
                FakeSpiOp::Read(8),
            ]),
            sfdp[..8].to_vec(),
        ));
        expected_ops.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_SFDP_READ, 0, 0, 8, 0]),
                FakeSpiOp::Read(8),
            ]),
            sfdp[8..16].to_vec(),
        ));
        expected_ops.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_SFDP_READ, 0, 0, 16, 0]),
                FakeSpiOp::Read(92),
            ]),
            sfdp[16..].to_vec(),
        ));
    }

    /// Configure the expected operations to playback the entire initialization sequence
    /// (SFDP read, status register check, and optional 4-byte address mode transition).
    fn preprogram_init(expected_ops: &mut Vec<(FakeSpiOp, Vec<u8>)>, size_bytes: usize) {
        preprogram_sfdp(expected_ops, size_bytes);
        expected_ops.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![STATUS_READY],
        ));
        if size_bytes > 16 * MIB {
            expected_ops.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            expected_ops.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));
        }
    }

    #[test]
    fn test_size_8mb() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 8 * MIB);

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        assert_eq!(flash.config.size.get(), 8 * MIB);
        flash.spi.verify();
    }

    #[test]
    fn test_size_128mb() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 128 * MIB);

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        assert_eq!(flash.config.size.get(), 128 * MIB);
        flash.spi.verify();
    }

    #[test]
    fn test_read_3b() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_READ, 0x12, 0x34, 0x56]),
                FakeSpiOp::Read(12),
            ]),
            b"Hello World!".to_vec(),
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        let mut buf = [0u8; 12];
        flash.read(FlashAddress::new(0x12_3456), &mut buf).unwrap();
        assert_eq!(&buf, b"Hello World!");

        // Out of bounds tests
        assert_eq!(
            flash.read(FlashAddress::new((8 * MIB - 6) as u32), &mut buf),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        assert_eq!(
            flash.read(FlashAddress::new((8 * MIB) as u32), &mut buf),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );

        flash.spi.verify();
    }

    #[test]
    fn test_read_4b() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 32 * MIB);

        expected.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_READ, 0x00, 0x12, 0x34, 0x56]),
                FakeSpiOp::Read(5),
            ]),
            b"Hello".to_vec(),
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_READ, 0x01, 0x23, 0x45, 0x67]),
                FakeSpiOp::Read(5),
            ]),
            b"World".to_vec(),
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        let mut buf = [0u8; 5];
        flash
            .read(FlashAddress::new(0x0012_3456), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello");
        flash
            .read(FlashAddress::new(0x0123_4567), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"World");
        flash.spi.verify();
    }

    #[test]
    fn test_read_4b_qspi() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 32 * MIB);

        // Expect OP_QREAD4B + 4B Address + 1 Dummy Byte (0x00)
        expected.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_QREAD4B, 0x00, 0x12, 0x34, 0x56, 0x00]),
                FakeSpiOp::Read(5),
            ]),
            b"Hello".to_vec(),
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.read = SfCmd::QREAD4B;

        let mut buf = [0u8; 5];
        flash
            .read(FlashAddress::new(0x0012_3456), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello");
        flash.spi.verify();
    }

    #[test]
    fn test_qread_3b() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 8 * MIB);
        // Expect OP_QREAD + 3B Address + 1 Dummy Byte (0x00)
        expected.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_QREAD, 0x12, 0x34, 0x56, 0x00]),
                FakeSpiOp::Read(5),
            ]),
            b"Hello".to_vec(),
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.read = SfCmd::QREAD;

        let mut buf = [0u8; 5];
        flash.read(FlashAddress::new(0x12_3456), &mut buf).unwrap();
        assert_eq!(&buf, b"Hello");
        flash.spi.verify();
    }

    #[test]
    fn test_qread_4b() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 32 * MIB);

        // Expect OP_QREAD4B + 4B Address + 1 Dummy Byte (0x00)
        expected.push((
            FakeSpiOp::Transaction(vec![
                FakeSpiOp::Write(vec![OP_QREAD4B, 0x00, 0x12, 0x34, 0x56, 0x00]),
                FakeSpiOp::Read(5),
            ]),
            b"Hello".to_vec(),
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.read = SfCmd::QREAD4B;

        let mut buf = [0u8; 5];
        flash
            .read(FlashAddress::new(0x0012_3456), &mut buf)
            .unwrap();
        assert_eq!(&buf, b"Hello");
        flash.spi.verify();
    }

    #[test]
    fn test_erase_3b() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, 16 * MIB);

        // write-enable
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        // erase
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_4K, 0xba, 0x10, 0x00]),
            vec![],
        ));
        // get-status: WRITE_EN=1, WIP=1
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![STATUS_WIP_WEL],
        ));
        // get-status: WRITE_EN=0, WIP=1
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![STATUS_WIP],
        ));
        // get-status: WRITE_EN=0, WIP=0
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![STATUS_READY],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        flash
            .erase(
                FlashAddress::new(0xba_1000),
                PowerOf2Usize::new(4096).unwrap(),
            )
            .unwrap();
        flash.spi.verify();

        assert_eq!(
            flash.erase(
                FlashAddress::new(0xba_1001),
                PowerOf2Usize::new(4096).unwrap(),
            ),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new((16 * MIB) as u32),
                PowerOf2Usize::new(4096).unwrap(),
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
    }

    #[test]
    fn test_erase_4b() {
        let mut expected = Vec::new();
        preprogram_init(&mut expected, GIB);

        // write-enable
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        // erase (new driver uses OP_ERASE_4K in 4B mode)
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_4K, 0x1a, 0x5e, 0xb0, 0x00]),
            vec![],
        ));
        // get-status: WRITE_EN=1, WIP=1
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![STATUS_WIP_WEL],
        ));
        // get-status: WRITE_EN=0, WIP=0
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![STATUS_READY],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        flash
            .erase(
                FlashAddress::new(0x1a5e_b000),
                PowerOf2Usize::new(4096).unwrap(),
            )
            .unwrap();
        flash.spi.verify();

        assert_eq!(
            flash.erase(
                FlashAddress::new(0xba_1001),
                PowerOf2Usize::new(4096).unwrap(),
            ),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        assert_eq!(
            flash.erase(
                FlashAddress::new(GIB as u32),
                PowerOf2Usize::new(4096).unwrap(),
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
    }

    #[test]
    fn test_erase_last_page() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_4K, 0x7f, 0xf0, 0x00]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        flash
            .erase(
                FlashAddress::new((8 * MIB - 4096) as u32),
                PowerOf2Usize::new(4096).unwrap(),
            )
            .unwrap();

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
        flash.spi.verify();
    }

    #[test]
    fn test_erase_single_page_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_4K, 0x00, 0x10, 0x00]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(FlashAddress::new(0x1000), PowerOf2Usize::new(4096).unwrap())
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_single_page_4b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 32 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_4K, 0x00, 0x00, 0x10, 0x00]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(FlashAddress::new(0x1000), PowerOf2Usize::new(4096).unwrap())
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_single_block_64k_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_64K, 0x01, 0x00, 0x00]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(0x10000),
                PowerOf2Usize::new(65536).unwrap(),
            )
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_single_block_64k_4b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 32 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_ERASE_64K, 0x01, 0x23, 0x00, 0x00]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(0x01230000),
                PowerOf2Usize::new(65536).unwrap(),
            )
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_mixed_granularity_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // Mixed test: We use a size of 128KB (Power of 2).
        // Starting at 56KB, to verify it uses page erase at 56KB, 60KB, and 64KB block erase.
        let ops = [
            (OP_ERASE_4K, vec![0x00, 0xE0, 0x00]),  // 56KB
            (OP_ERASE_4K, vec![0x00, 0xF0, 0x00]),  // 60KB
            (OP_ERASE_64K, vec![0x01, 0x00, 0x00]), // 64KB
            (OP_ERASE_4K, vec![0x02, 0x00, 0x00]),  // 128KB
            (OP_ERASE_4K, vec![0x02, 0x10, 0x00]),  // 132KB
            (OP_ERASE_4K, vec![0x02, 0x20, 0x00]),  // 136KB
            (OP_ERASE_4K, vec![0x02, 0x30, 0x00]),  // 140KB
            (OP_ERASE_4K, vec![0x02, 0x40, 0x00]),  // 144KB
            (OP_ERASE_4K, vec![0x02, 0x50, 0x00]),  // 148KB
            (OP_ERASE_4K, vec![0x02, 0x60, 0x00]),  // 152KB
            (OP_ERASE_4K, vec![0x02, 0x70, 0x00]),  // 156KB
            (OP_ERASE_4K, vec![0x02, 0x80, 0x00]),  // 160KB
            (OP_ERASE_4K, vec![0x02, 0x90, 0x00]),  // 164KB
            (OP_ERASE_4K, vec![0x02, 0xA0, 0x00]),  // 168KB
            (OP_ERASE_4K, vec![0x02, 0xB0, 0x00]),  // 172KB
            (OP_ERASE_4K, vec![0x02, 0xC0, 0x00]),  // 176KB
            (OP_ERASE_4K, vec![0x02, 0xD0, 0x00]),  // 180KB
        ];

        for (op, addr) in &ops {
            expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            let mut cmd = vec![*op];
            cmd.extend_from_slice(addr);
            expected.push((FakeSpiOp::Write(cmd), vec![]));
            expected.push((
                FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
                vec![0x00],
            ));
        }

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((56 * KIB) as u32),
                PowerOf2Usize::new(128 * KIB).unwrap(),
            )
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_mixed_granularity_4b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 32 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        // Mixed test: We use a size of 128KB (Power of 2).
        // Starting at MAX_3B_SIZE - 8KB, which is 16MB - 8KB.
        let ops = [
            (OP_ERASE_4K, vec![0x00, 0xFF, 0xE0, 0x00]),
            (OP_ERASE_4K, vec![0x00, 0xFF, 0xF0, 0x00]),
            (OP_ERASE_64K, vec![0x01, 0x00, 0x00, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x00, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x10, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x20, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x30, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x40, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x50, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x60, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x70, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x80, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0x90, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0xA0, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0xB0, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0xC0, 0x00]),
            (OP_ERASE_4K, vec![0x01, 0x01, 0xD0, 0x00]),
        ];

        for (op, addr) in &ops {
            expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            let mut cmd = vec![*op];
            cmd.extend_from_slice(addr);
            expected.push((FakeSpiOp::Write(cmd), vec![]));
            expected.push((
                FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
                vec![0x00],
            ));
        }

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new((MAX_3B_SIZE - 8 * KIB) as u32),
                PowerOf2Usize::new(128 * KIB).unwrap(),
            )
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_alignment_errors() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        assert_eq!(
            flash.erase(FlashAddress::new(1), PowerOf2Usize::new(4096).unwrap()),
            Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR)
        );
        // 4095 is not a power of 2, so it would fail compile or fail new().
        // We test with PowerOf2Usize::new(1) which is a power of 2 but invalid size (not multiple of 4K).
        assert_eq!(
            flash.erase(FlashAddress::new(0), PowerOf2Usize::new(1).unwrap()),
            Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE)
        );
    }

    #[test]
    fn test_erase_out_of_bounds() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        assert_eq!(
            flash.erase(
                FlashAddress::new((16 * MIB - 4096) as u32),
                PowerOf2Usize::new(8192).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );

        // 4B flash (32 MiB)
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 32 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();

        assert_eq!(
            flash.erase(
                FlashAddress::new((32 * MIB) as u32),
                PowerOf2Usize::new(4096).unwrap()
            ),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
    }

    #[test]
    fn test_erase_entire_flash_3b() {
        let mut expected = Vec::new();
        const SIZE: usize = 128 * KIB;
        preprogram_sfdp(&mut expected, SIZE);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let ops = [
            (OP_ERASE_64K, vec![0x00, 0x00, 0x00]),
            (OP_ERASE_64K, vec![0x01, 0x00, 0x00]),
        ];

        for (op, addr) in &ops {
            expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            let mut cmd = vec![*op];
            cmd.extend_from_slice(addr);
            expected.push((FakeSpiOp::Write(cmd), vec![]));
            expected.push((
                FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
                vec![0x00],
            ));
        }

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(FlashAddress::new(0), PowerOf2Usize::new(SIZE).unwrap())
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_all() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 128 * KIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_CHIP_ERASE]), vec![]));
        // status busy check loop
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01], // busy
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00], // ready
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.erase_all().unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_entire_flash_4b() {
        let mut expected = Vec::new();
        const SIZE: usize = 32 * MIB;
        preprogram_sfdp(&mut expected, SIZE);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        let num_blocks = SIZE / (64 * KIB);
        for i in 0..num_blocks {
            let addr = (i * 64 * KIB) as u32;
            expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            let mut cmd = vec![OP_ERASE_64K]; // OP_ERASE_64K in B7 mode
            cmd.extend_from_slice(&addr.to_be_bytes());
            expected.push((FakeSpiOp::Write(cmd), vec![]));
            expected.push((
                FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
                vec![0x00],
            ));
        }

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(FlashAddress::new(0), PowerOf2Usize::new(SIZE).unwrap())
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_multiple_4k_pages_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // 16KB is a valid power of 2 (erases four 4K pages)
        for i in 1..=4 {
            expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            expected.push((
                FakeSpiOp::Write(vec![OP_ERASE_4K, 0x00, (i * 0x10) as u8, 0x00]),
                vec![],
            ));
            expected.push((
                FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
                vec![0x00],
            ));
        }

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(0x1000),
                PowerOf2Usize::new(16 * KIB).unwrap(),
            )
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_erase_block_aligned_small_len_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 16 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // 8KB is a power of 2, erases two 4K pages
        for i in 0..2 {
            expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
            expected.push((
                FakeSpiOp::Write(vec![OP_ERASE_4K, 0x01, (i * 0x10) as u8, 0x00]),
                vec![],
            ));
            expected.push((
                FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
                vec![0x00],
            ));
        }

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .erase(
                FlashAddress::new(0x10000),
                PowerOf2Usize::new(8 * KIB).unwrap(),
            )
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_program_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_PROGRAM, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01], // busy
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00], // ready
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x74_1140), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_program_4b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 128 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        // Expect OP_PROGRAM (0x02) but with 4 address bytes under B7 mode
        expected.push((
            FakeSpiOp::Write(vec![
                OP_PROGRAM, 0x02, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11,
            ]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x0274_1140), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_qprogram_3b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_QPROGRAM, 0x74, 0x11, 0x40, 0xba, 0x5e, 0xba, 0x11]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM;
        flash
            .program(FlashAddress::new(0x74_1140), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_qprogram_4b() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 128 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![
                OP_QPROGRAM4B,
                0x02,
                0x74,
                0x11,
                0x40,
                0xba,
                0x5e,
                0xba,
                0x11,
            ]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM4B;
        flash
            .program(FlashAddress::new(0x0274_1140), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_program_last_byte() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_PROGRAM, 0x7f, 0xff, 0xff, 0x42]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x7f_ffff), &[0x42])
            .unwrap();

        // Check overflow past end of flash
        assert_eq!(
            flash.program(FlashAddress::new(0x7f_ffff), &[0x42, 0x42]),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        flash.spi.verify();
    }

    #[test]
    fn test_program_spans_pages() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // Page 1 Write
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_PROGRAM, 0x74, 0x11, 0xfd, 0xba, 0x5e, 0xba]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // Page 2 Write
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_PROGRAM, 0x74, 0x12, 0x00, 0x11]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash
            .program(FlashAddress::new(0x74_11fd), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_qprogram_last_byte() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_QPROGRAM, 0x7f, 0xff, 0xff, 0x42]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM;
        flash
            .program(FlashAddress::new(0x7f_ffff), &[0x42])
            .unwrap();

        assert_eq!(
            flash.program(FlashAddress::new(0x7f_ffff), &[0x42, 0x42]),
            Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
        );
        flash.spi.verify();
    }

    #[test]
    fn test_qprogram_spans_pages() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 8 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // Page 1
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_QPROGRAM, 0x74, 0x11, 0xfd, 0xba, 0x5e, 0xba]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        // Page 2
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((
            FakeSpiOp::Write(vec![OP_QPROGRAM, 0x74, 0x12, 0x00, 0x11]),
            vec![],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x01],
        ));
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.config.program = SfCmd::QPROGRAM;
        flash
            .program(FlashAddress::new(0x74_11fd), &[0xba, 0x5e, 0xba, 0x11])
            .unwrap();
        flash.spi.verify();
    }

    #[test]
    fn test_set_ear() {
        let mut expected = Vec::new();
        preprogram_sfdp(&mut expected, 128 * MIB);
        expected.push((
            FakeSpiOp::Transaction(vec![FakeSpiOp::Write(vec![OP_STATUS]), FakeSpiOp::Read(1)]),
            vec![0x00],
        ));
        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_ENTER_4B_ADDR_MODE]), vec![]));

        expected.push((FakeSpiOp::Write(vec![OP_WRITE_EN]), vec![]));
        expected.push((FakeSpiOp::Write(vec![OP_WR_EAR, 0x02]), vec![]));

        let spi = FakeSpiDevice::new(expected);
        let mut flash = SpiFlash::new(spi);
        flash.init().unwrap();
        flash.set_ear(0x02).unwrap();
        flash.spi.verify();
    }

    #[test]
    fn address_size_is_valid_addr() {
        assert!(AddressingMode::_3Byte.is_valid_addr(0));
        assert!(AddressingMode::_3Byte.is_valid_addr(MAX_3B_SIZE - 1));
        assert!(!AddressingMode::_3Byte.is_valid_addr(MAX_3B_SIZE));

        assert!(AddressingMode::_4Byte.is_valid_addr(MAX_3B_SIZE));
        assert!(AddressingMode::_4Byte.is_valid_addr(0xffff_ffff));
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
    }
}
