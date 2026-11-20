// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host side of the VeeR PLDM firmware update test. The host is the PLDM
//! update agent: it boots the firmware through `caliptra_mcu_hw_model` with
//! an I3C socket and exchanges PLDM messages with the firmware device as MCTP
//! messages over I3C.
//!
//! The update has two components, which are the boot files of
//! `//target/veer/tests/pldm/updated`: its SoC manifest and its MCU runtime
//! image. The test:
//!
//! 1. reads the device's identifiers and firmware parameters
//!    (`QueryDeviceIdentifiers`, `GetFirmwareParameters`);
//! 2. starts the update and describes the components (`RequestUpdate`,
//!    `PassComponentTable`);
//! 3. for the manifest and then the image, sends `UpdateComponent` and
//!    answers the requests that the device sends: `RequestFirmwareData` until
//!    the component is transferred, then `TransferComplete`, `VerifyComplete`
//!    and `ApplyComplete`;
//! 4. sends `ActivateFirmware`;
//! 5. waits for the updated image to run and exit with PASS.
//!
//! The update agent sends the messages in sequence and encodes and decodes
//! them by hand (DSP0267 1.3), because caliptra-mcu-sw's
//! `caliptra-mcu-pldm-ua` needs threads for its timers and its receive loop,
//! and only this thread advances the model.
//!
//! The MCTP and I3C helpers are copies of those in the host test of
//! `//target/veer/tests/mctp_i3c`.
//!
//! TODO: send a PLDM firmware update package (`caliptra-mcu-pldm-fw-pkg`)
//! and take the components from its header.

use caliptra_mcu_hw_model::McuHwModel;
use caliptra_mcu_testing_common::i3c_socket::BufferedStream;
use mctp_i3c_ids::{HOST_EID, OWN_EID};
use pldm_update_ids::{
    ACTIVE_COMPARISON_STAMP, ACTIVE_VERSION, DEVICE_UUID, MANIFEST_CLASSIFICATION,
    MANIFEST_IDENTIFIER, READY, RUNTIME_CLASSIFICATION, RUNTIME_IDENTIFIER, UPDATED,
};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use test_boot_files::BootFiles;

const TARGET_EID: u8 = OWN_EID;

/// The MCTP message type of PLDM (DSP0239).
const PLDM_MSG_TYPE: u8 = 1;

/// The MCTP transport header is 4 bytes; the message body (type byte, payload)
/// follows it.
const MCTP_HEADER_LEN: usize = 4;

/// Largest packet the host writes, header included. With the PEC it fits in
/// the 256-byte receive FIFO of the I3C target. The tests that also run on
/// the FPGA write packets of 68 bytes, because larger ones overflow that FIFO
/// on the FPGA (see `//target/veer/tests/mctp_i3c`). This test runs on the
/// emulator only. A 512-byte part of a component takes 3 packets of this
/// size, and 9 packets of 68 bytes.
const MAX_PACKET: usize = MCTP_HEADER_LEN + 240;

/// Largest packet the target sends, header included.
const MAX_REPLY_PACKET: usize = 254;

/// Logged by the control responder once its listener is armed.
const CONTROL_READY: &str = "mctp control: responder ready";

const TIMEOUT_SECS: u64 = 300;

/// Time to let the target drain one write before the next. The tests that
/// also run on the FPGA leave 50 ms and 150,000 cycles. The update is about
/// 370 writes, and this test, which runs on the emulator only, leaves less.
const WRITE_GAP: Duration = Duration::from_millis(10);

/// The least the target's clock must advance during a gap.
const MIN_GAP_CYCLES: u64 = 30_000;

/// The PLDM type of firmware update (DSP0245).
const PLDM_TYPE_FW_UPDATE: u8 = 5;

/// The request bit of the first byte of a PLDM header.
const PLDM_REQUEST: u8 = 0x80;

// Firmware update commands (DSP0267).
const QUERY_DEVICE_IDENTIFIERS: u8 = 0x01;
const GET_FIRMWARE_PARAMETERS: u8 = 0x02;
const REQUEST_UPDATE: u8 = 0x10;
const PASS_COMPONENT_TABLE: u8 = 0x13;
const UPDATE_COMPONENT: u8 = 0x14;
const REQUEST_FIRMWARE_DATA: u8 = 0x15;
const TRANSFER_COMPLETE: u8 = 0x16;
const VERIFY_COMPLETE: u8 = 0x17;
const APPLY_COMPLETE: u8 = 0x18;
const ACTIVATE_FIRMWARE: u8 = 0x1a;

/// The completion code of a command that succeeded.
const SUCCESS: u8 = 0;

/// The descriptor type of a UUID.
const DESCRIPTOR_UUID: u16 = 0x0002;

/// The ASCII string type.
const STRING_ASCII: u8 = 1;

/// The largest part of a component that the update agent sends in one
/// `RequestFirmwareData` response.
const MAX_TRANSFER_SIZE: u32 = 512;

/// The version string and the comparison stamp of the update.
const UPDATE_VERSION: &str = "2.0.0";
const UPDATE_COMPARISON_STAMP: u32 = 2;

/// `PassComponentTable` transfer flags.
const TRANSFER_START: u8 = 0x01;
const TRANSFER_END: u8 = 0x04;

/// A component of the update.
struct Component {
    name: &'static str,
    classification: u16,
    identifier: u16,
    image: Vec<u8>,
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

/// Read one packet from the target: wait for the target's IBI, then for the
/// read it triggers. Returns the packet with its PEC already checked and
/// stripped.
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
    packet.unwrap_or_else(|| panic!("no packet read from the target (IBI seen: {got_ibi})"))
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

/// A PLDM version string field: the type, the length and the characters.
fn version_field(version: &str) -> Vec<u8> {
    let mut field = vec![STRING_ASCII, version.len() as u8];
    field.extend_from_slice(version.as_bytes());
    field
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

/// A request that the firmware device sent.
struct DeviceRequest {
    /// The MCTP tag, which the response repeats.
    tag: u8,
    /// The PLDM instance identifier, which the response repeats.
    instance: u8,
    command: u8,
    /// The request after the PLDM header.
    payload: Vec<u8>,
}

/// The update agent: the hardware model, its I3C socket and the MCU log of
/// one run.
struct UpdateAgent<M: McuHwModel> {
    hw: M,
    stream: BufferedStream,
    log: String,
    target_addr: u8,
    /// The MCTP tag of the next request.
    tag: u8,
    /// The PLDM instance identifier of the next request.
    instance: u8,
}

impl<M: McuHwModel> UpdateAgent<M> {
    /// Send the firmware update command `command` with `payload` and return
    /// the response after its PLDM header: the completion code and the
    /// response data.
    fn request(&mut self, command: u8, payload: &[u8]) -> Vec<u8> {
        let tag = self.tag;
        self.tag = (self.tag + 1) & 0x07;
        let instance = self.instance;
        self.instance = (self.instance + 1) & 0x1f;

        let mut body = vec![
            PLDM_MSG_TYPE,
            PLDM_REQUEST | instance,
            PLDM_TYPE_FW_UPDATE,
            command,
        ];
        body.extend_from_slice(payload);
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
        assert!(reply.len() >= 5, "response too short: {reply:02x?}");
        assert_eq!(reply[0] & 0x7f, PLDM_MSG_TYPE, "message type");
        assert_eq!(reply[1], instance, "response instance identifier");
        assert_eq!(reply[2] & 0x3f, PLDM_TYPE_FW_UPDATE, "response PLDM type");
        assert_eq!(reply[3], command, "response command");
        println!(
            "pldm host: command 0x{command:02x}, {} byte request -> response {:02x?}",
            payload.len(),
            &reply[4..]
        );
        reply[4..].to_vec()
    }

    /// Read one request that the firmware device sends.
    fn device_request(&mut self) -> DeviceRequest {
        let (tag, tag_owner, body) = read_message(
            &mut self.hw,
            &mut self.stream,
            &mut self.log,
            self.target_addr,
        );
        assert!(tag_owner, "a request the device sends owns its tag");
        assert!(body.len() >= 4, "request too short: {body:02x?}");
        assert_eq!(body[0] & 0x7f, PLDM_MSG_TYPE, "message type");
        assert_eq!(body[1] & PLDM_REQUEST, PLDM_REQUEST, "request bit");
        assert_eq!(body[2] & 0x3f, PLDM_TYPE_FW_UPDATE, "request PLDM type");
        DeviceRequest {
            tag,
            instance: body[1] & 0x1f,
            command: body[3],
            payload: body[4..].to_vec(),
        }
    }

    /// Answer `request` with `completion_code` and `data`.
    fn respond(&mut self, request: &DeviceRequest, completion_code: u8, data: &[u8]) {
        let mut body = vec![
            PLDM_MSG_TYPE,
            request.instance,
            PLDM_TYPE_FW_UPDATE,
            request.command,
            completion_code,
        ];
        body.extend_from_slice(data);
        let packets = message_packets(&body, request.tag, false, TARGET_EID, HOST_EID);
        send_packets(
            &mut self.hw,
            &mut self.stream,
            &mut self.log,
            self.target_addr,
            packets,
        );
    }

    /// `QueryDeviceIdentifiers`: check that the device reports its UUID.
    fn query_device_identifiers(&mut self) {
        let response = self.request(QUERY_DEVICE_IDENTIFIERS, &[]);
        // [completion code][identifiers length (4)][descriptor count]
        // [descriptor type (2)][descriptor length (2)][descriptor data]
        assert_eq!(response[0], SUCCESS, "QueryDeviceIdentifiers");
        assert_eq!(u32_at(&response, 1), 4 + 16, "identifiers length");
        assert_eq!(response[5], 1, "descriptor count");
        assert_eq!(u16_at(&response, 6), DESCRIPTOR_UUID, "descriptor type");
        assert_eq!(u16_at(&response, 8), 16, "descriptor length");
        assert_eq!(&response[10..], DEVICE_UUID, "device UUID");
    }

    /// `GetFirmwareParameters`: check the version and the two components that
    /// the device reports.
    fn get_firmware_parameters(&mut self) {
        let response = self.request(GET_FIRMWARE_PARAMETERS, &[]);
        // [completion code][capabilities (4)][component count (2)]
        // [active string type][active string length]
        // [pending string type][pending string length]
        // [active string][pending string][component table]
        assert_eq!(response[0], SUCCESS, "GetFirmwareParameters");
        assert_eq!(u16_at(&response, 5), 2, "component count");
        let active_len = usize::from(response[8]);
        let pending_len = usize::from(response[10]);
        assert_eq!(
            &response[11..11 + active_len],
            ACTIVE_VERSION.as_bytes(),
            "active image set version"
        );
        assert_eq!(pending_len, 0, "no pending image set version");

        // A component: [classification (2)][identifier (2)]
        // [classification index][active comparison stamp (4)]
        // [active string type][active string length][active release date (8)]
        // [pending comparison stamp (4)][pending string type]
        // [pending string length][pending release date (8)]
        // [activation methods (2)][capabilities (4)]
        // [active string][pending string]
        let mut table = &response[11 + active_len + pending_len..];
        let mut components = Vec::new();
        while !table.is_empty() {
            let active_len = usize::from(table[10]);
            let pending_len = usize::from(table[24]);
            assert_eq!(
                u32_at(table, 5),
                ACTIVE_COMPARISON_STAMP,
                "active comparison stamp"
            );
            assert_eq!(
                &table[39..39 + active_len],
                ACTIVE_VERSION.as_bytes(),
                "active component version"
            );
            components.push((u16_at(table, 0), u16_at(table, 2)));
            table = &table[39 + active_len + pending_len..];
        }
        assert_eq!(
            components,
            [
                (MANIFEST_CLASSIFICATION, MANIFEST_IDENTIFIER),
                (RUNTIME_CLASSIFICATION, RUNTIME_IDENTIFIER)
            ],
            "components"
        );
    }

    /// `RequestUpdate` for an update of `components` components.
    fn request_update(&mut self, components: u16) {
        let mut payload = MAX_TRANSFER_SIZE.to_le_bytes().to_vec();
        payload.extend_from_slice(&components.to_le_bytes());
        // One outstanding transfer request and no package data.
        payload.push(1);
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.extend_from_slice(&version_field(UPDATE_VERSION));
        let response = self.request(REQUEST_UPDATE, &payload);
        // [completion code][device metadata length (2)]
        // [device will send GetPackageData]
        assert_eq!(response[0], SUCCESS, "RequestUpdate");
        assert_eq!(u16_at(&response, 1), 0, "no device metadata");
        assert_eq!(response[3], 0, "the device does not ask for package data");
    }

    /// `PassComponentTable` for `component`.
    fn pass_component(&mut self, component: &Component, transfer_flag: u8) {
        let mut payload = vec![transfer_flag];
        payload.extend_from_slice(&component.classification.to_le_bytes());
        payload.extend_from_slice(&component.identifier.to_le_bytes());
        payload.push(0);
        payload.extend_from_slice(&UPDATE_COMPARISON_STAMP.to_le_bytes());
        payload.extend_from_slice(&version_field(UPDATE_VERSION));
        let response = self.request(PASS_COMPONENT_TABLE, &payload);
        // [completion code][component response][component response code]
        assert_eq!(response[0], SUCCESS, "PassComponentTable");
        assert_eq!(
            &response[1..3],
            [0, 0],
            "the device can update {}",
            component.name
        );
    }

    /// `UpdateComponent` for `component`, then the requests that the device
    /// sends until it has transferred, verified and applied the component.
    fn update_component(&mut self, component: &Component) {
        let mut payload = component.classification.to_le_bytes().to_vec();
        payload.extend_from_slice(&component.identifier.to_le_bytes());
        payload.push(0);
        payload.extend_from_slice(&UPDATE_COMPARISON_STAMP.to_le_bytes());
        payload.extend_from_slice(&(component.image.len() as u32).to_le_bytes());
        // No update option flags.
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&version_field(UPDATE_VERSION));
        let response = self.request(UPDATE_COMPONENT, &payload);
        // [completion code][compatibility response]
        // [compatibility response code][update option flags enabled (4)]
        // [time before RequestFirmwareData (2)]
        assert_eq!(response[0], SUCCESS, "UpdateComponent");
        assert_eq!(
            &response[1..3],
            [0, 0],
            "the device will update {}",
            component.name
        );

        let mut sent = 0usize;
        let mut transferred = false;
        let mut verified = false;
        loop {
            let request = self.device_request();
            match request.command {
                REQUEST_FIRMWARE_DATA => {
                    // [offset (4)][length (4)]
                    let offset = u32_at(&request.payload, 0) as usize;
                    let length = u32_at(&request.payload, 4) as usize;
                    assert!(length <= MAX_TRANSFER_SIZE as usize, "transfer size");
                    assert_eq!(offset, sent, "the device asks for the parts in order");
                    // A part that ends after the component is padded with
                    // zeros.
                    let mut data = vec![0u8; length];
                    let end = (offset + length).min(component.image.len());
                    data[..end - offset].copy_from_slice(&component.image[offset..end]);
                    sent = end;
                    self.respond(&request, SUCCESS, &data);
                    println!(
                        "pldm host: {}: sent {sent} of {} bytes",
                        component.name,
                        component.image.len()
                    );
                }
                TRANSFER_COMPLETE => {
                    assert_eq!(request.payload, [0], "transfer result");
                    assert_eq!(sent, component.image.len(), "the whole component");
                    transferred = true;
                    self.respond(&request, SUCCESS, &[]);
                    println!("pldm host: {}: transfer complete", component.name);
                }
                VERIFY_COMPLETE => {
                    assert!(transferred, "VerifyComplete follows TransferComplete");
                    assert_eq!(request.payload, [0], "verify result");
                    verified = true;
                    self.respond(&request, SUCCESS, &[]);
                    println!("pldm host: {}: verify complete", component.name);
                }
                APPLY_COMPLETE => {
                    // [apply result][activation methods modification (2)]
                    assert!(verified, "ApplyComplete follows VerifyComplete");
                    assert_eq!(request.payload[0], 0, "apply result");
                    self.respond(&request, SUCCESS, &[]);
                    println!("pldm host: {}: apply complete", component.name);
                    return;
                }
                other => panic!("unexpected request 0x{other:02x} from the device"),
            }
        }
    }

    /// `ActivateFirmware`, with self-contained activation.
    fn activate_firmware(&mut self) {
        let response = self.request(ACTIVATE_FIRMWARE, &[1]);
        // [completion code][estimated time of the activation (2)]
        assert_eq!(response[0], SUCCESS, "ActivateFirmware");
    }
}

#[test]
fn pldm_firmware_update_host_test() {
    // A free port for the model's I3C socket.
    let i3c_port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let files = BootFiles::from_env();
    // The update: the image of //target/veer/tests/pldm/updated and the SoC
    // manifest that authorizes it.
    let update = BootFiles::try_from_env_var("PLDM_UPDATE_BOOT_ENV")
        .expect("failed to load the files of the update");
    let components = [
        Component {
            name: "SoC manifest",
            classification: MANIFEST_CLASSIFICATION,
            identifier: MANIFEST_IDENTIFIER,
            image: update.soc_manifest,
        },
        Component {
            name: "MCU runtime image",
            classification: RUNTIME_CLASSIFICATION,
            identifier: RUNTIME_IDENTIFIER,
            image: update.mcu_fw,
        },
    ];

    let mut params = files.init_params();
    params.i3c_port = Some(i3c_port);
    let mut hw = caliptra_mcu_hw_model::new(params).expect("hardware model failed to boot");

    hw.start_i3c_controller();

    let port = hw.i3c_port().unwrap();
    let target_addr = hw.i3c_address().unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect(addr).expect("failed to connect to the I3C socket");
    let stream = BufferedStream::new(stream);

    // The listeners must be armed before the first request arrives.
    let mut log = String::new();
    wait_for_log(&mut hw, &mut log, READY);
    wait_for_log(&mut hw, &mut log, CONTROL_READY);

    let mut agent = UpdateAgent {
        hw,
        stream,
        log,
        target_addr,
        tag: 0,
        instance: 0,
    };

    agent.query_device_identifiers();
    agent.get_firmware_parameters();
    agent.request_update(components.len() as u16);
    agent.pass_component(&components[0], TRANSFER_START);
    agent.pass_component(&components[1], TRANSFER_END);
    // The device verifies the image against the manifest, so the manifest
    // goes first.
    for component in &components {
        agent.update_component(component);
    }
    agent.activate_firmware();

    // Caliptra resets the MCU into the updated image, which exits with PASS
    // if the reset was for a hitless update.
    wait_for_log(&mut agent.hw, &mut agent.log, UPDATED);
    agent
        .hw
        .step_until_exit_success()
        .expect("the updated image did not exit with PASS");
    println!("pldm host: the updated image ran");
    drop(agent.stream);
}
