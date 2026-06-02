// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

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

/// Slave address the test binary listens on.
const SLAVE_ADDR: u8 = 0x42;

/// Payload the master binary sends.
const EXPECTED_PAYLOAD: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];

/// Bus 2 config: standard-speed buffer-mode, no SMBus timeout.
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

fn run_slave_rx_test() -> Result<(), &'static str> {
    pw_log::info!("=== AST10x0 I2C slave RX test (Bus 2, addr 0x42) ===");

    // Phase A — board init: SCU clock/reset + pin-mux + init_bus(2).
    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C2],
        i2c_buses: &[I2cBusCfg {
            bus: 2,
            config: SLAVE_CFG,
        }],
    });
    // SAFETY: Test target runs once at boot with exclusive access to the board.
    unsafe { board.init() }.map_err(|_| "board init failed")?;
    pw_log::info!("Board init complete");

    // Phase B — open the already-initialised bus and configure slave mode.
    // SAFETY: board.init() called init_bus(2); we are the sole owner of Bus 2.
    let mut driver = unsafe { i2c_backend::open_bus(2, &SLAVE_CFG) }.map_err(|e| {
        pw_log::error!("open_bus failed: {}", i2c_error_str(e) as &str);
        "open_bus failed"
    })?;

    driver.configure_slave_address(SLAVE_ADDR).map_err(|e| {
        pw_log::error!(
            "configure_slave_address failed: {}",
            i2c_error_str(e) as &str
        );
        "configure_slave_address failed"
    })?;
    driver.enable_slave_mode().map_err(|e| {
        pw_log::error!("enable_slave_mode failed: {}", i2c_error_str(e) as &str);
        "enable_slave_mode failed"
    })?;

    pw_log::info!(
        "SLAVE READY addr=0x{:02x} — start external master now",
        SLAVE_ADDR as u32
    );

    // Phase C — poll until a write from the external master arrives.
    let rx_len = loop {
        match driver.poll_slave_data() {
            Ok(Some(n)) => break n,
            Ok(None) => core::hint::spin_loop(),
            Err(e) => {
                pw_log::error!("poll_slave_data error: {}", i2c_error_str(e) as &str);
                return Err("poll_slave_data failed");
            }
        }
    };
    pw_log::info!("Received {} byte(s) from master", rx_len as u32);

    // Phase D — drain buffer and assert payload matches.
    let mut rx = [0u8; 32];
    let read_len = rx_len.min(rx.len());
    let n = driver.read_slave_buffer(&mut rx[..read_len]).map_err(|e| {
        pw_log::error!("read_slave_buffer failed: {}", i2c_error_str(e) as &str);
        "read_slave_buffer failed"
    })?;

    if n != EXPECTED_PAYLOAD.len() {
        pw_log::error!(
            "length mismatch: got {} expected {}",
            n as u32,
            EXPECTED_PAYLOAD.len() as u32
        );
        return Err("payload length mismatch");
    }

    let received = &rx[..n];
    if received != EXPECTED_PAYLOAD {
        pw_log::error!(
            "payload mismatch: got [{:02x} {:02x} {:02x} {:02x}] expected [{:02x} {:02x} {:02x} {:02x}]",
            received[0] as u32,
            received[1] as u32,
            received[2] as u32,
            received[3] as u32,
            EXPECTED_PAYLOAD[0] as u32,
            EXPECTED_PAYLOAD[1] as u32,
            EXPECTED_PAYLOAD[2] as u32,
            EXPECTED_PAYLOAD[3] as u32,
        );
        return Err("payload content mismatch");
    }

    // Phase E — verify latch is clear; disable slave mode.
    match driver.poll_slave_data() {
        Ok(None) => {}
        Ok(Some(extra)) => {
            pw_log::error!("latch not empty after drain: {} byte(s)", extra as u32);
            return Err("latch not empty after drain");
        }
        Err(e) => {
            pw_log::error!("poll after drain error: {}", i2c_error_str(e) as &str);
            return Err("poll after drain failed");
        }
    }

    driver.disable_slave_mode().map_err(|e| {
        pw_log::error!("disable_slave_mode failed: {}", i2c_error_str(e) as &str);
        "disable_slave_mode failed"
    })?;

    pw_log::info!("=== AST10x0 I2C slave RX test PASSED ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C Slave RX";

    fn main() -> ! {
        let sentinel: &[u8] = match run_slave_rx_test() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(error) => {
                pw_log::error!("I2C slave RX test failed: {}", error as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };

        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
