// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Hardware command-table descriptors for the AST10x0 SPI monitor.

/// A decoded command descriptor before the valid/lock state is applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandDescriptor {
    pub opcode: u8,
    pub generic: bool,
    pub write: bool,
    pub read: bool,
    pub memory: bool,
    pub data_width: u8,
    pub dummy_cycles: u8,
    pub program_size: u8,
    pub address_len: u8,
    pub address_width: u8,
}

impl CommandDescriptor {
    #[must_use]
    pub const fn encode(self) -> u32 {
        ((self.generic as u32) << 29)
            | ((self.write as u32) << 28)
            | ((self.read as u32) << 27)
            | ((self.memory as u32) << 26)
            | ((self.data_width as u32) << 24)
            | ((self.dummy_cycles as u32) << 16)
            | ((self.program_size as u32) << 13)
            | ((self.address_len as u32) << 10)
            | ((self.address_width as u32) << 8)
            | self.opcode as u32
    }
}

const fn command(
    opcode: u8,
    generic: bool,
    write: bool,
    read: bool,
    memory: bool,
    data_width: u8,
    dummy_cycles: u8,
    program_size: u8,
    address_len: u8,
    address_width: u8,
) -> CommandDescriptor {
    CommandDescriptor {
        opcode,
        generic,
        write,
        read,
        memory,
        data_width,
        dummy_cycles,
        program_size,
        address_len,
        address_width,
    }
}

/// Return the descriptor used by the Zephyr AST10x0 SPI monitor driver.
#[must_use]
pub const fn descriptor(opcode: u8) -> Option<CommandDescriptor> {
    let entry = match opcode {
        0x03 => command(opcode, true, false, true, true, 1, 0, 0, 3, 1),
        0x13 => command(opcode, true, false, true, true, 1, 0, 0, 4, 1),
        0x0b => command(opcode, true, false, true, true, 1, 8, 0, 3, 1),
        0x0c => command(opcode, true, false, true, true, 1, 8, 0, 4, 1),
        0x3b => command(opcode, true, false, true, true, 2, 8, 0, 3, 1),
        0x3c => command(opcode, true, false, true, true, 2, 8, 0, 4, 1),
        0xbb => command(opcode, true, false, true, true, 2, 4, 0, 3, 2),
        0xbc => command(opcode, true, false, true, true, 2, 4, 0, 4, 2),
        0x6b => command(opcode, true, false, true, true, 3, 8, 0, 3, 1),
        0x6c => command(opcode, true, false, true, true, 3, 8, 0, 4, 1),
        0xeb => command(opcode, true, false, true, true, 3, 6, 0, 3, 3),
        0xec => command(opcode, true, false, true, true, 3, 6, 0, 4, 3),
        0x02 => command(opcode, true, true, false, true, 1, 0, 1, 3, 1),
        0x12 => command(opcode, true, true, false, true, 1, 0, 1, 4, 1),
        0x32 => command(opcode, true, true, false, true, 3, 0, 1, 3, 1),
        0x34 => command(opcode, true, true, false, true, 3, 0, 1, 4, 1),
        0x20 => command(opcode, true, true, false, true, 0, 0, 1, 3, 1),
        0x21 => command(opcode, true, true, false, true, 0, 0, 1, 4, 1),
        0xd8 => command(opcode, true, true, false, true, 0, 0, 5, 3, 1),
        0xdc => command(opcode, true, true, false, true, 0, 0, 5, 4, 1),
        0x06 | 0x04 | 0x50 | 0x66 | 0x99 => {
            command(opcode, true, false, false, false, 0, 0, 0, 0, 0)
        }
        0x05 | 0x35 | 0x15 | 0x70 | 0x9f => {
            command(opcode, true, false, true, false, 1, 0, 0, 0, 0)
        }
        0x01 | 0x31 => command(opcode, true, true, false, false, 1, 0, 0, 0, 0),
        0x5a => command(opcode, true, false, true, false, 1, 8, 0, 3, 1),
        0xb7 | 0xe9 => command(opcode, false, false, false, false, 0, 0, 0, 0, 0),
        0xc5 => command(opcode, false, true, false, false, 1, 0, 0, 0, 0),
        _ => return None,
    };
    Some(entry)
}

pub const VALID: u32 = 1 << 30;
pub const VALID_ONCE: u32 = 1 << 31;
pub const LOCKED: u32 = 1 << 23;

#[must_use]
pub const fn table_value(opcode: u8, valid_once: bool) -> Option<u32> {
    match descriptor(opcode) {
        Some(entry) => Some(entry.encode() | if valid_once { VALID_ONCE } else { VALID }),
        None => None,
    }
}

#[must_use]
pub const fn fixed_slot(opcode: u8) -> Option<usize> {
    match opcode {
        0xb7 => Some(0),
        0xe9 => Some(1),
        0xc5 => Some(31),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{fixed_slot, table_value};

    #[test]
    fn fast_read_4b_matches_zephyr_encoding() {
        assert_eq!(table_value(0x0c, false), Some(0x6d08_110c));
    }

    #[test]
    fn reserved_commands_have_fixed_slots() {
        assert_eq!(fixed_slot(0xb7), Some(0));
        assert_eq!(fixed_slot(0xe9), Some(1));
        assert_eq!(fixed_slot(0xc5), Some(31));
    }
}
