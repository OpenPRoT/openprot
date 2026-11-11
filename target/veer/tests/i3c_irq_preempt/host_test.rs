// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-side harness for the VeeR I3C userspace-IRQ test.
//!
//! Drives a single private write with the expected payload and asserts the
//! firmware exits 0 — which it only does after receiving the frame through the
//! userspace interrupt path. The write is retried until the firmware exits to
//! cover the small window between the readiness log and the IRQ being armed.

use i3c_host::{connect_i3c_socket, send_private_write_on_stream, Runner};
use std::thread;
use std::time::Duration;

const PAYLOAD: [u8; 4] = [0x01, 0x02, 0x03, 0x04];

#[test]
fn i3c_irq_preempt_host_test() {
    let runner = Runner::spawn(
        "target/veer/tests/i3c_irq_preempt/i3c_irq_preempt_runner.sh",
        "waiting for private write",
    );
    assert!(
        runner.wait_ready(Duration::from_secs(600)),
        "runner exited or timed out before firmware readiness"
    );

    let addr = runner.target_addr();
    let mut stream =
        connect_i3c_socket(Duration::from_secs(5)).expect("failed to connect to I3C socket");

    let mut attempts = 0u32;
    while !runner.exited() && attempts < 200 {
        attempts += 1;
        let _ = send_private_write_on_stream(&mut stream, addr, &PAYLOAD);
        thread::sleep(Duration::from_millis(100));
    }

    let status = runner.wait();
    assert!(
        status.success(),
        "runner exited with status: {status} after {attempts} write attempts"
    );
}
