// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR MCTP-over-I3C echo test, written the way
//! caliptra-mcu-sw's own MCTP tests drive their firmware: boot through
//! `caliptra_mcu_hw_model` with an I3C socket, send an MCTP request as a private
//! write, then wait for the target's IBI and read the reply
//! (`BufferedStream::receive_ibi` and `receive_private_read`, which checks and
//! strips the PEC). Single-threaded: the model is only advanced by `step_until`.
//!
//! The request is one MCTP packet (DSP0233: nothing precedes it; the PEC that
//! `send_private_write` appends covers the address byte and the packet). The
//! target's MCTP router hands it to the echo app, which sends the payload back;
//! the MCTP server fragments and frames the reply and the i3c server raises an
//! IBI with the MCTP pending-read MDB. The host checks the reply's MCTP header
//! and payload, then waits for the firmware to exit PASS.
//!
//! The same binary runs on the emulator (x86_64) and on the VCK190, where the
//! model forwards the socket to the FPGA's I3C host controller.

use caliptra_mcu_hw_model::McuHwModel;
use caliptra_mcu_testing_common::i3c_socket::BufferedStream;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Instant;
use test_boot_files::BootFiles;

const TARGET_EID: u8 = 8;
const HOST_EID: u8 = 9;
/// The message type the echo app listens for (`ECHO_MSG_TYPE` in `//services/mctp/echo`).
const ECHO_MSG_TYPE: u8 = 1;

/// The MCTP transport header is 4 bytes; the message-type byte follows it.
const MCTP_HEADER_LEN: usize = 4;

/// Logged by `echo.rs` once its listener is armed.
const READY: &str = "mctp echo: waiting for request";

const TIMEOUT_SECS: u64 = 120;

/// The payload the host sends and expects echoed back.
const PAYLOAD: &[u8] = b"mctp-i3c";

/// The request: transport header (version 1, destination, source, flags
/// 0xC8 = SOM | EOM | tag owner | tag 0), the message type, the payload.
fn request() -> Vec<u8> {
    let mut packet = vec![0x01, TARGET_EID, HOST_EID, 0xC8, ECHO_MSG_TYPE];
    packet.extend_from_slice(PAYLOAD);
    packet
}

/// Step the model until `term` appears in the MCU log, accumulated in `log`.
/// Panics on a fatal error or timeout.
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
    assert!(
        log.contains(term),
        "timed out waiting for {term:?} in the MCU log"
    );
}

#[test]
fn mctp_i3c_echo_host_test() {
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

    // The listener must be armed before the request arrives.
    wait_for_log(&mut hw, &mut log, READY);

    assert!(
        stream.send_private_write(target_addr, request()),
        "I3C socket closed before the write"
    );

    // Wait for the reply the way an MCTP controller does: an IBI says the target
    // has a response, then the response is read.
    let mut reply: Option<Vec<u8>> = None;
    let mut got_ibi = false;
    let start = Instant::now();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        if !out.is_empty() {
            print!("{out}");
            log.push_str(&out);
        }
        if !got_ibi {
            got_ibi = stream.receive_ibi(target_addr);
        } else if let Some(data) = stream.receive_private_read(target_addr) {
            reply = Some(data);
        }
        reply.is_some()
            || m.mci_fw_fatal_error().is_some()
            || start.elapsed().as_secs() > TIMEOUT_SECS
    });
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");
    let reply =
        reply.unwrap_or_else(|| panic!("no reply read from the target (IBI seen: {got_ibi})"));
    println!("mctp-i3c host: reply = {reply:02x?}");

    // [ver][dest = host][src = target][flags][msg type][payload...]
    assert!(
        reply.len() > MCTP_HEADER_LEN + 1,
        "reply too short: {reply:02x?}"
    );
    assert_eq!(reply[0] & 0x0f, 0x01, "MCTP header version");
    assert_eq!(reply[1], HOST_EID, "reply destination EID");
    assert_eq!(reply[2], TARGET_EID, "reply source EID");
    assert_eq!(
        reply[3] & 0xc0,
        0xc0,
        "reply is a single packet (SOM | EOM)"
    );
    assert_eq!(reply[3] & 0x08, 0, "reply is a response (tag owner clear)");
    assert_eq!(reply[MCTP_HEADER_LEN] & 0x7f, ECHO_MSG_TYPE, "message type");
    assert_eq!(&reply[MCTP_HEADER_LEN + 1..], PAYLOAD, "echoed payload");

    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
    drop(stream);
}
