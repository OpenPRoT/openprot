// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR MCTP-over-I3C echo test, written the way
//! caliptra-mcu-sw's own MCTP tests drive their firmware: boot through
//! `caliptra_mcu_hw_model` with an I3C socket, send an MCTP request as private
//! writes, then wait for the target's IBIs and read each reply packet
//! (`BufferedStream::receive_ibi` and `receive_private_read`, which checks and
//! strips the PEC). Single-threaded: the model is only advanced by `step_until`.
//!
//! Two requests go through one boot (the firmware echoes `ECHO_REQUESTS`, then
//! exits PASS):
//!
//! 1. A short payload: one packet each way.
//! 2. A payload larger than the MTU: the host fragments the request into several
//!    packets, which the target reassembles; the target fragments the echo, and
//!    the i3c server holds one staged packet at a time, so the MCTP server waits
//!    out `Busy` between fragments until each has been read. The host
//!    reassembles the reply.
//!
//! Each packet is DSP0233: the MCTP packet alone (nothing precedes it; the PEC
//! that `send_private_write` appends covers the address byte and the packet).
//!
//! The same binary runs on the emulator (x86_64) and on the VCK190, where the
//! model forwards the socket to the FPGA's I3C host controller.

use caliptra_mcu_hw_model::McuHwModel;
use caliptra_mcu_testing_common::i3c_socket::BufferedStream;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use test_boot_files::BootFiles;

const TARGET_EID: u8 = 8;
const HOST_EID: u8 = 9;
/// The message type the echo app listens for (`ECHO_MSG_TYPE` in `//services/mctp/echo`).
const ECHO_MSG_TYPE: u8 = 1;

/// The MCTP transport header is 4 bytes; the message body (type byte, payload)
/// follows it.
const MCTP_HEADER_LEN: usize = 4;

/// Largest packet the host writes, header included. The target's receive limit
/// is 256 bytes including the PEC.
const MAX_PACKET: usize = 251;

/// Logged by `echo.rs` once its listener is armed.
const READY: &str = "mctp echo: waiting for request";

const TIMEOUT_SECS: u64 = 120;

/// Time to let the target drain one write before the next. The TTI receive FIFO
/// holds 256 bytes, so back-to-back full packets would overrun it (caliptra-mcu-sw's
/// own packetized write leaves the same gap).
const WRITE_GAP: Duration = Duration::from_millis(50);

/// The payloads the host sends and expects echoed back: one packet, then
/// several.
fn payloads() -> [Vec<u8>; 2] {
    let short = b"mctp-i3c".to_vec();
    let long: Vec<u8> = (0..600u32).map(|i| (i % 251) as u8).collect();
    [short, long]
}

/// The MCTP packets of a request: the message body (type byte, then payload) cut
/// into packets of at most `MAX_PACKET` bytes, each with a transport header
/// (version 1, destination, source, flags). Flags: SOM on the first, EOM on the
/// last, a 2-bit packet sequence number, tag owner set, and `tag`.
fn request_packets(payload: &[u8], tag: u8) -> Vec<Vec<u8>> {
    let mut body = vec![ECHO_MSG_TYPE];
    body.extend_from_slice(payload);
    let chunks: Vec<&[u8]> = body.chunks(MAX_PACKET - MCTP_HEADER_LEN).collect();
    let last = chunks.len() - 1;
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut flags = 0x08 | (tag & 0x07) | (((i & 0x03) as u8) << 4);
            if i == 0 {
                flags |= 0x80;
            }
            if i == last {
                flags |= 0x40;
            }
            let mut packet = vec![0x01, TARGET_EID, HOST_EID, flags];
            packet.extend_from_slice(chunk);
            packet
        })
        .collect()
}

/// Append anything the firmware has logged to `log` and print it.
fn drain_log(hw: &mut impl McuHwModel, log: &mut String) {
    let out = hw.output().take(usize::MAX);
    if !out.is_empty() {
        print!("{out}");
        log.push_str(&out);
    }
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

/// Step the model for about `gap` of wall-clock time.
fn step_for(hw: &mut impl McuHwModel, log: &mut String, gap: Duration) {
    let start = Instant::now();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        if !out.is_empty() {
            print!("{out}");
            log.push_str(&out);
        }
        start.elapsed() >= gap || m.mci_fw_fatal_error().is_some()
    });
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");
}

/// Read one reply packet: wait for the target's IBI, then for the read it
/// triggers. Returns the packet with its PEC already checked and stripped.
fn read_packet(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
) -> Vec<u8> {
    let mut packet: Option<Vec<u8>> = None;
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
            packet = Some(data);
        }
        packet.is_some()
            || m.mci_fw_fatal_error().is_some()
            || start.elapsed().as_secs() > TIMEOUT_SECS
    });
    assert_eq!(hw.mci_fw_fatal_error(), None, "firmware hit a fatal error");
    packet.unwrap_or_else(|| panic!("no reply packet read from the target (IBI seen: {got_ibi})"))
}

/// Send `payload` as one MCTP request and collect the echo: the reply packets
/// are checked (EIDs, tag, sequence, SOM and EOM) and reassembled. Returns the
/// reply's message body, the type byte then the payload, and the packet count.
fn echo(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
    payload: &[u8],
    tag: u8,
) -> (Vec<u8>, usize) {
    let packets = request_packets(payload, tag);
    println!(
        "mctp-i3c host: request of {} bytes in {} packet(s)",
        payload.len(),
        packets.len()
    );
    for packet in packets {
        assert!(
            stream.send_private_write(target_addr, packet),
            "I3C socket closed before the write"
        );
        step_for(hw, log, WRITE_GAP);
    }

    let mut body = Vec::new();
    let mut count = 0usize;
    let mut first_seq = 0u8;
    loop {
        let packet = read_packet(hw, stream, log, target_addr);
        // [ver][dest = host][src = target][flags][body...]
        assert!(
            packet.len() > MCTP_HEADER_LEN,
            "reply packet too short: {packet:02x?}"
        );
        assert!(
            packet.len() <= MAX_PACKET,
            "reply packet of {} bytes is over the MTU",
            packet.len()
        );
        let flags = packet[3];
        assert_eq!(packet[0] & 0x0f, 0x01, "MCTP header version");
        assert_eq!(packet[1], HOST_EID, "reply destination EID");
        assert_eq!(packet[2], TARGET_EID, "reply source EID");
        assert_eq!(flags & 0x08, 0, "reply is a response (tag owner clear)");
        assert_eq!(flags & 0x07, tag & 0x07, "reply tag");
        // The first packet may start the sequence anywhere (DSP0236); each one
        // after it follows the previous, modulo 4.
        let seq = (flags >> 4) & 0x03;
        if count == 0 {
            first_seq = seq;
        }
        assert_eq!(seq, (first_seq + count as u8) & 0x03, "packet sequence");
        assert_eq!(
            flags & 0x80 != 0,
            count == 0,
            "SOM on the first packet only"
        );
        body.extend_from_slice(&packet[MCTP_HEADER_LEN..]);
        count += 1;
        if flags & 0x40 != 0 {
            break;
        }
    }
    (body, count)
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

    for (n, payload) in payloads().iter().enumerate() {
        let (body, packets) = echo(
            &mut hw,
            &mut stream,
            &mut log,
            target_addr,
            payload,
            n as u8,
        );
        println!(
            "mctp-i3c host: echo {n}: {} packet(s), {} byte body",
            packets,
            body.len()
        );
        assert_eq!(body[0] & 0x7f, ECHO_MSG_TYPE, "message type");
        assert_eq!(&body[1..], &payload[..], "echoed payload (request {n})");
        if n == 0 {
            assert_eq!(packets, 1, "a short message is one packet");
        } else {
            assert!(packets > 1, "the long message should have been fragmented");
        }
    }
    drain_log(&mut hw, &mut log);

    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
    drop(stream);
}
