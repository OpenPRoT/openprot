// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave RX test — master side (device A)
//!
//! Writes EXPECTED_PAYLOAD to SLAVE_ADDR on I2C2 (Bus 2) once, then emits
//! TEST_RESULT:PASS. Device B must be running its slave image before this
//! image is loaded on device A.

#![no_std]
#![no_main]

use ast10x0_board::Pins;
use ast10x0_peripherals::i2c::{ClockConfig, I2cConfig, I2cSpeed, I2cXferMode};
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const SLAVE_ADDR: u8 = 0x42;
const EXPECTED_PAYLOAD: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];

fn i2c2_config() -> I2cConfig {
    I2cConfig {
        xfer_mode: I2cXferMode::BufferMode,
        speed: I2cSpeed::Standard,
        multi_master: false,
        smbus_timeout: false,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    }
}

fn run_master() -> Result<(), &'static str> {
    // SAFETY: single call at boot with exclusive SoC access; routes Bus 2's pins and brings up I2C.
    let pins = unsafe { Pins::take() };
    let mut master = pins
        .i2c2
        .open(&i2c2_config())
        .map_err(|_| "I2C2 master init failed")?;

    master
        .write(SLAVE_ADDR, EXPECTED_PAYLOAD)
        .map_err(|_| "master write failed — slave not responding (check J15 and slave firmware)")?;

    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C Slave RX Master";

    fn main() -> ! {
        let sentinel: &[u8] = match run_master() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(e) => {
                pw_log::error!("Master failed: {}", e as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
