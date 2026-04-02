// Licensed under the Apache-2.0 license

//! I2C Server — IPC Dispatch Loop
//!
//! Userspace service that receives I2C requests over a Pigweed IPC channel,
//! dispatches them to the AST1060 backend, and responds with results.
//!
//! # Architecture
//!
//! ```text
//! ┌─ Client ──────────────────────────┐
//! │ channel_transact(request)         │
//! └──────────────┬────────────────────┘
//!                │ IPC channel
//!                ▼
//! ┌─ This Server ─────────────────────┐
//! │ object_wait(READABLE)             │
//! │ channel_read → I2cRequestHeader   │
//! │ dispatch_i2c_op(op, backend)      │
//! │ channel_respond ← I2cResponseHdr  │
//! └──────────────┬────────────────────┘
//!                │ from_initialized()
//!                ▼
//! ┌─ AspeedI2cBackend ────────────────┐
//! │ Ast1060I2c::write/read/write_read │
//! └───────────────────────────────────┘
//! ```
//!
//! # WaitGroup + IRQ Pattern
//!
//! A WaitGroup multiplexes two event sources:
//!
//! - `user_data=0`: IPC from client — `channel_read`, dispatch, `channel_respond`
//! - `user_data=1`: I2C2 hardware interrupt — `drain_slave_rx`, `interrupt_ack`,
//!   `raise_peer_user_signal`
//!
//! On the IPC path, `channel_read` returns immediately (channel is already
//! `READABLE` when the WaitGroup fires). No async syscalls needed.
//!
//! # Handle Binding
//!
//! The IPC handle is provided by the `app_package` Bazel rule, which generates
//! `app_i2c_server::handle::I2C` from the system configuration.

#![no_main]
#![no_std]

use i2c_api::wire::{I2cOp, I2cRequestHeader, I2cResponseHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use i2c_api::ResponseCode;
use i2c_backend_aspeed::AspeedI2cBackend;

use pw_status::Result;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use app_i2c_server::{handle, signals};

// WaitGroup token constants — identify which event source woke the server.
const WTOKEN_IPC: usize = 0; // IPC channel became READABLE
const WTOKEN_IRQ: usize = 1; // I2C2 hardware interrupt

// ---------------------------------------------------------------------------
// Server loop
// ---------------------------------------------------------------------------

fn i2c_server_loop() -> Result<()> {
    pw_log::info!("I2C server starting");

    // SAFETY: Called once at server startup, exclusive peripheral access.
    let mut backend = unsafe { AspeedI2cBackend::new() };

    // Per-controller hardware init (I2CC00, timing, interrupts).
    // Platform init (entry.rs) already ran init_i2c_global() + pinmux.
    // TODO: Initialize all buses this server owns. For now, just bus 2 (I2C1 in hardware).
    backend.init_bus(2).map_err(|_| pw_status::Error::Internal)?;

    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];

    // Register both event sources with the WaitGroup.
    syscall::wait_group_add(handle::WG, handle::I2C, Signals::READABLE, WTOKEN_IPC)?;
    syscall::wait_group_add(handle::WG, handle::I2C2_IRQ, signals::I2C2, WTOKEN_IRQ)?;

    loop {
        let wait_return = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX)?;

        if wait_return.user_data == WTOKEN_IRQ {
            // Hardware I2C2 slave interrupt: drain data into flat buffers.
            // Only signal the client if bytes were actually received to avoid
            // spurious wakeups on non-data events (Stop, WriteRequest, etc.).
            if handle_i2c_interrupt(&mut backend) {
                let _ = syscall::raise_peer_user_signal(handle::I2C);
            }
            if let Err(e) = syscall::interrupt_ack(handle::I2C2_IRQ, signals::I2C2) {
                pw_log::warn!("interrupt_ack failed: {}", e as u32);
            }
        } else {
            // IPC request from client — channel_read returns immediately since
            // the channel was already READABLE when the WaitGroup fired.
            let len = syscall::channel_read(handle::I2C, 0, &mut request_buf)?;

            if len < I2cRequestHeader::SIZE {
                let resp = I2cResponseHeader::error(ResponseCode::ServerError);
                response_buf[..I2cResponseHeader::SIZE].copy_from_slice(&resp.to_bytes());
                syscall::channel_respond(handle::I2C, &response_buf[..I2cResponseHeader::SIZE])?;
                continue;
            }

            let response_len = dispatch_i2c_op(
                &request_buf[..len],
                &mut response_buf,
                &mut backend,
            );
            syscall::channel_respond(handle::I2C, &response_buf[..response_len])?;
        }
    }
}

// ---------------------------------------------------------------------------
// Interrupt handler
// ---------------------------------------------------------------------------

/// Handle a hardware I2C slave interrupt.
///
/// Called once per interrupt event (no polling loop). Drains any received
/// data into the per-bus flat buffer for every notification-enabled bus.
/// The backend gates each call: buses without notification enabled return
/// immediately.
///
/// Returns `true` if any bus had data latched (caller should signal the
/// client), `false` for spurious interrupts (Stop, WriteRequest, etc.).
fn handle_i2c_interrupt(backend: &mut AspeedI2cBackend) -> bool {
    let mut any_data = false;
    for bus in 0..14u8 {
        if let Ok(n) = backend.drain_slave_rx(bus) {
            if n > 0 {
                any_data = true;
            }
        }
    }
    any_data
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Decode request header, dispatch to backend, encode response.
///
/// Read operations write their data directly into `response` after the
/// response header (offset [`I2cResponseHeader::SIZE`]), avoiding an
/// extra copy.
fn dispatch_i2c_op(
    request: &[u8],
    response: &mut [u8],
    backend: &mut AspeedI2cBackend,
) -> usize {
    // Parse header
    let Some(header) = I2cRequestHeader::from_bytes(request) else {
        return encode_error(response, ResponseCode::ServerError);
    };

    let Some(op) = header.operation() else {
        return encode_error(response, ResponseCode::ServerError);
    };

    let payload = &request[I2cRequestHeader::SIZE..];

    match op {
        // ------------------------------------------------------------------
        // Write: header.write_len bytes from payload → device
        // ------------------------------------------------------------------
        I2cOp::Write => {
            pw_log::info!("I2C dispatch write");
            let wlen = header.write_len as usize;
            if payload.len() < wlen {
                return encode_error(response, ResponseCode::BufferTooSmall);
            }
            match backend.write(header.bus, header.address, &payload[..wlen]) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // Read: header.read_len bytes from device → response payload
        // ------------------------------------------------------------------
        I2cOp::Read => {
            pw_log::info!("I2C dispatch read");
            let rlen = header.read_len as usize;
            let avail = response.len().saturating_sub(I2cResponseHeader::SIZE);
            if rlen > avail {
                return encode_error(response, ResponseCode::BufferTooLarge);
            }
            let read_buf =
                &mut response[I2cResponseHeader::SIZE..I2cResponseHeader::SIZE + rlen];
            match backend.read(header.bus, header.address, read_buf) {
                Ok(()) => encode_success(response, rlen),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // WriteRead: write then read with repeated START
        // ------------------------------------------------------------------
        I2cOp::WriteRead => {
            pw_log::info!("I2C dispatch writeread");
            let wlen = header.write_len as usize;
            let rlen = header.read_len as usize;
            if payload.len() < wlen {
                return encode_error(response, ResponseCode::BufferTooSmall);
            }
            let avail = response.len().saturating_sub(I2cResponseHeader::SIZE);
            if rlen > avail {
                return encode_error(response, ResponseCode::BufferTooLarge);
            }
            let write_data = &payload[..wlen];
            let read_buf =
                &mut response[I2cResponseHeader::SIZE..I2cResponseHeader::SIZE + rlen];
            match backend.write_read(header.bus, header.address, write_data, read_buf) {
                Ok(()) => encode_success(response, rlen),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // Probe: write 0 bytes — ACK means device present
        // ------------------------------------------------------------------
        I2cOp::Probe => {
            pw_log::info!("I2C dispatch probe");


            match backend.write(header.bus, header.address, &[]) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // RecoverBus: attempt to unstick SDA via clock pulses
        // ------------------------------------------------------------------
        I2cOp::RecoverBus => {
            pw_log::info!("I2C dispatch recover bus");
            match backend.recover_bus(header.bus) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // ConfigureSlave: set slave address on a bus
        // ------------------------------------------------------------------
        I2cOp::ConfigureSlave => {
            pw_log::info!("I2C dispatch configure slave");
            match backend.configure_slave(header.bus, header.address) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // EnableSlave: activate slave receive mode
        // ------------------------------------------------------------------
        I2cOp::EnableSlave => {
            pw_log::info!("I2C dispatch enable slave");
            match backend.enable_slave(header.bus) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // DisableSlave: deactivate slave receive mode
        // ------------------------------------------------------------------
        I2cOp::DisableSlave => {
            pw_log::info!("I2C dispatch disable slave");
            match backend.disable_slave(header.bus) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // SlaveReceive: fetch buffered data (non-blocking, interrupt-driven)
        // ------------------------------------------------------------------
        I2cOp::SlaveReceive => {
            pw_log::info!("I2C dispatch slave receive");
            let rlen = header.read_len as usize;
            let avail = response.len().saturating_sub(I2cResponseHeader::SIZE);
            if rlen > avail {
                return encode_error(response, ResponseCode::BufferTooLarge);
            }
            let buf = &mut response[I2cResponseHeader::SIZE..I2cResponseHeader::SIZE + rlen];
            match backend.get_buffered_slave_message(header.bus, buf) {
                Ok(n) => encode_success(response, n),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // SlaveSetResponse: pre-load TX buffer for next master read
        // ------------------------------------------------------------------
        I2cOp::SlaveSetResponse => {
            // pw_log::info!("I2C dispatch slave set response");
            let wlen = header.write_len as usize;
            if payload.len() < wlen {
                return encode_error(response, ResponseCode::BufferTooSmall);
            }
            match backend.slave_set_response(header.bus, &payload[..wlen]) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // SlaveWaitEvent: block until next slave event, return kind + data
        //
        // Response payload layout:
        //   byte 0:    SlaveEventKind as u8
        //   bytes 1..: received data (only for DataReceived events)
        // ------------------------------------------------------------------
        I2cOp::SlaveWaitEvent => {
            let max_rx = header.read_len as usize;
            // Reserve space for event-kind byte + rx data.
            let avail = response.len().saturating_sub(I2cResponseHeader::SIZE + 1);
            let rx_cap = max_rx.min(avail);
            let rx_buf = &mut response[I2cResponseHeader::SIZE + 1..I2cResponseHeader::SIZE + 1 + rx_cap];
            match backend.slave_wait_event(header.bus, rx_buf) {
                Ok((kind, rx_len)) => {
                    let total = 1 + rx_len;
                    response[I2cResponseHeader::SIZE] = kind as u8;
                    encode_success(response, total)
                }
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // EnableSlaveNotification: arm interrupt-driven receive for a bus
        // ------------------------------------------------------------------
        I2cOp::EnableSlaveNotification => {
            pw_log::info!("I2C dispatch enable slave notification");
            match backend.enable_slave_notification(header.bus) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // DisableSlaveNotification: disarm interrupt-driven receive for a bus
        // ------------------------------------------------------------------
        I2cOp::DisableSlaveNotification => {
            pw_log::info!("I2C dispatch disable slave notification");
            match backend.disable_slave_notification(header.bus) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // Not yet implemented
        // ------------------------------------------------------------------
        I2cOp::ConfigureSpeed | I2cOp::Transaction => {
            encode_error(response, ResponseCode::ServerError)
        }
    }
}

// ---------------------------------------------------------------------------
// Response encoding
// ---------------------------------------------------------------------------

/// Encode an error response (header only, no payload).
fn encode_error(response: &mut [u8], code: ResponseCode) -> usize {
    let header = I2cResponseHeader::error(code);
    response[..I2cResponseHeader::SIZE].copy_from_slice(&header.to_bytes());
    I2cResponseHeader::SIZE
}

/// Encode a success response.
///
/// For read operations, the caller has already written the data into
/// `response[I2cResponseHeader::SIZE..]` before calling this function.
fn encode_success(response: &mut [u8], data_len: usize) -> usize {
    let header = I2cResponseHeader::success(data_len as u16);
    response[..I2cResponseHeader::SIZE].copy_from_slice(&header.to_bytes());
    I2cResponseHeader::SIZE + data_len
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[entry]
fn entry() -> ! {
    if let Err(e) = i2c_server_loop() {
        pw_log::error!("I2C server error: {}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
