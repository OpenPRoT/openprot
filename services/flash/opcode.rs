// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared flash IPC opcodes and data structures.

#![no_std]

use util_types::Opcode;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// IPC opcode for erasing a flash block.
pub const IPC_OP_FLASH_ERASE: Opcode = Opcode::new(*b"FLET");
/// IPC opcode for programming flash.
pub const IPC_OP_FLASH_PROGRAM: Opcode = Opcode::new(*b"FLWR");
/// IPC opcode for reading from flash.
pub const IPC_OP_FLASH_READ: Opcode = Opcode::new(*b"FLRD");
/// IPC opcode for retrieving flash information.
pub const IPC_OP_FLASH_GET_INFO: Opcode = Opcode::new(*b"FLIN");

/// Information about the flash device.
#[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct FlashInfo {
    /// The size of a single flash page in bytes.
    pub page_size: u32,
    /// The total size of the flash in bytes.
    pub total_size: u32,
    /// A bitmap of supported erase block sizes.
    pub erasable_sizes_bitmap: u32,
}
