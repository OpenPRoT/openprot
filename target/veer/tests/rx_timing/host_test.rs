// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the RX drain timing test. Boots the model with an I3C socket, and
//! for each frame waits until the firmware logs that it is ready, then sends one
//! private write. The firmware (`target.rs`) times draining each frame and logs
//! the result; this test checks nothing about the numbers, only that all frames
//! were received and the firmware exited PASS. Run it with
//! `--test_output=streamed` to see them.

use caliptra_mcu_hw_model::McuHwModel;
use caliptra_mcu_testing_common::i3c_socket::BufferedStream;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Instant;
use test_boot_files::BootFiles;

/// Frames to send: the firmware's `FRAMES`. The firmware drains even-numbered
/// frames with the driver and odd-numbered ones with raw register reads, so each
/// size is sent twice in a row, then the next.
const FRAMES: usize = 18;

/// Payload sizes. The host's PEC adds a byte, and the core's RX data queue holds
/// 64 words (256 bytes), so the largest fits one frame.
const SIZES: [usize; 3] = [4, 64, 252];

const TIMEOUT_SECS: u64 = 120;

/// Step the model until `term` appears in the MCU log, accumulated in `log`.
fn wait_for_log(hw: &mut impl McuHwModel, log: &mut String, term: &str) {
    let start = Instant::now();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        if !out.is_empty() {
            print!("{out}");
            log.push_str(&out);
        }
        log.contains(term)
            || m.mci_fw_fatal_error().is_some()
            || start.elapsed().as_secs() > TIMEOUT_SECS
    });
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");
    assert!(log.contains(term), "timed out waiting for {term:?}");
}

#[test]
fn rx_timing_host_test() {
    let i3c_port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let files = BootFiles::from_env();
    let mut params = files.init_params();
    params.i3c_port = Some(i3c_port);
    let mut hw = caliptra_mcu_hw_model::new(params).expect("hardware model failed to boot");

    hw.start_i3c_controller();

    let port = hw.i3c_port().unwrap();
    let target_addr = hw.i3c_address().unwrap();
    let stream = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port)))
        .expect("failed to connect to the I3C socket");
    let mut stream = BufferedStream::new(stream);

    let mut log = String::new();
    for i in 0..FRAMES {
        // One frame at a time: a second frame that arrives before the firmware
        // has drained the first would fill the RX queue and be cut short.
        wait_for_log(&mut hw, &mut log, &format!("rx_timing: ready #{i};"));
        let size = SIZES[(i / 2) % SIZES.len()];
        let payload: Vec<u8> = (0..size).map(|b| (b + i) as u8).collect();
        assert!(
            stream.send_private_write(target_addr, payload),
            "I3C socket closed before frame {i}"
        );
    }

    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
    drop(stream);
}
