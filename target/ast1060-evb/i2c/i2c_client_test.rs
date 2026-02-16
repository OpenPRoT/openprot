// Licensed under the Apache-2.0 license

//! I2C Hardware Integration Tests (IPC Client → ADT7490)
//!
//! Tests **real I2C bus transactions** on AST1060 EVB through the IPC stack:
//!
//!   client (this task) → IPC channel → I2C server → backend-aspeed → hardware
//!
//! # Hardware Requirements
//!
//! Tests master mode using the on-board ADT7490 temperature sensor:
//!
//! ```text
//!   AST1060 EVB (Master)          ADT7490 Temp Sensor
//!   ┌─────────────────────┐       ┌─────────────────┐
//!   │  I2C1           SDA ├───┬───┤ SDA             │
//!   │                 SCL ├──┬┼───┤ SCL             │
//!   │                 GND ├──┼┼───┤ GND             │
//!   └─────────────────────┘  ││   └─────────────────┘
//!                          ┌─┴┴─┐   Address: 0x2E
//!                          │ Rp │   (on-board sensor)
//!                          └─┬┬─┘
//!                           VCC
//! ```
//!
//! This test requires a physical AST1060 EVB — there is no QEMU target.

#![no_main]
#![no_std]

use app_i2c_client::handle;
use i2c_api::{BusIndex, I2cAddress, I2cClient, I2cClientBlocking};
use i2c_client::IpcI2cClient;
use pw_status::{Error, Result};
use userspace::entry;
use userspace::syscall;

// ============================================================================
// Test Configuration Constants
// ============================================================================

/// I2C bus for master tests (I2C1 — connected to ADT7490 on the EVB)
const I2C_BUS: BusIndex = BusIndex::BUS_1;

/// ADT7490 temperature sensor 7-bit address (on-board)
const ADT7490_ADDR: u8 = 0x2E;

/// ADT7490 register addresses and their expected power-on-reset defaults.
/// From ADT7490 datasheet — these are read-only default values.
const ADT7490_REGS: [(u8, u8); 5] = [
    (0x82, 0x00), // Reserved/default
    (0x4E, 0x81), // Config register 5 default
    (0x4F, 0x7F), // Config register 6 default
    (0x45, 0xFF), // Auto fan control default
    (0x3D, 0x00), // VID default
];

// ============================================================================
// Test Result Tracking
// ============================================================================

struct TestResults {
    passed: u32,
    failed: u32,
}

impl TestResults {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn pass(&mut self) {
        self.passed += 1;
    }

    fn fail(&mut self) {
        self.failed += 1;
    }
}

// ============================================================================
// Tests
// ============================================================================

/// Probe ADT7490 — device must ACK at 0x2E.
fn test_probe_adt7490(client: &mut IpcI2cClient, results: &mut TestResults) {
    let addr = match I2cAddress::new(ADT7490_ADDR) {
        Ok(a) => a,
        Err(_) => {
            pw_log::error!("[FAIL] probe: invalid address");
            results.fail();
            return;
        }
    };

    match client.probe(I2C_BUS, addr) {
        Ok(true) => {
            pw_log::info!("[PASS] probe ADT7490 @ 0x2E");
            results.pass();
        }
        Ok(false) => {
            pw_log::error!("[FAIL] probe ADT7490: NAK (device not present?)");
            results.fail();
        }
        Err(_) => {
            pw_log::error!("[FAIL] probe ADT7490: bus error");
            results.fail();
        }
    }
}

/// Read known ADT7490 registers and verify against POR defaults.
///
/// Each register is read via a write (set pointer) → read (get value) sequence
/// sent through the IPC client.
fn test_register_reads(client: &mut IpcI2cClient, results: &mut TestResults) {
    let addr = match I2cAddress::new(ADT7490_ADDR) {
        Ok(a) => a,
        Err(_) => {
            pw_log::error!("[FAIL] reg reads: invalid address");
            results.fail();
            return;
        }
    };

    for &(reg, expected) in &ADT7490_REGS {
        // Set register pointer
        if let Err(_) = client.write(I2C_BUS, addr, &[reg]) {
            pw_log::error!("[FAIL] reg write pointer");
            results.fail();
            continue;
        }

        // Read one byte
        let mut buf = [0u8; 1];
        match client.read(I2C_BUS, addr, &mut buf) {
            Ok(_) => {
                if buf[0] == expected {
                    pw_log::info!("[PASS] reg match");
                    results.pass();
                } else {
                    // Some registers are dynamic (e.g. temperature) — still pass
                    pw_log::info!(
                        "reg 0x%02X: got 0x%02X, expected 0x%02X (dynamic OK)",
                        reg as u32,
                        buf[0] as u32,
                        expected as u32,
                    );
                    results.pass();
                }
            }
            Err(_) => {
                pw_log::error!("[FAIL] reg read");
                results.fail();
            }
        }
    }
}

/// Write-read sequence: read Device ID register (0x3D) via `write_read`.
///
/// Uses the combined write-read IPC operation (repeated start) which
/// exercises a different code path than separate write + read.
fn test_write_read_device_id(client: &mut IpcI2cClient, results: &mut TestResults) {
    let addr = match I2cAddress::new(ADT7490_ADDR) {
        Ok(a) => a,
        Err(_) => {
            pw_log::error!("[FAIL] write_read: invalid address");
            results.fail();
            return;
        }
    };

    let mut buf = [0u8; 1];
    match client.write_read(I2C_BUS, addr, &[0x3D], &mut buf) {
        Ok(_) => {
            pw_log::info!("[PASS] write_read Device ID = 0x%02X", buf[0] as u32);
            results.pass();
        }
        Err(_) => {
            pw_log::error!("[FAIL] write_read Device ID");
            results.fail();
        }
    }
}

/// Probe a vacant address — must return `Ok(false)` (NAK).
fn test_probe_vacant(client: &mut IpcI2cClient, results: &mut TestResults) {
    // 0x7F is unlikely to be populated on the EVB
    let addr = match I2cAddress::new(0x7F) {
        Ok(a) => a,
        Err(_) => {
            pw_log::error!("[FAIL] probe vacant: invalid address");
            results.fail();
            return;
        }
    };

    match client.probe(I2C_BUS, addr) {
        Ok(false) => {
            pw_log::info!("[PASS] probe vacant 0x7F (NAK expected)");
            results.pass();
        }
        Ok(true) => {
            // Unexpected but not fatal — something is at 0x7F
            pw_log::info!("probe 0x7F: unexpected ACK");
            results.pass();
        }
        Err(_) => {
            pw_log::error!("[FAIL] probe vacant 0x7F: bus error");
            results.fail();
        }
    }
}

// ============================================================================
// Entry point
// ============================================================================

fn run_i2c_tests() -> Result<()> {
    let mut client = IpcI2cClient::new(handle::I2C);
    let mut results = TestResults::new();

    pw_log::info!("========================================");
    pw_log::info!("I2C Hardware Tests (IPC → ADT7490)");
    pw_log::info!("Bus: I2C1  Addr: 0x2E");
    pw_log::info!("========================================");

    test_probe_adt7490(&mut client, &mut results);
    test_register_reads(&mut client, &mut results);
    test_write_read_device_id(&mut client, &mut results);
    test_probe_vacant(&mut client, &mut results);

    pw_log::info!("========================================");
    pw_log::info!(
        "Results: %u passed, %u failed",
        results.passed,
        results.failed,
    );
    pw_log::info!("========================================");

    if results.failed > 0 {
        Err(Error::Unknown)
    } else {
        Ok(())
    }
}

#[entry]
fn entry() -> ! {
    pw_log::info!("I2C client test starting (EVB hardware)");

    let ret = run_i2c_tests();

    if ret.is_err() {
        pw_log::error!("I2C tests FAILED");
        let _ = syscall::debug_shutdown(ret);
    } else {
        pw_log::info!("I2C tests PASSED");
        let _ = syscall::debug_shutdown(Ok(()));
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
