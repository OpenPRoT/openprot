// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use util_types::Opcode;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub const IPC_OP_FLASH_ERASE_PAGE: Opcode = Opcode::new(*b"FLEP");
pub const IPC_OP_FLASH_PROGRAM: Opcode = Opcode::new(*b"FLWR");
pub const IPC_OP_FLASH_READ: Opcode = Opcode::new(*b"FLRD");
pub const IPC_OP_FLASH_GET_INFO: Opcode = Opcode::new(*b"FLIN");

#[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct FlashInfo {
    pub page_size: usize,
    pub total_size: usize,
}
