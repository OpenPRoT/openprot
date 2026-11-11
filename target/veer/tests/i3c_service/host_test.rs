// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR I3C service test, written the way caliptra-mcu-sw's
//! own `tests/integration/src/rom/test_i3c_services.rs` drives its I3C ROM
//! tests: boot through `caliptra_mcu_hw_model` with an I3C socket, connect a
//! `BufferedStream`, wait for the firmware to be ready, send a private write,
//! and check the MCU log. Single-threaded: the model is only advanced by
//! `step_until`.
//!
//! The same binary runs on the emulator (x86_64) and on the VCK190, where the
//! model forwards the socket to the FPGA's I3C host controller.
//!
//! The firmware (`client.rs`) verifies the payload itself and exits PASS only
//! if it matched, so a clean `step_until_exit_success` is the verdict. Reading
//! the staged reply back is not covered: `BufferedStream::receive_private_read`
//! requires a PEC that the target driver does not append.

use caliptra_mcu_hw_model::McuHwModel;
use caliptra_mcu_testing_common::i3c_socket::BufferedStream;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Instant;
use test_boot_files::BootFiles;

/// The payload the host writes to the target (see `EXPECTED` in `client.rs`).
const PAYLOAD: [u8; 4] = [0x01, 0x02, 0x03, 0x04];

/// Logged by `client.rs` once it is parked waiting for the write.
const READY: &str = "waiting for private write";

/// Logged by `client.rs` after it has checked the payload.
const RECEIVED: &str = "received expected payload";

const TIMEOUT_SECS: u64 = 120;

/// Step the model until `term` appears in the MCU log. Panics on a fatal error
/// or timeout.
fn wait_for_log(hw: &mut impl McuHwModel, term: &str) {
    hw.output().set_search_term(term);
    let start = Instant::now();
    hw.step_until(|m| {
        m.output().search_matched()
            || m.mci_fw_fatal_error().is_some()
            || start.elapsed().as_secs() > TIMEOUT_SECS
    });
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");
    assert!(
        start.elapsed().as_secs() <= TIMEOUT_SECS,
        "timed out waiting for {term:?} in the MCU log"
    );
}

#[test]
fn i3c_service_host_test() {
    // A free port for the model's I3C socket.
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

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect(addr).expect("failed to connect to the I3C socket");
    let mut stream = BufferedStream::new(stream);

    // Don't write before the client is parked: a frame latched earlier would
    // fail its "nothing received yet" check.
    wait_for_log(&mut hw, READY);

    assert!(
        stream.send_private_write(target_addr, PAYLOAD.to_vec()),
        "I3C socket closed before the write"
    );

    // The firmware checks the bytes; it logs this only on a match.
    wait_for_log(&mut hw, RECEIVED);

    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
    drop(stream);
}
