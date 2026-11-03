// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave RX burst test — master side (device A)
//!
//! Writes BURST_LEN sequence-tagged frames to SLAVE_ADDR on I2C2 (Bus 2) with
//! no delay between them, then emits TEST_RESULT:PASS.
//!
//! The burst is preceded by a silent delay, not by probing: the harness flashes
//! and boots device A before device B, so the burst would otherwise race the
//! slave's arrival. Probing for an ACK would leave half-captured fragments in
//! the slave's receive latch and mask the defect under test, so nothing touches
//! the bus until the burst itself.
//!
//! Nothing paces the loop: frames go out back-to-back. The slave ACKs each
//! address and then holds SCL low ("slave wait for FW") until its consumer has
//! taken the previous frame, so a write blocks longer rather than failing.

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i2c::{
    Ast1060I2c, Ast1060I2cRegisters, ClockConfig, I2cConfig, I2cSpeed, I2cXferMode,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const SLAVE_ADDR: u8 = 0x42;

/// Number of frames sent back-to-back.
const BURST_LEN: u8 = 8;

/// Spin iterations burned before the burst, waiting for device B to be flashed
/// and boot. Its 500 KB upload alone takes about a minute. The serial log
/// timestamps the message either side of this delay, so the achieved wait is
/// readable from a run and this count can be retuned against it.
const ARM_DELAY_SPINS: u64 = 500_000_000;

/// Attempts per frame before the burst is declared failed. A refused frame means
/// the slave has not re-armed yet, so retrying is the expected path, not an error.
const MAX_RETRIES: u8 = 16;

/// Spin iterations between retries of the same frame.
const RETRY_SPINS: u32 = 1_000;

fn frame(seq: u8) -> [u8; 4] {
    [seq, 0xA5, 0x5A, !seq]
}

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
    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C2],
        i2c_buses: &[],
    });
    // SAFETY: single call at boot with exclusive access to SCU/I2C global regs.
    unsafe { board.init() }.map_err(|_| "board init failed")?;

    // SAFETY: I2C2 registers accessed only through `master` for this test.
    let mmio =
        unsafe { Ast1060I2cRegisters::new(ast1060_pac::I2c2::ptr(), ast1060_pac::I2cbuff2::ptr()) };
    let mut master = Ast1060I2c::new(mmio, &i2c2_config(), |_| core::hint::spin_loop())
        .map_err(|_| "I2C2 master init failed")?;

    pw_log::info!("Master waiting for slave to boot");
    for _ in 0..ARM_DELAY_SPINS {
        core::hint::spin_loop();
    }
    pw_log::info!("Master starting burst");

    for seq in 0..BURST_LEN {
        let mut attempts = 0;
        while master.write(SLAVE_ADDR, &frame(seq)).is_err() {
            attempts += 1;
            if attempts == MAX_RETRIES {
                pw_log::error!("master gave up at seq={}", seq as u32);
                return Err("master write failed mid-burst");
            }
            for _ in 0..RETRY_SPINS {
                core::hint::spin_loop();
            }
        }
    }

    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C Slave RX Burst Master";

    fn main() -> ! {
        let sentinel: &[u8] = match run_master() {
            Ok(()) => {
                pw_log::info!("Master sent {} frames back-to-back", BURST_LEN as u32);
                b"TEST_RESULT:PASS\n"
            }
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
