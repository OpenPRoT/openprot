// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared host-side harness for VeeR emulator I3C tests.
//!
//! Wire protocol (see caliptra-mcu-sw common/testing/src/i3c_socket_server.rs):
//! - Host -> emulator: `to_addr: u8` + two LE u32 command words + data bytes.
//!   `rnw` is bit 29 of the 64-bit command; `data_length` is bits 63:48.
//! - Emulator -> host: `ibi: u8`, `from_addr: u8`, LE u32 response
//!   descriptor (`data_length` in bits 15:0), then data bytes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const I3C_HOST: &str = "127.0.0.1";
pub const I3C_PORT: u16 = 65534;
pub const DEFAULT_TARGET_ADDR: u8 = 0x08;

pub fn crc8_smbus(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &value in data {
        crc ^= value;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Payload followed by the SMBus PEC over (write-address byte + payload).
pub fn body_with_pec(target_addr: u8, payload: &[u8]) -> Vec<u8> {
    let mut pec_input = Vec::with_capacity(1 + payload.len());
    pec_input.push(target_addr << 1);
    pec_input.extend_from_slice(payload);
    let pec = crc8_smbus(&pec_input);
    let mut body = Vec::with_capacity(payload.len() + 1);
    body.extend_from_slice(payload);
    body.push(pec);
    body
}

pub fn make_private_write_header(target_addr: u8, data_len: u16) -> [u8; 9] {
    // rnw (bit 29) = 0; data_length in bits 63:48 (word1 bits 31:16).
    let cmd_word0: u32 = 0;
    let cmd_word1: u32 = (data_len as u32) << 16;
    let mut out = [0u8; 9];
    out[0] = target_addr;
    out[1..5].copy_from_slice(&cmd_word0.to_le_bytes());
    out[5..9].copy_from_slice(&cmd_word1.to_le_bytes());
    out
}

pub fn make_private_read_header(target_addr: u8) -> [u8; 9] {
    // rnw (bit 29) = 1; data_length = 0 (the target reports its own length).
    let cmd_word0: u32 = 1 << 29;
    let cmd_word1: u32 = 0;
    let mut out = [0u8; 9];
    out[0] = target_addr;
    out[1..5].copy_from_slice(&cmd_word0.to_le_bytes());
    out[5..9].copy_from_slice(&cmd_word1.to_le_bytes());
    out
}

pub fn connect_i3c_socket(timeout: Duration) -> Result<TcpStream, String> {
    let deadline = Instant::now() + timeout;
    let mut last_err: Option<String> = None;
    while Instant::now() < deadline {
        match TcpStream::connect((I3C_HOST, I3C_PORT)) {
            Ok(s) => return Ok(s),
            Err(e) => {
                last_err = Some(e.to_string());
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Err(format!(
        "timed out connecting to {}:{} ({})",
        I3C_HOST,
        I3C_PORT,
        last_err.unwrap_or_else(|| "unknown error".to_string())
    ))
}

pub fn send_private_write_on_stream(
    stream: &mut TcpStream,
    target_addr: u8,
    payload: &[u8],
) -> Result<(), String> {
    let body = body_with_pec(target_addr, payload);
    let header = make_private_write_header(target_addr, body.len() as u16);
    let mut frame = Vec::with_capacity(header.len() + body.len());
    frame.extend_from_slice(&header);
    frame.extend_from_slice(&body);
    stream
        .write_all(&frame)
        .map_err(|e| format!("failed writing I3C private-write frame: {e}"))?;
    println!(
        "I3C HOST TRACE: wrote frame to addr=0x{target_addr:02x} header={header:02x?} body={body:02x?}"
    );
    Ok(())
}

/// Send a private write whose data block is transmitted verbatim — no PEC is
/// appended. The caller owns the entire framing, including any trailing PEC.
///
/// For MCTP-over-I3C (DSP0233) use [`send_private_write_on_stream`] instead: the
/// frame is the MCTP packet followed by a PEC over the write address byte and the
/// packet, which is what it appends. This raw variant is for tests that need a
/// deliberately malformed or non-standard frame.
pub fn send_private_write_raw_on_stream(
    stream: &mut TcpStream,
    target_addr: u8,
    data: &[u8],
) -> Result<(), String> {
    let header = make_private_write_header(target_addr, data.len() as u16);
    let mut frame = Vec::with_capacity(header.len() + data.len());
    frame.extend_from_slice(&header);
    frame.extend_from_slice(data);
    stream
        .write_all(&frame)
        .map_err(|e| format!("failed writing raw I3C private-write frame: {e}"))?;
    println!("I3C HOST TRACE: wrote raw frame to addr=0x{target_addr:02x} data={data:02x?}");
    Ok(())
}

pub fn send_private_read_on_stream(stream: &mut TcpStream, target_addr: u8) -> Result<(), String> {
    let header = make_private_read_header(target_addr);
    stream
        .write_all(&header)
        .map_err(|e| format!("failed writing I3C private-read command: {e}"))?;
    println!("I3C HOST TRACE: wrote read command to addr=0x{target_addr:02x}");
    Ok(())
}

pub struct OutgoingPacket {
    pub ibi: u8,
    pub from_addr: u8,
    pub data: Vec<u8>,
}

pub fn read_outgoing_packet<R: Read>(reader: &mut R) -> Result<OutgoingPacket, String> {
    let mut header = [0u8; 6];
    reader
        .read_exact(&mut header)
        .map_err(|e| format!("failed reading outgoing packet header: {e}"))?;
    let descriptor = u32::from_le_bytes([header[2], header[3], header[4], header[5]]);
    let len = (descriptor & 0xffff) as usize;
    let mut data = vec![0u8; len];
    reader
        .read_exact(&mut data)
        .map_err(|e| format!("failed reading outgoing packet data ({len} bytes): {e}"))?;
    Ok(OutgoingPacket {
        ibi: header[0],
        from_addr: header[1],
        data,
    })
}

/// The MCTP packet inside the data a target returned for a private read.
///
/// DSP0233 frames it as the packet followed by a PEC over the read address byte
/// `(target_addr << 1) | 1` and the packet. Nothing precedes the packet. Fails if
/// the data is empty, is only a PEC, or the PEC does not match.
pub fn packet_from_read(target_addr: u8, data: &[u8]) -> Result<&[u8], String> {
    let (received, packet) = data
        .split_last()
        .ok_or_else(|| "empty read frame".to_string())?;
    if packet.is_empty() {
        return Err("read frame holds a PEC but no packet".to_string());
    }
    let mut pec_input = Vec::with_capacity(1 + packet.len());
    pec_input.push((target_addr << 1) | 1);
    pec_input.extend_from_slice(packet);
    let expected = crc8_smbus(&pec_input);
    if *received != expected {
        return Err(format!(
            "bad PEC on read from 0x{target_addr:02x}: got {received:#04x}, expected {expected:#04x}"
        ));
    }
    Ok(packet)
}

pub fn extract_target_addr(line: &str) -> Option<u8> {
    let marker = "target DynamicI3cAddress(";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find(')')?;
    let parsed = rest[..end].parse::<u16>().ok()?;
    u8::try_from(parsed).ok()
}

fn resolve_runner_path(runner_rel_path: &str) -> PathBuf {
    let srcdir = std::env::var("TEST_SRCDIR").expect("missing TEST_SRCDIR environment variable");
    let workspace =
        std::env::var("TEST_WORKSPACE").expect("missing TEST_WORKSPACE environment variable");
    let candidate = Path::new(&srcdir).join(&workspace).join(runner_rel_path);
    if candidate.exists() {
        return candidate;
    }
    panic!(
        "unable to locate emulator runner at {:?}; TEST_SRCDIR={:?}, TEST_WORKSPACE={:?}",
        candidate,
        std::env::var("TEST_SRCDIR").ok(),
        std::env::var("TEST_WORKSPACE").ok(),
    );
}

fn resolve_runner_cwd(runner: &Path) -> PathBuf {
    if let (Ok(srcdir), Ok(workspace)) = (
        std::env::var("TEST_SRCDIR"),
        std::env::var("TEST_WORKSPACE"),
    ) {
        let root = Path::new(&srcdir).join(&workspace);
        if root.exists() {
            return root;
        }
    }
    runner
        .parent()
        .expect("runner path has no parent directory")
        .to_path_buf()
}

/// A spawned emulator runner with background stdout/stderr watchers.
pub struct Runner {
    child: Child,
    ready: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
    target_addr: Arc<AtomicU8>,
    stdout_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
}

impl Runner {
    /// Spawn `runner_rel_path` (workspace-relative runfiles path). `ready`
    /// flips when `ready_marker` appears on either stream; `exited` flips
    /// when stdout reaches EOF.
    pub fn spawn(runner_rel_path: &str, ready_marker: &'static str) -> Self {
        let runner = resolve_runner_path(runner_rel_path);
        let cwd = resolve_runner_cwd(&runner);
        let mut child = Command::new(&runner)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn emulator runner");

        let ready = Arc::new(AtomicBool::new(false));
        let exited = Arc::new(AtomicBool::new(false));
        let target_addr = Arc::new(AtomicU8::new(DEFAULT_TARGET_ADDR));

        let stdout = child
            .stdout
            .take()
            .expect("failed to capture runner stdout");
        let stderr = child
            .stderr
            .take()
            .expect("failed to capture runner stderr");

        let watch = |reader: Box<dyn Read + Send>,
                     to_stderr: bool,
                     ready: Arc<AtomicBool>,
                     exited: Option<Arc<AtomicBool>>,
                     target_addr: Arc<AtomicU8>| {
            move || {
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                loop {
                    line.clear();
                    let n = reader
                        .read_line(&mut line)
                        .expect("failed to read runner output");
                    if n == 0 {
                        break;
                    }
                    if to_stderr {
                        eprint!("{line}");
                    } else {
                        print!("{line}");
                    }
                    if let Some(addr) = extract_target_addr(&line) {
                        target_addr.store(addr, Ordering::Relaxed);
                    }
                    if line.contains(ready_marker) {
                        ready.store(true, Ordering::Relaxed);
                    }
                }
                if let Some(exited) = exited {
                    exited.store(true, Ordering::Relaxed);
                }
            }
        };

        let stdout_thread = thread::spawn(watch(
            Box::new(stdout),
            false,
            Arc::clone(&ready),
            Some(Arc::clone(&exited)),
            Arc::clone(&target_addr),
        ));
        let stderr_thread = thread::spawn(watch(
            Box::new(stderr),
            true,
            Arc::clone(&ready),
            None,
            Arc::clone(&target_addr),
        ));

        Runner {
            child,
            ready,
            exited,
            target_addr,
            stdout_thread: Some(stdout_thread),
            stderr_thread: Some(stderr_thread),
        }
    }

    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    pub fn target_addr(&self) -> u8 {
        self.target_addr.load(Ordering::Relaxed)
    }

    /// Block until the ready marker is seen. Returns false if the runner
    /// exits first or `timeout` elapses.
    pub fn wait_ready(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.ready() {
                return true;
            }
            if self.exited() {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }

    /// Join the watcher threads and reap the child.
    pub fn wait(mut self) -> ExitStatus {
        if let Some(t) = self.stdout_thread.take() {
            t.join().expect("failed to join stdout watcher");
        }
        if let Some(t) = self.stderr_thread.take() {
            t.join().expect("failed to join stderr watcher");
        }
        self.child.wait().expect("failed to wait for runner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn crc8_matches_known_value() {
        // Address 0x08 (write byte 0x10) + payload [1,2,3,4] -> PEC 0xd1,
        // as observed on the wire in the i3c_smoke test.
        assert_eq!(crc8_smbus(&[0x10, 0x01, 0x02, 0x03, 0x04]), 0xd1);
    }

    #[test]
    fn body_appends_pec() {
        assert_eq!(
            body_with_pec(0x08, &[0x01, 0x02, 0x03, 0x04]),
            vec![0x01, 0x02, 0x03, 0x04, 0xd1]
        );
    }

    #[test]
    fn write_header_layout() {
        // data_length lands in word1 bits 31:16.
        assert_eq!(
            make_private_write_header(0x08, 5),
            [0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00]
        );
    }

    #[test]
    fn read_header_sets_rnw_bit_29() {
        assert_eq!(
            make_private_read_header(0x08),
            [0x08, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn parses_outgoing_packet() {
        // ibi=0, from_addr=8, descriptor len=3, data [aa,bb,cc].
        let bytes = [0x00, 0x08, 0x03, 0x00, 0x00, 0x00, 0xaa, 0xbb, 0xcc];
        let mut cursor = Cursor::new(&bytes[..]);
        let pkt = read_outgoing_packet(&mut cursor).unwrap();
        assert_eq!(pkt.ibi, 0);
        assert_eq!(pkt.from_addr, 8);
        assert_eq!(pkt.data, vec![0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn packet_from_read_strips_a_read_direction_pec() {
        let packet = [0x01, 0x0a, 0x08, 0xc8, 0x01, 0x02];
        let mut pec_input = vec![(0x08u8 << 1) | 1];
        pec_input.extend_from_slice(&packet);
        let mut data = packet.to_vec();
        data.push(crc8_smbus(&pec_input));

        assert_eq!(packet_from_read(0x08, &data).unwrap(), &packet);
    }

    #[test]
    fn packet_from_read_rejects_bad_framing() {
        let packet = [0x01, 0x0a, 0x08, 0xc8];

        // PEC computed for a write (address byte with R/W# = 0) must not pass.
        let mut write_pec = vec![0x08u8 << 1];
        write_pec.extend_from_slice(&packet);
        let mut data = packet.to_vec();
        data.push(crc8_smbus(&write_pec));
        assert!(packet_from_read(0x08, &data).is_err());

        // Empty, and a PEC with no packet.
        assert!(packet_from_read(0x08, &[]).is_err());
        assert!(packet_from_read(0x08, &[0x00]).is_err());
    }

    #[test]
    fn extracts_dynamic_address() {
        assert_eq!(
            extract_target_addr("i3c target DynamicI3cAddress(9) attached"),
            Some(9)
        );
        assert_eq!(extract_target_addr("no address here"), None);
    }
}
