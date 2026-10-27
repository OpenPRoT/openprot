// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! DMA-guard abort test — stretcher slave (device B).
//!
//! Provides the two bus states the master (device A) needs:
//!
//! 1. **Serviceable:** serves exactly one slave RX transaction so the master's
//!    phase-1 DMA write completes with `Ok` (the guard-commit path). This uses
//!    the same buffer-mode slave recipe as `i2c_slave_rx`, which is known to work
//!    on the bench rig.
//! 2. **Wedged:** after that one transaction it stops polling / re-arming. The
//!    next master write matches this address, and with no armed RX buffer command
//!    the slave holds SCL low (clock stretch) until firmware re-arms — which it
//!    never does. That sustained stretch makes the master's DMA `wait_completion`
//!    time out, the trigger for the `ArmedDma` teardown under test.
//!
//! If a future rig shows the wedge NAKs instead of stretching (master would
//! report `NoAcknowledge`, not `Timeout`), the fallback is a fixture GPIO holding
//! SCL low — see this test's README.

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor, I2cBusCfg};
use ast10x0_peripherals::i2c::{ClockConfig, I2cConfig, I2cError, I2cSpeed, I2cXferMode};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use openprot_hal_blocking::i2c_hardware::slave::{I2cSlaveBuffer, I2cSlaveCore};
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const SLAVE_ADDR: u8 = 0x42;

/// Bus 2 config: standard-speed buffer mode (matches the working i2c_slave_rx).
const SLAVE_CFG: I2cConfig = I2cConfig {
    speed: I2cSpeed::Standard,
    xfer_mode: I2cXferMode::BufferMode,
    multi_master: false,
    smbus_timeout: false,
    smbus_alert: false,
    clock_config: ClockConfig::ast1060_default(),
};

fn i2c_error_str(error: I2cError) -> &'static str {
    match error {
        I2cError::Overrun => "Overrun",
        I2cError::NoAcknowledge => "NoAcknowledge",
        I2cError::Timeout => "Timeout",
        I2cError::BusRecoveryFailed => "BusRecoveryFailed",
        I2cError::Bus => "Bus",
        I2cError::Busy => "Busy",
        I2cError::Invalid => "Invalid",
        I2cError::Abnormal => "Abnormal",
        I2cError::ArbitrationLoss => "ArbitrationLoss",
        I2cError::SlaveError => "SlaveError",
        I2cError::InvalidAddress => "InvalidAddress",
    }
}

/// Bring up the Bus 2 slave and serve exactly one RX transaction. Returns the
/// live driver so the caller can hold it (keeping slave mode armed) while it
/// stops polling — that non-servicing state is what stretches SCL for phase 2.
fn setup_and_serve_one() -> Result<i2c_backend::BusDriver, &'static str> {
    pw_log::info!("=== AST10x0 I2C DMA-guard abort stretcher (Bus 2, addr 0x42) ===");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C2],
        i2c_buses: &[I2cBusCfg {
            bus: 2,
            config: SLAVE_CFG,
        }],
    });
    // SAFETY: single call at boot with exclusive access to the board.
    unsafe { board.init() }.map_err(|_| "board init failed")?;

    // SAFETY: board.init() ran init_bus(2); we are the sole owner of Bus 2.
    let mut driver = unsafe { i2c_backend::open_bus(2, &SLAVE_CFG) }.map_err(|e| {
        pw_log::error!("open_bus failed: {}", i2c_error_str(e) as &str);
        "open_bus failed"
    })?;

    driver
        .configure_slave_address(SLAVE_ADDR)
        .map_err(|_| "configure_slave_address failed")?;
    driver
        .enable_slave_mode()
        .map_err(|_| "enable_slave_mode failed")?;

    pw_log::info!("stretcher ready — serving one transaction, then wedging");

    // Serve exactly ONE transaction (drives the master's phase 1 to Ok).
    //
    // We consume the packet-done interrupt via `poll_slave_data` but deliberately
    // do NOT call `read_slave_buffer`: draining re-arms the RX buffer command
    // (slave.rs `slave_read`), which would let the master's phase-2 write complete
    // cleanly. The `SLAVE_MATCH | RX_DONE | STOP` packet-done branch that this
    // transaction hits does not re-arm on its own, so once we stop here the next
    // master write finds no armed RX command and the slave stretches SCL — the
    // phase-2 stall the guard-teardown test needs.
    loop {
        match driver.poll_slave_data() {
            Ok(Some(_n)) => break,
            Ok(None) => core::hint::spin_loop(),
            Err(e) => {
                pw_log::error!("poll_slave_data error: {}", i2c_error_str(e) as &str);
                return Err("poll_slave_data failed");
            }
        }
    }

    pw_log::info!("stretcher: served one txn (RX left un-rearmed); now wedged");
    Ok(driver)
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C DMA Abort Stretcher";

    fn main() -> ! {
        match setup_and_serve_one() {
            Ok(driver) => {
                let _ = console_backend_write_all(b"TEST_RESULT:PASS\n");
                // Hold the driver so slave mode stays configured, and STOP
                // polling: the next master write matches but finds no re-armed
                // RX command, so the slave stretches SCL (phase 2 stall) until
                // A gives up.
                let _held = driver;
                loop {
                    core::hint::spin_loop();
                }
            }
            Err(e) => {
                pw_log::error!("stretcher setup failed: {}", e as &str);
                let _ = console_backend_write_all(b"TEST_RESULT:FAIL\n");
                #[expect(clippy::empty_loop)]
                loop {}
            }
        }
    }
}

declare_target!(Target);
