// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! IPC dispatch loop. **One IPC channel per bus** — a client process is wired
//! by configuration to exactly one bus's channel and physically cannot
//! address another. The server owns one driver per bus (master *and* slave)
//! and routes each wake-up to its bus by channel handle.
//!
//! Topology-agnostic, like the usart server's loop: the runtime takes a list
//! of bus bindings + the i2c IRQ, registers them with the WaitGroup, and
//! routes wake-ups via `user_data`.
//!
//! ## Slave-receive notification (thin slice)
//!
//! The WaitGroup multiplexes the per-bus client channels (`READABLE`) **and**
//! the i2c hardware IRQ. On the IRQ: for every notification-armed bus, drain
//! the slave RX into that bus's latch, `interrupt_ack`, then raise
//! `Signals::USER` on that bus's client channel. The client then issues
//! `SlaveReceive`, which returns the latched bytes (status `NoData` if empty).
//! Arm/disarm is `EnableSlaveNotification` / `DisableSlaveNotification`.
//!
//! The **only** kernel-tagged server crate; it wraps the host-buildable
//! `i2c_server::{dispatch, slave::dispatch_slave}` in the Pigweed loop.

#![no_std]

use i2c_api::seam::{
    I2c, I2cBusRecovery, I2cSEvent, I2cSlaveBuffer, I2cSlaveEvent, SevenBitAddress,
};
use i2c_api::{
    I2cError, I2cOp, I2cRequestHeader, I2cResponseHeader, SlaveEventKind, MAX_PAYLOAD_SIZE,
};
use i2c_server::slave::dispatch_slave;
use i2c_server::{dispatch, MAX_BUF_SIZE};
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// One bus the server owns: its dedicated IPC channel, IRQ handle, the driver instance
/// (master + slave), and the per-bus slave-RX notification latch.
pub struct Bus<B> {
    /// IPC channel handle (`channel_handler`) dedicated to this bus.
    pub channel: u32,
    /// IRQ handle for this bus's controller.
    pub irq: u32,
    /// The bus driver — implements both the master and slave seams.
    pub driver: B,
    notif_enabled: bool,
    rx: [u8; MAX_PAYLOAD_SIZE],
    rx_len: usize,
    /// Source address (7-bit) of the master that wrote to us; only valid
    /// when rx_len > 0. Captured on IRQ drain; ignored if not available.
    rx_source: u8,
    /// Event kind that triggered the latch (DataReceived, ReadRequest, Stop).
    /// Only meaningful when rx_len > 0 or notification was armed.
    rx_event_kind: SlaveEventKind,
}

impl<B> Bus<B> {
    pub const fn new(channel: u32, irq: u32, driver: B) -> Self {
        Self {
            channel,
            irq,
            driver,
            notif_enabled: false,
            rx: [0u8; MAX_PAYLOAD_SIZE],
            rx_len: 0,
            rx_source: 0,
            rx_event_kind: SlaveEventKind::DataReceived,
        }
    }
}

fn encode_error(resp: &mut [u8], e: I2cError) -> usize {
    let h = I2cResponseHeader::error(e);
    resp[..I2cResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&h));
    I2cResponseHeader::SIZE
}

fn encode_ok(resp: &mut [u8], payload_len: usize) -> usize {
    let h = I2cResponseHeader::success(payload_len as u16);
    resp[..I2cResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&h));
    I2cResponseHeader::SIZE + payload_len
}

/// Parse just the request header (op + the SlaveReceive max-len in op_count).
fn header(req: &[u8]) -> Option<(I2cOp, usize)> {
    if req.len() < I2cRequestHeader::SIZE {
        return None;
    }
    let h =
        zerocopy::Ref::<_, I2cRequestHeader>::from_bytes(&req[..I2cRequestHeader::SIZE]).ok()?;
    Some((h.operation().ok()?, h.op_count_value()))
}

/// Run the i2c server forever.
///
/// Registers every bus channel (`READABLE`) and each bus's IRQ (`irq_signals`)
/// with `wg`, then loops. `buses` must be non-empty with distinct channels.
pub fn run<B>(wg: u32, irq_signals: Signals, buses: &mut [Bus<B>]) -> !
where
    B: I2c<SevenBitAddress> + I2cSlaveBuffer<SevenBitAddress> + I2cSlaveEvent + I2cBusRecovery,
{
    for bus in buses.iter() {
        if let Err(_) =
            syscall::wait_group_add(wg, bus.channel, Signals::READABLE, bus.channel as usize)
        {
            pw_log::error!("wait_group_add bus channel failed");
        }
        if let Err(_) = syscall::wait_group_add(wg, bus.irq, irq_signals, bus.irq as usize) {
            pw_log::error!("wait_group_add irq failed");
        }
    }

    let mut request_buf = [0u8; MAX_BUF_SIZE];
    let mut response_buf = [0u8; MAX_BUF_SIZE];
    let wait_mask = Signals::READABLE | irq_signals;

    loop {
        let Ok(w) = syscall::object_wait(wg, wait_mask, Instant::MAX) else {
            continue;
        };

        // ---- hardware slave IRQ: drain the armed bus, ack, wake client ----
        // Each bus registered its IRQ with user_data = bus.irq, so check if this
        // signal came from an IRQ (vs. a client channel READABLE event).
        if w.pending_signals.contains(irq_signals) {
            let irq = w.user_data as u32;
            let acked = w.pending_signals & irq_signals;
            if let Some(bus) = buses.iter_mut().find(|b| b.irq == irq) {
                if bus.notif_enabled {
                    match bus.driver.try_next_slave_event() {
                        Ok(Some((kind, _))) => {
                            // Store the actual hardware event kind
                            bus.rx_event_kind = match kind {
                                I2cSEvent::SlaveWrRecvd => SlaveEventKind::DataReceived,
                                I2cSEvent::SlaveRdReq => SlaveEventKind::ReadRequest,
                                I2cSEvent::SlaveStop => SlaveEventKind::Stop,
                                _ => SlaveEventKind::DataReceived,
                            };
                            // For DataReceived, read the buffer; other events have no data
                            if kind == I2cSEvent::SlaveWrRecvd {
                                match bus.driver.read_slave_buffer(&mut bus.rx) {
                                    Ok(n) => {
                                        if n > 0 {
                                            bus.rx_len = n;
                                            // Source address extraction from MCTP-I2C header.
                                            // With AST_I2CC_SLAVE_PKT_SAVE_ADDR set, the hardware
                                            // prepends dest address at byte[0]; the MCTP-I2C header
                                            // carries source at byte[3]: src_addr << 1 | 1.
                                            // Extract it (bits 7:1 = 7-bit address).
                                            if n > 3 {
                                                bus.rx_source = (bus.rx[3] >> 1) & 0x7F;
                                            } else {
                                                bus.rx_source = 0xFF; // Invalid: message too short
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        pw_log::error!("read_slave_buffer failed");
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            pw_log::debug!(
                                "slave IRQ fired but no data ready — spurious or non-data event"
                            );
                        }
                        Err(_) => {
                            pw_log::error!("try_next_slave_event failed");
                        }
                    }
                }
                if let Err(_) = syscall::interrupt_ack(irq, acked) {
                    pw_log::error!("interrupt_ack failed");
                }
                // Wake client on data events (DataReceived with bytes) or transaction
                // boundaries (Stop). ReadRequest without data is deferred (post-demo).
                let should_wake = bus.notif_enabled
                    && (bus.rx_len > 0 || bus.rx_event_kind == SlaveEventKind::Stop);
                if should_wake {
                    // ORs USER onto the bus channel without disturbing READABLE.
                    if let Err(_) = syscall::object_set_peer_user_signal(bus.channel, true) {
                        pw_log::error!("object_set_peer_user_signal failed");
                    }
                }
            }
            continue;
        }

        if !w.pending_signals.contains(Signals::READABLE) {
            continue;
        }
        let channel = w.user_data as u32;
        let Some(bus) = buses.iter_mut().find(|b| b.channel == channel) else {
            continue;
        };
        let Ok(req_len) = syscall::channel_read(channel, 0, &mut request_buf) else {
            continue;
        };
        let req = &request_buf[..req_len];

        let resp_len = match header(req) {
            Some((I2cOp::Transaction, _)) => {
                let n = dispatch(&mut bus.driver, req, &mut response_buf);
                // After a bus-level error, attempt hardware recovery so the
                // next transaction starts clean. The client already has the
                // encoded error; recovery is best-effort (ignore its result).
                if n >= I2cResponseHeader::SIZE {
                    if let Some(rhdr) = zerocopy::Ref::<_, I2cResponseHeader>::from_bytes(
                        &response_buf[..I2cResponseHeader::SIZE],
                    )
                    .ok()
                    {
                        use i2c_api::I2cError;
                        match rhdr.error_code() {
                            I2cError::Bus | I2cError::ArbitrationLoss | I2cError::Timeout => {
                                let _ = bus.driver.recover_bus();
                            }
                            _ => {}
                        }
                    }
                }
                n
            }
            Some((I2cOp::ConfigureSlave | I2cOp::EnableSlave | I2cOp::DisableSlave, _)) => {
                dispatch_slave(&mut bus.driver, req, &mut response_buf)
            }
            Some((I2cOp::EnableSlaveNotification, _)) => {
                bus.notif_enabled = true;
                encode_ok(&mut response_buf, 0)
            }
            Some((I2cOp::DisableSlaveNotification, _)) => {
                bus.notif_enabled = false;
                bus.rx_len = 0;
                bus.rx_source = 0;
                bus.rx_event_kind = SlaveEventKind::DataReceived;
                encode_ok(&mut response_buf, 0)
            }
            Some((I2cOp::SlaveReceive, max_len)) => {
                if bus.rx_len == 0 {
                    encode_error(&mut response_buf, I2cError::NoData)
                } else {
                    // Response payload: [kind (1), source_addr (1), data (0..max_len)]
                    let cap = response_buf.len() - I2cResponseHeader::SIZE;
                    let metadata_size = 2; // kind + source
                    if cap < metadata_size {
                        encode_error(&mut response_buf, I2cError::BufferTooSmall)
                    } else {
                        let data_cap = cap - metadata_size;
                        let n = bus.rx_len.min(max_len).min(data_cap);
                        let payload_offset = I2cResponseHeader::SIZE;
                        response_buf[payload_offset] = bus.rx_event_kind as u8;
                        response_buf[payload_offset + 1] = bus.rx_source;
                        response_buf[payload_offset + 2..payload_offset + 2 + n]
                            .copy_from_slice(&bus.rx[..n]);
                        bus.rx_len = 0;
                        encode_ok(&mut response_buf, metadata_size + n)
                    }
                }
            }
            // NOTE: not needed for MCTP (master-write only). For testing slave-TX patterns.
            Some((I2cOp::SlaveSetResponse, _)) => {
                let hdr_end = I2cRequestHeader::SIZE;
                if req.len() < hdr_end {
                    encode_error(&mut response_buf, I2cError::InvalidOperation)
                } else {
                    let payload = &req[hdr_end..];
                    match bus.driver.write_slave_response(payload) {
                        Ok(()) => encode_ok(&mut response_buf, 0),
                        Err(_) => encode_error(&mut response_buf, I2cError::InternalError),
                    }
                }
            }
            None => encode_error(&mut response_buf, I2cError::InvalidOperation),
        };
        if let Err(_) = syscall::channel_respond(channel, &response_buf[..resp_len]) {
            pw_log::error!("channel_respond failed");
        }
    }
}
