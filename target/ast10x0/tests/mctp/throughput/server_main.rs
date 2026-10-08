// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Combined I2C + MCTP server for the throughput test (responder, card A).

#![no_main]
#![no_std]

use app_mctp_i2c_server::{handle, signals};

const OWN_EID: u8 = 8;
const OWN_I2C_ADDR: u8 = 0x10;
const REMOTE_I2C_ADDR: u8 = 0x42;

include!("server_common.rs");
