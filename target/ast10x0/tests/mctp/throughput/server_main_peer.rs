// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Combined I2C + MCTP server for the throughput test (requester, card B).

#![no_main]
#![no_std]

use app_mctp_i2c_server_peer::{handle, signals};

const OWN_EID: u8 = 9;
const OWN_I2C_ADDR: u8 = 0x42;
const REMOTE_I2C_ADDR: u8 = 0x10;

include!("server_common.rs");
