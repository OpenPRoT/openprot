// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave interrupt test — master side (device A)
//!
//! Runs on the AST1060 Test Harness board with J15 pins 1 and 2 connected,
//! which links I2C2 (PAC I2c2, SCU418[0:1]) between device A and device B.
//!
//! This binary is the MASTER. Load the companion slave binary
//! (i2c_slave_irq_slave image) on device B first, then load this image on
//! device A.  Device B must be running and listening before device A starts
//! transmitting.
//!
//! Tests exercised:
//!   1. Master write → slave DataReceived interrupt
//!   2. Master read  → slave DataSent interrupt (slave pre-arms TX)
//!   3. Zero-length write → slave Stop event

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i2c::{Ast1060I2c, ClockConfig, I2cConfig, I2cSpeed, I2cXferMode};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const SLAVE_ADDR: u8 = 0x42;
const WRITE_PAYLOAD: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD];

fn i2c2_config() -> I2cConfig {
    I2cConfig {
        xfer_mode: I2cXferMode::BufferMode,
        speed: I2cSpeed::Fast,
        multi_master: false,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    }
}

fn run_master() -> Result<(), &'static str> {
    pw_log::info!("=== I2C slave IRQ test: MASTER (device A) ===");
    pw_log::info!("J15 must be connected. Load slave image on device B first.");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C2],
    });
    // SAFETY: single call at boot with exclusive access to SCU/I2C global regs.
    unsafe { board.init() };

    // SAFETY: I2C2 registers accessed only through `master` for this test.
    let mut master = unsafe {
        Ast1060I2c::new(
            ast1060_pac::I2c2::ptr(),
            ast1060_pac::I2cbuff2::ptr(),
            &i2c2_config(),
            |_| core::hint::spin_loop(),
        )
    }
    .map_err(|_| "master I2C2 init failed")?;

    // ------------------------------------------------------------------
    // Test 1: master write → slave DataReceived
    // ------------------------------------------------------------------
    pw_log::info!("Test 1: master write");
    master.write(SLAVE_ADDR, WRITE_PAYLOAD).map_err(|_| {
        "test 1: master write failed (slave not responding — check J15 and slave firmware)"
    })?;
    pw_log::info!("Test 1 passed");

    // ------------------------------------------------------------------
    // Test 2: master read → slave DataSent
    // The slave pre-arms its TX buffer before this read arrives.
    // ------------------------------------------------------------------
    pw_log::info!("Test 2: master read");
    let mut rx = [0u8; 1];
    master
        .read(SLAVE_ADDR, &mut rx)
        .map_err(|_| "test 2: master read failed")?;
    let [rx_byte] = rx;
    if rx_byte != 0x55 {
        pw_log::error!("test 2: got 0x{:02x}, expected 0x55", rx_byte as u32);
        return Err("test 2: rx data mismatch");
    }
    pw_log::info!("Test 2 passed: rx=0x{:02x}", rx_byte as u32);

    // ------------------------------------------------------------------
    // Test 3: single-byte write → slave Stop event after packet done
    // ------------------------------------------------------------------
    pw_log::info!("Test 3: single-byte write (triggers slave Stop)");
    master
        .write(SLAVE_ADDR, &[0xFF])
        .map_err(|_| "test 3: write failed")?;
    pw_log::info!("Test 3 passed");

    pw_log::info!("=== Master tests complete ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C Slave IRQ Master";

    fn main() -> ! {
        let sentinel: &[u8] = match run_master() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(e) => {
                pw_log::error!("Master test failed: {}", e as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
