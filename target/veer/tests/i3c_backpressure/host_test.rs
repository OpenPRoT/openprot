// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-side harness for the VeeR I3C outbound back-pressure test.
//!
//! Sends one trigger private write, then reads three staged frames back. The
//! firmware stages them with `send`, which blocks on the server's back-pressure
//! until each is read — so the frames arrive strictly in order (A, B, C), one
//! per private read. Receiving them in that order, and the firmware exiting 0,
//! is the proof the back-pressure engages.

use i3c_host::{
    connect_i3c_socket, read_outgoing_packet, send_private_read_on_stream,
    send_private_write_on_stream, Runner,
};
use std::thread;
use std::time::Duration;

const TARGET_ADDR: u8 = 0x08;
const TRIGGER: [u8; 1] = [0x01];
/// Completion write: tells the client we have read all three frames, so it may
/// exit. This is the rendezvous that avoids racing the client's exit against our
/// read of the last frame.
const DONE: [u8; 1] = [0xee];
const FRAMES: [[u8; 4]; 3] = [
    [0xa1, 0xa2, 0xa3, 0xa4],
    [0xb1, 0xb2, 0xb3, 0xb4],
    [0xc1, 0xc2, 0xc3, 0xc4],
];

#[test]
fn i3c_backpressure_host_test() {
    let runner = Runner::spawn(
        "target/veer/tests/i3c_backpressure/i3c_backpressure_runner.sh",
        "i3c backpressure: waiting for trigger",
    );
    assert!(
        runner.wait_ready(Duration::from_secs(600)),
        "runner exited or timed out before firmware readiness"
    );

    let mut stream =
        connect_i3c_socket(Duration::from_secs(5)).expect("failed to connect to I3C socket");

    // Fire the trigger until the firmware acts on it (the readiness log races
    // slightly ahead of the IRQ being armed).
    let mut triggered = false;
    for _ in 0..50 {
        if send_private_write_on_stream(&mut stream, TARGET_ADDR, &TRIGGER).is_ok() {
            triggered = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(triggered, "failed to send trigger");

    // Read the three staged frames. Each read releases the next (back-pressure),
    // so they must arrive in order. Poll, since a frame may not be staged the
    // instant we read.
    let mut got: Vec<Vec<u8>> = Vec::new();
    let mut attempts = 0u32;
    while got.len() < FRAMES.len() && attempts < 400 {
        attempts += 1;
        let want = &FRAMES[got.len()];
        if send_private_read_on_stream(&mut stream, TARGET_ADDR).is_ok()
            && let Ok(packet) = read_outgoing_packet(&mut stream)
            && packet.data == want
        {
            got.push(packet.data);
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert_eq!(
        got.len(),
        FRAMES.len(),
        "did not receive all three frames in order"
    );

    // Rendezvous: all three read, so release the client to exit cleanly.
    let _ = send_private_write_on_stream(&mut stream, TARGET_ADDR, &DONE);

    let status = runner.wait();
    assert!(
        status.success(),
        "runner exited with status: {status} after all three frames were read"
    );
}
