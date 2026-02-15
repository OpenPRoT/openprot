// Licensed under the Apache-2.0 license

//! I2C IPC Client
//!
//! This crate provides an I2cClient implementation that uses IPC to communicate
//! with the I2C server via Pigweed's `userspace::syscall::channel_transact`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use i2c_client::IpcI2cClient;
//! use i2c_api::{BusIndex, I2cAddress, I2cClient};
//!
//! // Create client bound to the I2C server channel (handle from app's handle module)
//! let mut client = IpcI2cClient::new(handle::I2C);
//!
//! let addr = I2cAddress::new(0x48)?;
//! let mut buf = [0u8; 2];
//! client.write_read(BusIndex::BUS_0, addr, &[], &mut buf)?;
//! ```

#![no_std]
#![warn(missing_docs)]

use i2c_api::{
    wire::{
        self, encode_probe_request, encode_read_request, encode_write_read_request,
        encode_write_request, I2cResponseHeader,
    },
    BusIndex, I2cAddress, I2cClient, I2cError, I2cErrorKind, NoAcknowledgeSource, Operation,
    ResponseCode,
};

use userspace::syscall;
use userspace::time::Instant;

// Re-export wire module for advanced users
pub use i2c_api::wire as protocol;

/// I2C client that communicates with the I2C server over Pigweed IPC
///
/// This client implements the `I2cClient` trait and uses the wire protocol
/// to encode/decode messages sent via `channel_transact`.
pub struct IpcI2cClient {
    handle: u32,
    request_buf: [u8; wire::MAX_REQUEST_SIZE],
    response_buf: [u8; wire::MAX_RESPONSE_SIZE],
}

impl IpcI2cClient {
    /// Create a new IPC I2C client bound to the given channel handle
    ///
    /// # Arguments
    /// * `handle` - Channel handle from the application's handle module (e.g., `handle::I2C`)
    pub fn new(handle: u32) -> Self {
        Self {
            handle,
            request_buf: [0u8; wire::MAX_REQUEST_SIZE],
            response_buf: [0u8; wire::MAX_RESPONSE_SIZE],
        }
    }

    /// Get the channel handle
    pub fn handle(&self) -> u32 {
        self.handle
    }

    /// Send request and receive response via IPC
    fn send_recv(&mut self, req_len: usize) -> Result<usize, I2cError> {
        syscall::channel_transact(
            self.handle,
            &self.request_buf[..req_len],
            &mut self.response_buf,
            Instant::MAX,
        )
        .map_err(|_| I2cError::from_code(ResponseCode::ServerError))
    }

    /// Decode a response and check for errors
    fn decode_response(&self, len: usize) -> Result<&[u8], I2cError> {
        if len < I2cResponseHeader::SIZE {
            return Err(I2cError::from_code(ResponseCode::ServerError));
        }

        let header = wire::decode_response_header(&self.response_buf[..len])
            .ok_or_else(|| I2cError::from_code(ResponseCode::ServerError))?;

        if !header.is_success() {
            return Err(response_to_error(header.response_code()));
        }

        wire::get_response_data(&self.response_buf[..len], &header)
            .ok_or_else(|| I2cError::from_code(ResponseCode::ServerError))
    }
}

/// Convert a ResponseCode to an I2cError
fn response_to_error(code: ResponseCode) -> I2cError {
    let kind = match code {
        ResponseCode::NoDevice => I2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Address),
        ResponseCode::NackData => I2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
        ResponseCode::ArbitrationLost => I2cErrorKind::ArbitrationLoss,
        ResponseCode::BusStuck => I2cErrorKind::Bus,
        ResponseCode::Timeout => I2cErrorKind::Other,
        _ => I2cErrorKind::Other,
    };
    I2cError::new(code, kind)
}

impl embedded_hal::i2c::ErrorType for IpcI2cClient {
    type Error = I2cError;
}

impl I2cClient for IpcI2cClient {
    fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error> {
        // Handle different operation types
        if write.is_empty() && read.is_empty() {
            // Probe operation
            let req_len = encode_probe_request(&mut self.request_buf, bus.value(), address.value())
                .ok_or_else(|| I2cError::from_code(ResponseCode::BufferTooSmall))?;

            let resp_len = self.send_recv(req_len)?;
            let _ = self.decode_response(resp_len)?;
            return Ok(0);
        }

        if write.is_empty() {
            // Read only
            let req_len = encode_read_request(
                &mut self.request_buf,
                bus.value(),
                address.value(),
                read.len() as u16,
            )
            .ok_or_else(|| I2cError::from_code(ResponseCode::BufferTooSmall))?;

            let resp_len = self.send_recv(req_len)?;
            let data = self.decode_response(resp_len)?;
            let copy_len = core::cmp::min(data.len(), read.len());
            read[..copy_len].copy_from_slice(&data[..copy_len]);
            return Ok(copy_len);
        }

        if read.is_empty() {
            // Write only
            let req_len =
                encode_write_request(&mut self.request_buf, bus.value(), address.value(), write)
                    .ok_or_else(|| I2cError::from_code(ResponseCode::BufferTooSmall))?;

            let resp_len = self.send_recv(req_len)?;
            let _ = self.decode_response(resp_len)?;
            return Ok(0);
        }

        // Write-read
        let req_len = encode_write_read_request(
            &mut self.request_buf,
            bus.value(),
            address.value(),
            write,
            read.len() as u16,
        )
        .ok_or_else(|| I2cError::from_code(ResponseCode::BufferTooSmall))?;

        let resp_len = self.send_recv(req_len)?;
        let data = self.decode_response(resp_len)?;
        let copy_len = core::cmp::min(data.len(), read.len());
        read[..copy_len].copy_from_slice(&data[..copy_len]);
        Ok(copy_len)
    }

    fn transaction(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        // For now, handle simple cases by converting to write_read
        // A full implementation would encode all operations in a single transaction message
        for op in operations.iter_mut() {
            match op {
                Operation::Write(data) => {
                    self.write_read(bus, address, data, &mut [])?;
                }
                Operation::Read(buffer) => {
                    self.write_read(bus, address, &[], buffer)?;
                }
            }
        }
        Ok(())
    }
}
