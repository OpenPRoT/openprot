// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR MCTP-over-I3C echo test, written the way
//! caliptra-mcu-sw's own MCTP tests drive their firmware: boot through
//! `caliptra_mcu_hw_model` with an I3C socket, send an MCTP request as private
//! writes, then wait for the target's IBIs and read each reply packet
//! (`BufferedStream::receive_ibi` and `receive_private_read`, which checks and
//! strips the PEC). Single-threaded: the model is only advanced by `step_until`.
//!
//! Two tests share one firmware image, which echoes every request and exits PASS
//! after one whose payload is `DONE`. A request whose payload is `INITIATE` also
//! makes the firmware send requests of its own (`TARGET_REQUEST_SIZES`), which the
//! host reads and answers: the target-initiated direction, where the target owns
//! the tag and the host is the responder.
//!
//! * `mctp_i3c_echo_host_test`, the quick test, sends two requests: a short
//!   payload (one packet each way), then one larger than the MTU. The host
//!   fragments the request into several packets, which the target reassembles;
//!   the target fragments the echo, and the i3c server holds one staged packet at
//!   a time, so the MCTP server waits out `Busy` between fragments until each has
//!   been read. The host reassembles the reply. Then the target-initiated
//!   requests.
//! * `mctp_i3c_soak_host_test` sends `SOAK_REQUESTS` requests of different sizes
//!   and tags in one boot, with the target-initiated requests in between, to see
//!   whether anything fails after many frames. It is `#[ignore]`d so that the
//!   quick test stays quick; run it with `--test_arg=--ignored --test_arg=--nocapture`.
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

/// Largest packet the host writes, header included: the MCTP baseline, a 64-byte
/// payload and the header. A full frame fills the target's 64-word RX FIFO and takes
/// about 12 ms to read out, and the writes reach the target as little as a few
/// milliseconds apart, so larger packets were cut short (see `MCTP_I3C_IPC_MAXMTU`).
/// caliptra-mcu-sw's own host helper writes 64-byte payloads too.
const MAX_PACKET: usize = MCTP_HEADER_LEN + 64;

/// Largest packet the target sends, header included: its MTU
/// (`MCTP_I3C_IPC_MAXMTU`).
const MAX_REPLY_PACKET: usize = MAX_PACKET;

/// Logged by `echo.rs` once its listener is armed.
const READY: &str = "mctp echo: waiting for request";

const TIMEOUT_SECS: u64 = 120;

/// Time to let the target drain one write before the next. The TTI receive FIFO
/// holds 256 bytes, so back-to-back full packets would overrun it (caliptra-mcu-sw's
/// own packetized write leaves the same gap).
const WRITE_GAP: Duration = Duration::from_millis(50);

/// The payload of the request that ends the run (the firmware's `DONE`).
const DONE: &[u8] = b"mctp-i3c-done";

/// The payload of the request that makes the firmware send requests of its own
/// (the firmware's `INITIATE`).
const INITIATE: &[u8] = b"mctp-i3c-initiate";

/// Payload sizes of the requests the firmware sends once asked, in order (the
/// firmware's `TARGET_REQUEST_SIZES`). They are the sizes of caliptra-mcu-sw's own
/// loopback test: the baseline, one under it, and two that need several packets.
const TARGET_REQUEST_SIZES: [usize; 4] = [64, 63, 256, 1000];

/// The payload of the firmware's request of `len` bytes: the pattern the firmware
/// builds the same way (`fill_target_payload` in `echo.rs`).
fn target_payload(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 253) as u8).collect()
}

/// One step of a run.
enum Step {
    /// Send this payload as a request and check the echo.
    Echo(Vec<u8>),
    /// Ask the firmware to send its requests, and answer them.
    TargetRequests,
}

/// The payloads the quick test sends and expects echoed back: one packet, then
/// several.
fn payloads() -> Vec<Vec<u8>> {
    let short = b"mctp-i3c".to_vec();
    let long: Vec<u8> = (0..600u32).map(|i| (i % 251) as u8).collect();
    vec![short, long]
}

/// How many echo requests the soak sends.
const SOAK_REQUESTS: usize = 60;

/// The soak asks for the target-initiated requests after every this many echoes.
const SOAK_TARGET_EVERY: usize = 25;

/// Payload sizes the soak cycles through. With 68-byte packets (64 body bytes
/// after the header) they give 1 to 17 packets each way. The body is the type byte
/// and the payload, so 63 bytes fill one packet exactly and 64 spill into a second;
/// 255, 511 and so on are the same edge at each size. 249 and 250 are the old
/// 254-byte packet edge, and 1000 and 1024 are the sizes of caliptra-mcu-sw's own
/// loopback tests. The largest is within what the stack accepts and the firmware's
/// echo buffer holds (1032).
const SOAK_SIZES: [usize; 12] = [8, 63, 64, 120, 249, 250, 256, 499, 600, 749, 1000, 1024];

/// A payload of `len` bytes that differs from one request to the next, so that a
/// frame delivered in the wrong place or twice does not pass as the right one.
fn soak_payload(n: usize, len: usize) -> Vec<u8> {
    let mut x = 0x9e37_79b9u32 ^ (n as u32).wrapping_mul(0x85eb_ca6b) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 8) as u8
        })
        .collect()
}

/// The MCTP packets of a message: the body (type byte, then payload) cut into
/// packets of at most `MAX_PACKET` bytes, each with a transport header (version 1,
/// `dest`, `src`, flags). Flags: SOM on the first, EOM on the last, a 2-bit packet
/// sequence number, the tag owner bit (set for a request, clear for a response),
/// and `tag`.
fn message_packets(body: &[u8], tag: u8, tag_owner: bool, dest: u8, src: u8) -> Vec<Vec<u8>> {
    let chunks: Vec<&[u8]> = body.chunks(MAX_PACKET - MCTP_HEADER_LEN).collect();
    let last = chunks.len() - 1;
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut flags = (tag & 0x07) | (((i & 0x03) as u8) << 4);
            if tag_owner {
                flags |= 0x08;
            }
            if i == 0 {
                flags |= 0x80;
            }
            if i == last {
                flags |= 0x40;
            }
            let mut packet = vec![0x01, dest, src, flags];
            packet.extend_from_slice(chunk);
            packet
        })
        .collect()
}

/// The packets of a request from the host to the target.
fn request_packets(payload: &[u8], tag: u8) -> Vec<Vec<u8>> {
    let mut body = vec![ECHO_MSG_TYPE];
    body.extend_from_slice(payload);
    message_packets(&body, tag, true, TARGET_EID, HOST_EID)
}

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

/// Write each packet to the target, leaving it time to drain its receive FIFO
/// after each.
fn send_packets(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
    packets: Vec<Vec<u8>>,
) {
    for packet in packets {
        assert!(
            stream.send_private_write(target_addr, packet),
            "I3C socket closed before the write"
        );
        step_for(hw, log, WRITE_GAP);
    }
}

/// A message the target sent, reassembled.
struct Message {
    /// The message tag (3 bits).
    tag: u8,
    /// Whether the tag owner bit is set: a request, not a response.
    tag_owner: bool,
    /// The type byte, then the payload.
    body: Vec<u8>,
    /// How many packets it took.
    packets: usize,
}

/// Read one message from the target: its packets are checked (version, EIDs, tag
/// and owner, sequence, SOM and EOM, size) and reassembled.
fn read_message(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
) -> Message {
    let mut body = Vec::new();
    let mut count = 0usize;
    let mut first_seq = 0u8;
    let mut first_len = 0usize;
    let mut tag = 0u8;
    let mut tag_owner = false;
    loop {
        let packet = read_packet(hw, stream, log, target_addr);
        if count == 0 {
            first_len = packet.len();
        }
        // [ver][dest = host][src = target][flags][body...]
        assert!(
            packet.len() > MCTP_HEADER_LEN,
            "packet too short: {packet:02x?}"
        );
        assert!(
            packet.len() <= MAX_REPLY_PACKET,
            "packet of {} bytes is over the MTU",
            packet.len()
        );
        let flags = packet[3];
        assert_eq!(packet[0] & 0x0f, 0x01, "MCTP header version");
        assert_eq!(packet[1], HOST_EID, "destination EID");
        assert_eq!(packet[2], TARGET_EID, "source EID");
        // The tag and its owner bit are the same in every packet of a message.
        if count == 0 {
            tag = flags & 0x07;
            tag_owner = flags & 0x08 != 0;
        }
        assert_eq!(flags & 0x07, tag, "tag, packet {count}");
        assert_eq!(flags & 0x08 != 0, tag_owner, "tag owner, packet {count}");
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
    // A message of more than one packet starts with a packet of the target's
    // whole MTU, so this is what shows the target sends the size it reports,
    // and not just a size under the bound.
    if count > 1 {
        assert_eq!(
            first_len, MAX_REPLY_PACKET,
            "the first packet of a long message should fill the MTU"
        );
    }
    Message {
        tag,
        tag_owner,
        body,
        packets: count,
    }
}

/// Send `payload` as one MCTP request and collect the echo. Returns the reply's
/// message body, the type byte then the payload, and the packet count.
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
    send_packets(hw, stream, log, target_addr, packets);
    let msg = read_message(hw, stream, log, target_addr);
    assert!(!msg.tag_owner, "reply is a response (tag owner clear)");
    assert_eq!(msg.tag, tag & 0x07, "reply tag");
    (msg.body, msg.packets)
}

/// How many packets the host has written to the target and read from it.
#[derive(Default)]
struct Frames {
    sent: usize,
    received: usize,
}

/// Send `payload` as a request and check the echo: type, payload and packet count.
fn exchange(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
    payload: &[u8],
    tag: u8,
    frames: &mut Frames,
) {
    let (body, packets) = echo(hw, stream, log, target_addr, payload, tag);
    println!(
        "mctp-i3c host: echo: {} packet(s), {} byte body",
        packets,
        body.len()
    );
    assert_eq!(body[0] & 0x7f, ECHO_MSG_TYPE, "message type");
    assert_eq!(&body[1..], payload, "echoed payload");
    // The type byte then the payload, in packets of up to the MTU.
    let expected = (payload.len() + 1).div_ceil(MAX_REPLY_PACKET - MCTP_HEADER_LEN);
    assert_eq!(packets, expected, "packets in the echo");
    frames.sent += (payload.len() + 1).div_ceil(MAX_PACKET - MCTP_HEADER_LEN);
    frames.received += packets;
}

/// Read one request the target sends, check it, and answer it with the same body
/// and tag as a response. The target owns the tag of its request: its owner bit
/// is set, and the response clears it.
fn serve_request(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
    expected: &[u8],
    frames: &mut Frames,
) {
    let msg = read_message(hw, stream, log, target_addr);
    println!(
        "mctp-i3c host: target request of {} bytes in {} packet(s), tag {}",
        msg.body.len() - 1,
        msg.packets,
        msg.tag
    );
    assert!(msg.tag_owner, "a request the target sends owns its tag");
    assert_eq!(msg.body[0] & 0x7f, ECHO_MSG_TYPE, "message type");
    assert_eq!(&msg.body[1..], expected, "the target's request payload");
    let response = message_packets(&msg.body, msg.tag, false, TARGET_EID, HOST_EID);
    frames.received += msg.packets;
    frames.sent += response.len();
    send_packets(hw, stream, log, target_addr, response);
}

/// Ask the firmware to send its requests, and answer each.
fn target_requests(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
    tag: u8,
    frames: &mut Frames,
) {
    exchange(hw, stream, log, target_addr, INITIATE, tag, frames);
    for len in TARGET_REQUEST_SIZES {
        serve_request(
            hw,
            stream,
            log,
            target_addr,
            &target_payload(len),
            frames,
        );
    }
}

/// Boot the firmware and run each step (echo requests are tagged `n & 7`, `n`
/// counting the requests the host sends), then end the run with `DONE` and wait for
/// the firmware to exit PASS. Prints a line per request, so that a failure shows
/// where it stopped.
fn run(plan: &[Step]) {
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

    let mut frames = Frames::default();
    let mut n = 0usize;
    for (i, step) in plan.iter().enumerate() {
        match step {
            Step::Echo(payload) => {
                println!(
                    "mctp-i3c host: step {}/{}: echo, {} bytes, tag {}",
                    i + 1,
                    plan.len(),
                    payload.len(),
                    n & 7
                );
                exchange(
                    &mut hw,
                    &mut stream,
                    &mut log,
                    target_addr,
                    payload,
                    (n & 7) as u8,
                    &mut frames,
                );
            }
            Step::TargetRequests => {
                println!(
                    "mctp-i3c host: step {}/{}: target requests, tag {}",
                    i + 1,
                    plan.len(),
                    n & 7
                );
                target_requests(
                    &mut hw,
                    &mut stream,
                    &mut log,
                    target_addr,
                    (n & 7) as u8,
                    &mut frames,
                );
            }
        }
        n += 1;
    }
    println!("mctp-i3c host: ending the run");
    exchange(
        &mut hw,
        &mut stream,
        &mut log,
        target_addr,
        DONE,
        (n & 7) as u8,
        &mut frames,
    );
    drain_log(&mut hw, &mut log);

    hw.step_until_exit_success()
        .expect("firmware did not exit with PASS");
    println!(
        "mctp-i3c host: completed {} steps, {} packets sent, {} received",
        plan.len() + 1,
        frames.sent,
        frames.received
    );
    drop(stream);
}

#[test]
fn mctp_i3c_echo_host_test() {
    let mut plan: Vec<Step> = payloads().into_iter().map(Step::Echo).collect();
    plan.push(Step::TargetRequests);
    run(&plan);
}

/// Many requests of different sizes and tags in one boot, with the
/// target-initiated requests in between. Slow, so it is ignored by default.
#[test]
#[ignore = "soak; run with --test_arg=--ignored --test_arg=--nocapture"]
fn mctp_i3c_soak_host_test() {
    let mut plan = Vec::new();
    for n in 0..SOAK_REQUESTS {
        plan.push(Step::Echo(soak_payload(n, SOAK_SIZES[n % SOAK_SIZES.len()])));
        if (n + 1) % SOAK_TARGET_EVERY == 0 {
            plan.push(Step::TargetRequests);
        }
    }
    run(&plan);
}
