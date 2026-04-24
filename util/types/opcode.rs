// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable)]
pub struct Opcode(u32);

impl Opcode {
    pub const fn new(val: [u8; 4]) -> Self {
        Opcode(u32::from_le_bytes(val))
    }
}
