// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # MCTP over I2C Transport Binding
//!
//! This crate provides the I2C transport binding for the MCTP server.
//!
//! It implements [`mctp_lib::Sender`] for outbound MCTP-over-I2C packets
//! and provides [`MctpI2cReceiver`] for decoding inbound I2C target frames
//! into MCTP packets.
//!
//! ## Current I2C seam
//!
//! - Outbound path is built on the `embedded_hal::i2c::I2c` contract.
//! - Inbound target-mode data comes from the i2c userspace driver
//!   notification + `SlaveReceive` flow.
//! - MCTP framing/PEC logic stays in `mctp_lib::i2c::MctpI2cEncap`.

#![no_std]
#![warn(missing_docs)]

mod receiver;
mod sender;

pub use receiver::MctpI2cReceiver;
pub use sender::I2cSender;
