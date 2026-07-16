// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPIPF allow-command table encoding.

use crate::spimonitor::registers::SpiMonitorRegisters;

/// Number of SPIPFWT slots (0..=31).
pub const SPIM_CMD_TABLE_NUM: usize = 32;
/// Last fixed/dynamic slot index.
pub const MAX_CMD_INDEX: usize = 31;

/// general command 13
pub const CMD_RDID: u8 = 0x9F;
pub const CMD_WREN: u8 = 0x06;
pub const CMD_WRDIS: u8 = 0x04;
pub const CMD_RDSR: u8 = 0x05;
pub const CMD_RDCR: u8 = 0x15;
pub const CMD_RDSR2: u8 = 0x35;
pub const CMD_WRSR: u8 = 0x01;
pub const CMD_WRSR2: u8 = 0x31;
pub const CMD_SFDP: u8 = 0x5A;
pub const CMD_EN4B: u8 = 0xB7;
pub const CMD_EX4B: u8 = 0xE9;
pub const CMD_RDFSR: u8 = 0x70;
pub const CMD_VSR_WREN: u8 = 0x50;

/// read commands 12
pub const CMD_READ_1_1_1_3B: u8 = 0x03;
pub const CMD_READ_1_1_1_4B: u8 = 0x13;
pub const CMD_FREAD_1_1_1_3B: u8 = 0x0B;
pub const CMD_FREAD_1_1_1_4B: u8 = 0x0C;
pub const CMD_READ_1_1_2_3B: u8 = 0x3B;
pub const CMD_READ_1_1_2_4B: u8 = 0x3C;
pub const CMD_READ_1_2_2_3B: u8 = 0xBB;
pub const CMD_READ_1_2_2_4B: u8 = 0xBC;
pub const CMD_READ_1_1_4_3B: u8 = 0x6B;
pub const CMD_READ_1_1_4_4B: u8 = 0x6C;
pub const CMD_READ_1_4_4_3B: u8 = 0xEB;
pub const CMD_READ_1_4_4_4B: u8 = 0xEC;

// write command 6
pub const CMD_PP_1_1_1_3B: u8 = 0x02;
pub const CMD_PP_1_1_1_4B: u8 = 0x12;
pub const CMD_PP_1_1_4_3B: u8 = 0x32;
pub const CMD_PP_1_1_4_4B: u8 = 0x34;
pub const CMD_PP_1_4_4_3B: u8 = 0x38;
pub const CMD_PP_1_4_4_4B: u8 = 0x3E;

// sector erase command 4
pub const CMD_SE_1_1_0_3B: u8 = 0x20;
pub const CMD_SE_1_1_0_4B: u8 = 0x21;
pub const CMD_SE_1_1_0_64_3B: u8 = 0xD8;
pub const CMD_SE_1_1_0_64_4B: u8 = 0xDC;

// Write Extend Address Register
pub const CMD_WREAR: u8 = 0xC5;
/// Winbond die select
pub const CMD_WINBOND_DIE_SEL: u8 = 0xC2;


pub const CMD_TABLE_LOCK_MASK: u32 = 1 << 23;
pub const CMD_TABLE_VALID_ONCE_BIT: u32 = 1 << 31;
pub const CMD_TABLE_VALID_BIT: u32 = 1 << 30;
pub const CMD_TABLE_CMD_MASK: u32 = 0xFF;




#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CmdTableInfo {
    cmd: u8,
    reserved: [u8; 3],
    cmd_table_val: u32,
}

//#[derive(Debug, Clone, Copy)]
//pub struct GpioInfo {

//}
//compile time - const fn
#[allow(clippy::too_many_arguments)]
#[must_use]
pub const fn cmd_table_value(
    g: u32,
    w: u32,
    r: u32,
    m: u32,
    dat_mode: u32,
    dummy: u32,
    prog_sz: u32,
    addr_len: u32,
    addr_mode: u32,
    cmd: u32,
) -> u32 {
    (g << 29)
        | (w << 28)
        | (r << 27)
        | (m << 26)
        | (dat_mode << 24)
        | (dummy << 16)
        | (prog_sz << 13)
        | (addr_len << 10)
        | (addr_mode << 8)
        | cmd
}

//32 Allow Command Table Entries
//total commands: 36
static CMDS_ARRAY: &[CmdTableInfo] = &[
    CmdTableInfo {
        cmd: CMD_READ_1_1_1_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 0, 0, 3, 1, CMD_READ_1_1_1_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_1_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 0, 0, 4, 1, CMD_READ_1_1_1_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_FREAD_1_1_1_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 8, 0, 3, 1, CMD_FREAD_1_1_1_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_FREAD_1_1_1_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 8, 0, 4, 1, CMD_FREAD_1_1_1_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_2_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 8, 0, 3, 1, CMD_READ_1_1_2_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_2_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 8, 0, 4, 1, CMD_READ_1_1_2_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_2_2_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 4, 0, 3, 2, CMD_READ_1_2_2_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_2_2_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 4, 0, 4, 2, CMD_READ_1_2_2_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_4_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 8, 0, 3, 1, CMD_READ_1_1_4_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_4_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 8, 0, 4, 1, CMD_READ_1_1_4_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_4_4_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 6, 0, 3, 3, CMD_READ_1_4_4_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_4_4_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 6, 0, 4, 3, CMD_READ_1_4_4_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_1_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 1, 0, 1, 3, 1, CMD_PP_1_1_1_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_1_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 1, 0, 1, 4, 1, CMD_PP_1_1_1_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_4_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 3, 0, 1, 3, 1, CMD_PP_1_1_4_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_4_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 3, 0, 1, 4, 1, CMD_PP_1_1_4_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 1, 3, 1, CMD_SE_1_1_0_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 1, 4, 1, CMD_SE_1_1_0_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_64_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 5, 3, 1, CMD_SE_1_1_0_64_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_64_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 5, 4, 1, CMD_SE_1_1_0_64_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_WREN,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 0, 0, 0, 0, 0, 0, 0, CMD_WREN as u32),
    },
    CmdTableInfo {
        cmd: CMD_WRDIS,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 0, 0, 0, 0, 0, 0, 0, CMD_WRDIS as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDSR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDSR as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDSR2,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDSR2 as u32),
    },
    CmdTableInfo {
        cmd: CMD_WRSR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WRSR as u32),
    },
    CmdTableInfo {
        cmd: CMD_WRSR2,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WRSR2 as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDCR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDCR as u32),
    },
    CmdTableInfo {
        cmd: CMD_EN4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(0, 0, 0, 0, 0, 0, 0, 0, 0, CMD_EN4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_EX4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(0, 0, 0, 0, 0, 0, 0, 0, 0, CMD_EX4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SFDP,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 8, 0, 3, 1, CMD_SFDP as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDID,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDID as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDFSR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDFSR as u32),
    },
    CmdTableInfo {
        cmd: CMD_VSR_WREN,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 0, 0, 0, 0, 0, 0, 0, CMD_VSR_WREN as u32),
    },
    CmdTableInfo {
        cmd: CMD_WREAR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(0, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WREAR as u32),
    },
    CmdTableInfo {
        cmd: CMD_WINBOND_DIE_SEL,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WINBOND_DIE_SEL as u32),
    },
];

/// Look up the base encoded word for `cmd` (without VALID bit).
#[must_use]
pub fn lookup_cmd_table_val(cmd: u8) -> Option<u32> {
    for entry in CMDS_ARRAY {
        if entry.cmd == cmd {
            return Some(entry.cmd_table_val);
        }
    }
    None
}

/// Program SPIPFWT from an opcode list.
///
/// Fixed slots: `EN4B`→0, `EX4B`→1, `WREAR`→31. Dynamic opcodes fill slots 2..31.
/// Unknown opcodes are skipped.
pub fn init_allow_cmd_table(regs: &SpiMonitorRegisters, cmd_list: &[u8]) {
    let mut idx = 1usize;
    for &cmd in cmd_list {
        let Some(mut reg_val) = lookup_cmd_table_val(cmd) else {
            continue;
        };
        reg_val |= CMD_TABLE_VALID_BIT;

        match cmd {
            CMD_EN4B => {
                regs.write_allow_cmd_slot(0, reg_val);
                continue;
            }
            CMD_EX4B => {
                regs.write_allow_cmd_slot(1, reg_val);
                continue;
            }
            CMD_WREAR => {
                regs.write_allow_cmd_slot(MAX_CMD_INDEX, reg_val);
                continue;
            }
            _ => {
                idx += 1;
            }
        }

        if idx > MAX_CMD_INDEX {
            break;
        }
        regs.write_allow_cmd_slot(idx, reg_val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_03_encodes_with_valid_bit() {
        let base = lookup_cmd_table_val(CMD_READ_1_1_1_3B).expect("0x03 in table");
        let encoded = base | CMD_TABLE_VALID_BIT;
        assert_eq!(encoded & 0xFF, 0x03);
        assert_ne!(encoded & CMD_TABLE_VALID_BIT, 0);
    }

    #[test]
    fn winbond_die_sel_in_table() {
        assert!(lookup_cmd_table_val(CMD_WINBOND_DIE_SEL).is_some());
    }
}
