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
//! # IPC Pattern
//!
//! Follows the same loop as `services/crypto/server/src/main.rs`:
//!
//! 1. `object_wait(handle, READABLE)` — block until a client sends a request
//! 2. `channel_read(handle)` — read the raw request bytes
//! 3. Parse `I2cRequestHeader`, extract op + payload
//! 4. Dispatch to backend (write / read / write_read / probe / recover)
//! 5. `channel_respond(handle)` — send response header + data
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

use app_i2c_server::handle;

// ---------------------------------------------------------------------------
// Server loop
// ---------------------------------------------------------------------------

fn i2c_server_loop() -> Result<()> {
    pw_log::info!("I2C server starting");

    // SAFETY: Called once at server startup, exclusive peripheral access.
    let mut backend = unsafe { AspeedI2cBackend::new() };

    // Per-controller hardware init (I2CC00, timing, interrupts).
    // Platform init (entry.rs) already ran init_i2c_global() + pinmux.
    // TODO: Initialize all buses this server owns. For now, just bus 0 (I2C1).
    backend.init_bus(0).map_err(|_| pw_status::Error::Internal)?;

    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];

    loop {
        // Block until a client sends a request
        syscall::object_wait(handle::I2C, Signals::READABLE, Instant::MAX)?;

        // Read the request
        let len = syscall::channel_read(handle::I2C, 0, &mut request_buf)?;

        if len < I2cRequestHeader::SIZE {
            // Truncated request — respond with error
            let resp = I2cResponseHeader::error(ResponseCode::ServerError);
            response_buf[..I2cResponseHeader::SIZE].copy_from_slice(&resp.to_bytes());
            syscall::channel_respond(handle::I2C, &response_buf[..I2cResponseHeader::SIZE])?;
            continue;
        }

        // Dispatch and respond
        let response_len =
            dispatch_i2c_op(&request_buf[..len], &mut response_buf, &mut backend);
        syscall::channel_respond(handle::I2C, &response_buf[..response_len])?;
    }
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
            match backend.write(header.bus, header.address, &[]) {
                Ok(()) => encode_success(response, 0),
                Err(code) => encode_error(response, code),
            }
        }

        // ------------------------------------------------------------------
        // RecoverBus: attempt to unstick SDA via clock pulses
        // ------------------------------------------------------------------
        I2cOp::RecoverBus => {
            match backend.recover_bus(header.bus) {
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
