// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Common utility types.

#![no_std]

mod opcode;
mod power_of_2;
mod time;

pub use opcode::Opcode;
pub use power_of_2::PowerOf2Usize;
pub use time::MultiplyDuration;
pub use time::Nanoseconds;
