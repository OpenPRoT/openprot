// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave interrupt test — slave side (device B)
//!
//! Runs on the AST1060 Test Harness board with J15 pins 1 and 2 connected.
//! This binary is the SLAVE. Load it on device B before loading the master
//! image on device A.
//!
//! The slave listens at address 0x42, handles three transactions initiated
//! by the master, verifies the expected interrupt events, then reports PASS.

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i2c::{
    Ast1060I2c, ClockConfig, I2cConfig, I2cSpeed, I2cXferMode, SlaveConfig, SlaveEvent,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const SLAVE_ADDR: u8 = 0x42;
const EXPECTED_WRITE: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD];
const READ_RESPONSE: &[u8] = &[0x55];

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

/// Poll handle_slave_interrupt until an event arrives or the budget runs out.
fn wait_event<Y: FnMut(u32)>(slave: &mut Ast1060I2c<'_, Y>, max_polls: u32) -> Option<SlaveEvent> {
    for _ in 0..max_polls {
        if let Some(ev) = slave.handle_slave_interrupt() {
            return Some(ev);
        }
        core::hint::spin_loop();
    }
    None
}

fn run_slave() -> Result<(), &'static str> {
    pw_log::info!("=== I2C slave IRQ test: SLAVE (device B) ===");
    pw_log::info!(
        "Listening at addr 0x{:02x}. Start master (device A) now.",
        SLAVE_ADDR as u32
    );

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C2],
    });
    // SAFETY: single call at boot with exclusive access to SCU/I2C global regs.
    unsafe { board.init() };

    // SAFETY: I2C2 registers accessed only through `slave` for this test.
    let mut slave = unsafe {
        Ast1060I2c::new(
            ast1060_pac::I2c2::ptr(),
            ast1060_pac::I2cbuff2::ptr(),
            &i2c2_config(),
            |_| core::hint::spin_loop(),
        )
    }
    .map_err(|_| "slave I2C2 init failed")?;

    let slave_cfg = SlaveConfig::new(SLAVE_ADDR).map_err(|_| "SlaveConfig::new failed")?;

    // ------------------------------------------------------------------
    // Test 1: receive master write → expect DataReceived
    // ------------------------------------------------------------------
    slave
        .configure_slave(&slave_cfg)
        .map_err(|_| "test 1: configure_slave failed")?;

    match wait_event(&mut slave, 50_000_000) {
        Some(SlaveEvent::DataReceived { len }) => {
            if len != EXPECTED_WRITE.len() {
                pw_log::error!(
                    "test 1: DataReceived len={} expected={}",
                    len as u32,
                    EXPECTED_WRITE.len() as u32
                );
                return Err("test 1: DataReceived len mismatch");
            }
            let mut buf = [0u8; EXPECTED_WRITE.len()];
            slave
                .slave_read(&mut buf)
                .map_err(|_| "test 1: slave_read failed")?;
            if buf != *EXPECTED_WRITE {
                return Err("test 1: DataReceived payload mismatch");
            }
            pw_log::info!("Test 1 passed: DataReceived len={}", len as u32);
        }
        Some(_) => return Err("test 1: unexpected slave event"),
        None => return Err("test 1: timed out waiting for DataReceived"),
    }

    // ------------------------------------------------------------------
    // Test 2: respond to master read → pre-arm TX, expect DataSent
    // ------------------------------------------------------------------
    slave
        .configure_slave(&slave_cfg)
        .map_err(|_| "test 2: configure_slave failed")?;

    slave
        .slave_write(READ_RESPONSE)
        .map_err(|_| "test 2: slave_write failed")?;

    match wait_event(&mut slave, 50_000_000) {
        Some(SlaveEvent::DataSent { len }) => {
            pw_log::info!("Test 2 passed: DataSent len={}", len as u32);
        }
        Some(SlaveEvent::Stop) => {
            pw_log::info!("Test 2 passed: Stop (short read path)");
        }
        Some(_) => return Err("test 2: unexpected slave event"),
        None => return Err("test 2: timed out waiting for DataSent"),
    }

    // ------------------------------------------------------------------
    // Test 3: single-byte write from master → DataReceived then Stop
    // ------------------------------------------------------------------
    slave
        .configure_slave(&slave_cfg)
        .map_err(|_| "test 3: configure_slave failed")?;

    match wait_event(&mut slave, 50_000_000) {
        Some(SlaveEvent::DataReceived { len: _ })
        | Some(SlaveEvent::Stop)
        | Some(SlaveEvent::WriteRequest) => {
            pw_log::info!("Test 3 passed: data/stop/write-req observed");
        }
        Some(_) => return Err("test 3: unexpected slave event"),
        None => return Err("test 3: timed out waiting for event"),
    }

    pw_log::info!("=== Slave tests complete ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C Slave IRQ Slave";

    fn main() -> ! {
        let sentinel: &[u8] = match run_slave() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(e) => {
                pw_log::error!("Slave test failed: {}", e as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
