// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

pub mod protocol;
pub mod seam;
pub mod transport;

pub use protocol::*;
pub use seam::I2cSlaveEvent;
pub use transport::{Transport, TransportError};
