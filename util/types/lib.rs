// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

mod opcode;
mod power_of_2;

pub use opcode::Opcode;
pub use power_of_2::PowerOf2Usize;

pub trait Blocking {
    fn wait_for_notification(&self);
}
