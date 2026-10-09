// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave RX burst test app.
//!
//! Arms the AST1060 as an I2C slave at address 0x42 on Bus 2 via the IPC
//! server, then collects BURST_LEN sequence-tagged frames the master sends
//! back-to-back and asserts every sequence number arrives exactly once, in
//! order.
//!
//! The server latches one received frame per bus. A frame arriving before this
//! task is scheduled to drain the previous one overwrites it, so a burst shows
//! up here as a gap in the sequence numbers or as a wait that times out with
//! fewer than BURST_LEN frames collected.

#![no_main]
#![no_std]

use app_i2c_slave_burst::handle;
use i2c_client::I2cClient;
use i2c_client_ipc::IpcTransport;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::{Clock, Duration, Instant, SystemClock};

/// Slave address the test listens on.
const SLAVE_ADDR: u8 = 0x42;

/// Number of frames the master sends. Must match `BURST_LEN` in `master_target.rs`.
const BURST_LEN: u8 = 8;

/// `SLAVE_PKT_SAVE_ADDR` prepends the destination address byte at offset 0, so a
/// 4-byte master payload lands here as 5 bytes.
const FRAME_LEN: usize = 5;

/// How long to wait for each frame after the first. Long enough that a slow bus
/// is not mistaken for a drop, short enough that a drop fails instead of hangs.
const FRAME_TIMEOUT_MS: u64 = 1000;

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
        "SLAVE READY addr=0x{:02x} — expecting {} frames",
        SLAVE_ADDR as u32,
        BURST_LEN as u32,
    );

    let mut rx = [0u8; 32];
    let mut received: u8 = 0;

    while received < BURST_LEN {
        // The first frame waits indefinitely: the master board may not have
        // booted yet. Later frames are part of the burst and must arrive promptly.
        let deadline = if received == 0 {
            Instant::MAX
        } else {
            SystemClock::now()
                .checked_add_duration(Duration::from_millis(FRAME_TIMEOUT_MS))
                .unwrap_or(Instant::MAX)
        };

        if syscall::object_wait(handle::I2C, Signals::USER, deadline).is_err() {
            pw_log::error!(
                "timed out after {} of {} frames — the rest were dropped",
                received as u32,
                BURST_LEN as u32,
            );
            let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
            loop {}
        }

        // TEMP DIAG: sentinel so stale bytes are distinguishable from fresh ones.
        rx.fill(0xCC);

        let event = match client.slave_receive(&mut rx) {
            Ok(event) => event,
            Err(_) => fail!("slave_receive failed"),
        };

        // TEMP DIAG: is a short frame the payload without its address byte, or a
        // genuine truncation?
        pw_log::info!(
            "DIAG frame {} len={} bytes={:02x} {:02x} {:02x} {:02x} {:02x}",
            received as u32,
            event.data_len as u32,
            rx[0] as u32,
            rx[1] as u32,
            rx[2] as u32,
            rx[3] as u32,
            rx[4] as u32,
        );

        if event.data_len != FRAME_LEN {
            pw_log::error!(
                "frame {} length mismatch: got {} expected {}",
                received as u32,
                event.data_len as u32,
                FRAME_LEN as u32,
            );
            let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
            loop {}
        }

        let seq = rx[1];
        if seq != received {
            pw_log::error!(
                "sequence gap: expected frame {} but got frame {} — {} frame(s) dropped",
                received as u32,
                seq as u32,
                seq.wrapping_sub(received) as u32,
            );
            let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
            loop {}
        }

        received += 1;
    }

    pw_log::info!("I2C slave RX burst test PASSED — {} frames", received as u32);
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
