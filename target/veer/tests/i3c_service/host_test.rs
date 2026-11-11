// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-side harness for the VeeR I3C service test.
//!
//! Drives both directions of the transport protocol:
//!
//! - a private write carrying [`PAYLOAD`], which the server latches from its
//!   IRQ handler and the client collects with `I3cOp::Recv`; then
//! - a private read, which collects the [`REPLY`] the client staged with
//!   `I3cOp::Send`.
//!
//! The firmware exits 0 only after walking its whole sequence (see
//! `client.rs`), so a clean exit plus the expected reply bytes covers every
//! `dispatch()` arm.

use i3c_host::{
    connect_i3c_socket, read_outgoing_packet, send_private_read_on_stream,
    send_private_write_on_stream, Runner,
};
use std::thread;
use std::time::Duration;

/// The payload the host writes to the target.
const PAYLOAD: [u8; 4] = [0x01, 0x02, 0x03, 0x04];

/// The payload the firmware stages for us to read back.
const REPLY: [u8; 4] = [0xa5, 0x5a, 0xc3, 0x3c];

#[test]
fn i3c_service_host_test() {
    let runner = Runner::spawn(
        "target/veer/tests/i3c_service/i3c_service_runner.sh",
        "waiting for private write",
    );
    assert!(
        runner.wait_ready(Duration::from_secs(600)),
        "runner exited or timed out before firmware readiness"
    );

    let addr = runner.target_addr();
    let mut stream =
        connect_i3c_socket(Duration::from_secs(5)).expect("failed to connect to I3C socket");

    // Retry the write until the firmware acts on it: the readiness log races
    // slightly ahead of the IRQ actually being armed.
    let mut attempts = 0u32;
    while !runner.exited() && attempts < 200 {
        attempts += 1;
        let _ = send_private_write_on_stream(&mut stream, addr, &PAYLOAD);
        thread::sleep(Duration::from_millis(100));

        // Once the client has staged its reply, collect it with a private read.
        // The IBI raised by `Send` is what tells a real controller to read; here
        // we simply poll for it alongside the write retry.
        if send_private_read_on_stream(&mut stream, addr).is_ok()
            && let Ok(packet) = read_outgoing_packet(&mut stream)
            && packet.data == REPLY
        {
            break;
        }
    }

    let status = runner.wait();
    assert!(
        status.success(),
        "runner exited with status: {status} after {attempts} write attempts"
    );
}
