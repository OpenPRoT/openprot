// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR SPDM test. The host is the SPDM requester: it boots
//! the firmware through `caliptra_mcu_hw_model` with an I3C socket, sends each
//! SPDM request as an MCTP message over I3C and reads the response.
//!
//! The test negotiates a connection (`GET_VERSION`, `GET_CAPABILITIES` and
//! `NEGOTIATE_ALGORITHMS`) and checks each response. It then sends a
//! `GET_DIGESTS` request, which the responder answers with an error because
//! it has no certificates, and checks that the responder still answers
//! `GET_VERSION`.
//!
//! The MCTP and I3C helpers are copies of those in the host test of
//! `//target/veer/tests/mctp_i3c`.
//!
//! TODO: build the requests and parse the responses with an SPDM requester
//! library. The test writes each request byte by byte and reads the response
//! fields at fixed offsets.
//!
//! The same binary runs on the emulator (x86_64) and on the VCK190, where the
//! model forwards the socket to the FPGA's I3C host controller.

use caliptra_mcu_hw_model::McuHwModel;
use caliptra_mcu_testing_common::i3c_socket::BufferedStream;
use mctp_i3c_ids::{HOST_EID, OWN_EID};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use test_boot_files::BootFiles;

const TARGET_EID: u8 = OWN_EID;

/// The MCTP message type of SPDM (DSP0239).
const SPDM_MSG_TYPE: u8 = 5;

/// The MCTP transport header is 4 bytes; the message body (type byte, payload)
/// follows it.
const MCTP_HEADER_LEN: usize = 4;

/// Largest packet the host writes, header included: the MCTP baseline, a
/// 64-byte payload and the header.
const MAX_PACKET: usize = MCTP_HEADER_LEN + 64;

/// Largest packet the target sends, header included.
const MAX_REPLY_PACKET: usize = 254;

/// Logged by `spdm_responder.rs` once its listener is armed.
const READY: &str = "spdm responder: waiting for request";

const TIMEOUT_SECS: u64 = 120;

/// Time to let the target drain one write before the next.
const WRITE_GAP: Duration = Duration::from_millis(50);

/// The least the target's clock must advance during a gap.
const MIN_GAP_CYCLES: u64 = 150_000;

/// SPDM version 1.0, which `GET_VERSION` and `VERSION` always use.
const SPDM_10: u8 = 0x10;

/// SPDM version 1.2, the highest that the responder supports.
const SPDM_12: u8 = 0x12;

// Request and response codes (DSP0274).
const GET_DIGESTS: u8 = 0x81;
const GET_VERSION: u8 = 0x84;
const GET_CAPABILITIES: u8 = 0xe1;
const NEGOTIATE_ALGORITHMS: u8 = 0xe3;
const VERSION: u8 = 0x04;
const CAPABILITIES: u8 = 0x61;
const ALGORITHMS: u8 = 0x63;
const ERROR: u8 = 0x7f;

/// The SPDM error code `UnsupportedRequest`.
const UNSUPPORTED_REQUEST: u8 = 0x07;

/// The largest SPDM message that the responder (`MAX_SPDM_MESSAGE` in
/// `spdm_responder.rs`) and this requester send or receive.
const MAX_SPDM_MESSAGE: u32 = 1024;

/// `BaseAsymAlgo` bit for ECDSA with NIST P-384.
const ECDSA_P384: u32 = 1 << 7;

/// `BaseHashAlgo` bit for SHA-384.
const SHA_384: u32 = 1 << 1;

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

/// Step the model for at least `gap` of wall-clock time and `MIN_GAP_CYCLES` cycles.
fn step_for(hw: &mut impl McuHwModel, log: &mut String, gap: Duration) {
    let start = Instant::now();
    let c0 = hw.cycle_count();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        if !out.is_empty() {
            print!("{out}");
            log.push_str(&out);
        }
        (start.elapsed() >= gap && m.cycle_count() - c0 >= MIN_GAP_CYCLES)
            || m.mci_fw_fatal_error().is_some()
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

/// Read one message from the target and reassemble it. Returns the tag, the
/// tag owner bit and the body (the type byte, then the payload).
fn read_message(
    hw: &mut impl McuHwModel,
    stream: &mut BufferedStream,
    log: &mut String,
    target_addr: u8,
) -> (u8, bool, Vec<u8>) {
    let mut body = Vec::new();
    loop {
        let packet = read_packet(hw, stream, log, target_addr);
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
        assert_eq!(
            flags & 0x80 != 0,
            body.is_empty(),
            "SOM on the first packet only"
        );
        body.extend_from_slice(&packet[MCTP_HEADER_LEN..]);
        if flags & 0x40 != 0 {
            return (flags & 0x07, flags & 0x08 != 0, body);
        }
    }
}

/// The hardware model, its I3C socket and the MCU log of one run.
struct Requester<M: McuHwModel> {
    hw: M,
    stream: BufferedStream,
    log: String,
    target_addr: u8,
    /// The MCTP tag of the next request.
    tag: u8,
}

impl<M: McuHwModel> Requester<M> {
    /// Send one SPDM request and return the SPDM response.
    fn exchange(&mut self, request: &[u8]) -> Vec<u8> {
        let tag = self.tag;
        self.tag = (self.tag + 1) & 0x07;
        let mut body = vec![SPDM_MSG_TYPE];
        body.extend_from_slice(request);
        let packets = message_packets(&body, tag, true, TARGET_EID, HOST_EID);
        send_packets(
            &mut self.hw,
            &mut self.stream,
            &mut self.log,
            self.target_addr,
            packets,
        );
        let (reply_tag, tag_owner, reply) = read_message(
            &mut self.hw,
            &mut self.stream,
            &mut self.log,
            self.target_addr,
        );
        assert!(!tag_owner, "reply is a response (tag owner clear)");
        assert_eq!(reply_tag, tag, "reply tag");
        assert_eq!(reply[0] & 0x7f, SPDM_MSG_TYPE, "message type");
        println!(
            "spdm host: request {:02x?} -> response {:02x?}",
            request,
            &reply[1..]
        );
        reply[1..].to_vec()
    }

    /// Send `GET_VERSION` and return the versions that the responder supports,
    /// each as one byte (major in the high four bits, minor in the low four).
    fn get_version(&mut self) -> Vec<u8> {
        let response = self.exchange(&[SPDM_10, GET_VERSION, 0, 0]);
        assert_eq!(&response[..2], [SPDM_10, VERSION], "VERSION header");
        // [version][code][param1][param2][reserved][count][entries...]
        let count = usize::from(response[5]);
        assert_eq!(response.len(), 6 + 2 * count, "VERSION length");
        // Each entry is 16 bits, little-endian. The high byte has the major
        // version in its high four bits and the minor in its low four.
        response[6..].chunks(2).map(|entry| entry[1]).collect()
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn spdm_negotiate_host_test() {
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
    let mut requester = Requester {
        hw,
        stream: BufferedStream::new(stream),
        log: String::new(),
        target_addr,
        tag: 0,
    };

    // The listener must be armed before the request arrives.
    wait_for_log(&mut requester.hw, &mut requester.log, READY);

    let versions = requester.get_version();
    assert!(versions.contains(&0x11), "responder supports SPDM 1.1");
    assert!(versions.contains(&SPDM_12), "responder supports SPDM 1.2");

    // GET_CAPABILITIES of SPDM 1.2:
    // [version][code][param1][param2][reserved][CTExponent][reserved x2]
    // [Flags x4][DataTransferSize x4][MaxSPDMmsgSize x4]
    let mut request = vec![SPDM_12, GET_CAPABILITIES, 0, 0, 0, 0, 0, 0];
    request.extend_from_slice(&0u32.to_le_bytes());
    request.extend_from_slice(&MAX_SPDM_MESSAGE.to_le_bytes());
    request.extend_from_slice(&MAX_SPDM_MESSAGE.to_le_bytes());
    let response = requester.exchange(&request);
    assert_eq!(
        &response[..2],
        [SPDM_12, CAPABILITIES],
        "CAPABILITIES header"
    );
    assert_eq!(response.len(), 20, "CAPABILITIES length");
    assert_eq!(u32_at(&response, 8), 0, "responder capability flags");
    assert_eq!(u32_at(&response, 12), MAX_SPDM_MESSAGE, "DataTransferSize");
    assert_eq!(u32_at(&response, 16), MAX_SPDM_MESSAGE, "MaxSPDMmsgSize");

    // NEGOTIATE_ALGORITHMS of SPDM 1.2 with no algorithm structures:
    // [version][code][param1 = structures][param2][Length x2]
    // [MeasurementSpecification][OtherParamsSupport][BaseAsymAlgo x4]
    // [BaseHashAlgo x4][reserved x12][ExtAsymCount][ExtHashCount][reserved x2]
    let mut request = vec![SPDM_12, NEGOTIATE_ALGORITHMS, 0, 0, 32, 0, 0x01, 0];
    request.extend_from_slice(&ECDSA_P384.to_le_bytes());
    request.extend_from_slice(&SHA_384.to_le_bytes());
    request.extend_from_slice(&[0; 16]);
    assert_eq!(request.len(), 32);
    let response = requester.exchange(&request);
    assert_eq!(&response[..2], [SPDM_12, ALGORITHMS], "ALGORITHMS header");
    // [version][code][param1][param2][Length x2][MeasurementSpecificationSel]
    // [OtherParamsSelection][MeasurementHashAlgo x4][BaseAsymSel x4]
    // [BaseHashSel x4]...
    assert_eq!(
        usize::from(u16::from_le_bytes([response[4], response[5]])),
        response.len(),
        "ALGORITHMS length"
    );
    // The responder has no measurements, so it selects no measurement
    // specification and no measurement hash.
    assert_eq!(response[6], 0, "MeasurementSpecificationSel");
    assert_eq!(u32_at(&response, 8), 0, "MeasurementHashAlgo");
    assert_eq!(u32_at(&response, 12), ECDSA_P384, "BaseAsymSel");
    assert_eq!(u32_at(&response, 16), SHA_384, "BaseHashSel");

    // The responder has no certificates, so it answers GET_DIGESTS with an
    // error.
    let response = requester.exchange(&[SPDM_12, GET_DIGESTS, 0, 0]);
    assert_eq!(&response[..2], [SPDM_12, ERROR], "ERROR header");
    assert_eq!(response[2], UNSUPPORTED_REQUEST, "error code");

    // The responder still answers after the error.
    let versions = requester.get_version();
    assert!(versions.contains(&SPDM_12), "responder supports SPDM 1.2");
}
