// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

pub mod protocol;
pub mod seam;
pub mod transport;

#[doc(inline)]
pub use protocol::{
    I2cError, I2cOp, I2cOpDesc, I2cOpKind, I2cRequestHeader, I2cResponseHeader, SlaveEvent,
    MAX_OPS, MAX_PAYLOAD_SIZE,
};
pub use seam::I2cSlaveEvent;
pub use transport::{Transport, TransportError};
