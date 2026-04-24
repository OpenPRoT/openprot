// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! IPC opcode definition.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// A 32-bit IPC opcode.
///
/// Opcodes are typically represented as 4-character ASCII strings.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Opcode(u32);

impl Opcode {
    /// Creates a new `Opcode` from a 4-byte array.
    pub const fn new(val: [u8; 4]) -> Self {
        Opcode(u32::from_le_bytes(val))
    }
}

impl From<Opcode> for u32 {
    fn from(op: Opcode) -> Self {
        op.0
    }
}
