// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR I3C service test, written the way caliptra-mcu-sw's
//! own `tests/integration/src/rom/test_i3c_services.rs` drives its I3C ROM
//! tests: boot through `caliptra_mcu_hw_model` with an I3C socket, connect a
//! `BufferedStream`, wait for the firmware to be ready, send a private write,
//! and check the MCU log. Single-threaded: the model is only advanced by
//! `step_until`.
//!
//! After the first write the firmware stages a reply and then sends a second
//! one. Whether the server holds that second send (outbound back-pressure)
//! depends on whether the hardware model reads the first reply on its own. The
//! emulator does, so the second send is not held there and the test carries on.
//! The FPGA model leaves the reply queued (the server raises IBIs with a
//! non-MCTP MDB), so on the FPGA the test requires the send to be held, and
//! fails if it is not. When it is held, the test writes again to release it.
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

/// Logged by the server when it holds the client's second send.
const HELD: &str = "i3c: send held";

/// Logged by `client.rs` when its second send has returned.
const RELEASED: &str = "second send released";

/// A second write, to release a held send. Its contents do not matter.
const PAYLOAD2: [u8; 4] = [0x05, 0x06, 0x07, 0x08];

const TIMEOUT_SECS: u64 = 120;

/// Step the model until any of `terms` appears in the MCU log, accumulated in
/// `log`, and return the index of the first term found. Looks at everything
/// logged so far, so a term that arrived in the same burst as an earlier one is
/// still found. Panics on a fatal error or timeout.
fn wait_for_any(hw: &mut impl McuHwModel, log: &mut String, terms: &[&str]) -> usize {
    let found = |log: &str| terms.iter().position(|t| log.contains(t));
    let start = Instant::now();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        if !out.is_empty() {
            print!("{out}");
            log.push_str(&out);
        }
        found(log).is_some()
            || m.mci_fw_fatal_error().is_some()
            || start.elapsed().as_secs() > TIMEOUT_SECS
    });
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");
    found(log).unwrap_or_else(|| panic!("timed out waiting for any of {terms:?} in the MCU log"))
}

/// Like [`wait_for_any`] for a single term.
fn wait_for_log(hw: &mut impl McuHwModel, log: &mut String, term: &str) {
    wait_for_any(hw, log, &[term]);
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

    let mut log = String::new();

    // Don't write before the client is parked: a frame latched earlier would
    // fail its "nothing received yet" check.
    wait_for_log(&mut hw, &mut log, READY);

    assert!(
        stream.send_private_write(target_addr, PAYLOAD.to_vec()),
        "I3C socket closed before the write"
    );

    // The firmware checks the bytes; it logs this only on a match.
    wait_for_log(&mut hw, &mut log, RECEIVED);

    // The client now sends a second reply. If the first is still unread the
    // server holds it and logs so; release it with a second write. Otherwise it
    // returns at once. Held is checked first: a held send is always logged
    // before it is released.
    let on_fpga = std::env::var("CALIPTRA_INTERFACE").as_deref() == Ok("fpga");
    let held = wait_for_any(&mut hw, &mut log, &[HELD, RELEASED]) == 0;
    assert!(
        held || !on_fpga,
        "the second send was not held on the FPGA: the hardware model read the first reply itself"
    );
    if held {
        println!("i3c host: second send was HELD; writing again to release it");
        assert!(
            stream.send_private_write(target_addr, PAYLOAD2.to_vec()),
            "I3C socket closed before the second write"
        );
        wait_for_log(&mut hw, &mut log, RELEASED);
    } else {
        println!("i3c host: second send was NOT held (emulator: first reply already read)");
    }

    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
    drop(stream);
}
