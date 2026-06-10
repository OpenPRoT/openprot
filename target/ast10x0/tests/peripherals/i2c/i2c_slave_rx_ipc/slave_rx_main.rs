// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave RX test app.
//!
//! Arms the AST1060 as an I2C slave at address 0x42 on Bus 2 via the IPC
//! server, then waits for `Signals::USER` (raised by the server when the
//! hardware IRQ fires and the RX latch is filled). Drains the buffer via
//! `slave_receive` and asserts the payload matches `EXPECTED_PAYLOAD`.

#![no_main]
#![no_std]

use app_i2c_slave_rx::handle;
use i2c_client::I2cClient;
use i2c_client_ipc::IpcTransport;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// Slave address the test listens on.
const SLAVE_ADDR: u8 = 0x42;

/// Expected buffer contents: SLAVE_PKT_SAVE_ADDR prepends dest address byte
/// (SLAVE_ADDR << 1) at offset 0, followed by the master's payload [0xDE, 0xAD, 0xBE, 0xEF].
const EXPECTED_PAYLOAD: &[u8] = &[SLAVE_ADDR << 1, 0xDE, 0xAD, 0xBE, 0xEF];

macro_rules! fail {
    ($msg:literal) => {{
        pw_log::error!($msg);
        let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        loop {}
    }};
}

#[entry]
fn entry() {
    let mut client = I2cClient::new(IpcTransport::new(handle::I2C));

    // Configure slave address, enable slave mode, arm IRQ-driven notification.
    if client.configure_slave(SLAVE_ADDR).is_err() {
        fail!("configure_slave failed");
    }
    if client.enable_slave().is_err() {
        fail!("enable_slave failed");
    }
    if client.enable_notification().is_err() {
        fail!("enable_notification failed");
    }

    pw_log::info!(
        "SLAVE READY addr=0x{:02x} — start external master now",
        SLAVE_ADDR as u32,
    );

    // Block until the server raises Signals::USER on our channel (means the
    // hardware IRQ fired and the RX latch is filled).
    if syscall::object_wait(handle::I2C, Signals::USER, Instant::MAX).is_err() {
        fail!("object_wait for USER signal failed");
    }

    // Drain the latched bytes from the server.
    let mut rx = [0u8; 32];
    let event = match client.slave_receive(&mut rx) {
        Ok(event) => event,
        Err(_) => fail!("slave_receive failed"),
    };

    pw_log::info!("Received {} byte(s)", event.data_len as u32);

    // Assert length and payload content.
    if event.data_len != EXPECTED_PAYLOAD.len() {
        pw_log::error!(
            "length mismatch: got {} expected {}",
            event.data_len as u32,
            EXPECTED_PAYLOAD.len() as u32,
        );
        let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
        loop {}
    }

    if &rx[..event.data_len] != EXPECTED_PAYLOAD {
        pw_log::error!(
            "payload mismatch: got [{:02x} {:02x} {:02x} {:02x} {:02x}]",
            rx[0] as u32,
            rx[1] as u32,
            rx[2] as u32,
            rx[3] as u32,
            rx[4] as u32,
        );
        let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
        loop {}
    }

    pw_log::info!("I2C slave RX IPC test PASSED");
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
