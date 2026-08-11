// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Standard SPI Flash opcodes.

#![no_std]

use core::ops::Deref;

/// Standard SPI Flash Command Opcode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(transparent)]
pub struct Opcode(pub u8);

impl Opcode {
    // Read commands
    pub const READ: Self = Self(0x03);
    pub const FAST_READ: Self = Self(0x0B);
    pub const FAST_QUAD_READ: Self = Self(0x6B);
    pub const READ_4B: Self = Self(0x13);
    pub const FAST_QUAD_READ_4B: Self = Self(0x6C);

    // Program commands
    pub const PAGE_PROGRAM: Self = Self(0x02);
    pub const PAGE_PROGRAM_QUAD: Self = Self(0x32);
    pub const PAGE_PROGRAM_4B: Self = Self(0x12);
    pub const PAGE_PROGRAM_QUAD_4B: Self = Self(0x34);

    // Erase commands
    pub const SECTOR_ERASE: Self = Self(0x20);
    pub const SECTOR_ERASE_4B: Self = Self(0x21);
    pub const BLOCK_ERASE_32K: Self = Self(0x52);
    pub const BLOCK_ERASE_32K_4B: Self = Self(0x5C);
    pub const BLOCK_ERASE_64K: Self = Self(0xD8);
    pub const BLOCK_ERASE_64K_4B: Self = Self(0xDC);
    pub const CHIP_ERASE: Self = Self(0xC7);
    pub const CHIP_ERASE2: Self = Self(0x60);

    // Control and Status commands
    pub const WRITE_ENABLE: Self = Self(0x06);
    pub const WRITE_DISABLE: Self = Self(0x04);
    pub const READ_STATUS: Self = Self(0x05);
    pub const WRITE_STATUS: Self = Self(0x01);
    pub const WRITE_EAR: Self = Self(0xC5);
    pub const ENTER_4B_ADDR_MODE: Self = Self(0xB7);
    pub const EXIT_4B_ADDR_MODE: Self = Self(0xE9);
    pub const RESET_ENABLE: Self = Self(0x66);
    pub const RESET: Self = Self(0x99);

    // Identification and Parameters commands
    pub const SFDP: Self = Self(0x5A);
    pub const JEDEC_ID: Self = Self(0x9F);
}

impl Deref for Opcode {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

impl From<Opcode> for u8 {
    fn from(val: Opcode) -> Self {
        val.0
    }
}

impl From<Opcode> for u32 {
    fn from(val: Opcode) -> Self {
        val.0 as u32
    }
}

// Backward-compatible u8 constants mapped to canonical Opcode definitions
pub const OP_STATUS: u8 = Opcode::READ_STATUS.0;
pub const OP_WRITE_EN: u8 = Opcode::WRITE_ENABLE.0;
pub const OP_WR_STATUS: u8 = Opcode::WRITE_STATUS.0;
pub const OP_WR_EAR: u8 = Opcode::WRITE_EAR.0;
pub const OP_READ: u8 = Opcode::READ.0;
pub const OP_QREAD: u8 = Opcode::FAST_QUAD_READ.0;
pub const OP_READ4B: u8 = Opcode::READ_4B.0;
pub const OP_QREAD4B: u8 = Opcode::FAST_QUAD_READ_4B.0;
pub const OP_CHIP_ERASE: u8 = Opcode::CHIP_ERASE.0;
pub const OP_ERASE_4K: u8 = Opcode::SECTOR_ERASE.0;
pub const OP_ERASE4B_4K: u8 = Opcode::SECTOR_ERASE_4B.0;
pub const OP_ERASE_64K: u8 = Opcode::BLOCK_ERASE_64K.0;
pub const OP_ERASE4B_64K: u8 = Opcode::BLOCK_ERASE_64K_4B.0;
pub const OP_PROGRAM: u8 = Opcode::PAGE_PROGRAM.0;
pub const OP_QPROGRAM: u8 = Opcode::PAGE_PROGRAM_QUAD.0;
pub const OP_PROGRAM4B: u8 = Opcode::PAGE_PROGRAM_4B.0;
pub const OP_QPROGRAM4B: u8 = Opcode::PAGE_PROGRAM_QUAD_4B.0;
pub const OP_SFDP_READ: u8 = Opcode::SFDP.0;
pub const OP_RESET_ENABLE: u8 = Opcode::RESET_ENABLE.0;
pub const OP_RESET: u8 = Opcode::RESET.0;
pub const OP_READ_JEDEC_ID: u8 = Opcode::JEDEC_ID.0;
pub const OP_ENTER_4B_ADDR_MODE: u8 = Opcode::ENTER_4B_ADDR_MODE.0;
