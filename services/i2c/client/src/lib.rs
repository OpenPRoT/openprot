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
        self, encode_configure_slave_request, encode_disable_slave_request,
        encode_enable_slave_request,
        encode_enable_slave_notification_request, encode_probe_request, encode_read_request,
        encode_slave_receive_request, encode_slave_set_response_request,
        encode_slave_wait_event_request, encode_write_read_request, encode_write_request,
        I2cResponseHeader, WireError,
    },
    BusIndex, I2cAddress, I2cClient, I2cError, I2cErrorKind, I2cTargetClient, NoAcknowledgeSource,
    Operation, ResponseCode, SlaveEventKind, TargetMessage, TARGET_MESSAGE_MAX_LEN,
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
            .map_err(wire_to_i2c_error)?;

        if !header.is_success() {
            return Err(response_to_error(header.response_code()));
        }

        wire::get_response_data(&self.response_buf[..len], &header)
            .map_err(wire_to_i2c_error)
    }
}

/// Convert a WireError to an I2cError
fn wire_to_i2c_error(e: WireError) -> I2cError {
    let code = match e {
        WireError::BufferTooSmall => ResponseCode::BufferTooSmall,
        WireError::PayloadTooLarge => ResponseCode::BufferTooLarge,
        WireError::InvalidOpcode(_) => ResponseCode::ServerError,
        WireError::Truncated => ResponseCode::ServerError,
    };
    I2cError::from_code(code)
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
                .map_err(wire_to_i2c_error)?;

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
            .map_err(wire_to_i2c_error)?;

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
                    .map_err(wire_to_i2c_error)?;

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
        .map_err(wire_to_i2c_error)?;

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

impl IpcI2cClient {
    /// Pre-load data the slave will send when the master reads from us.
    ///
    /// Must be called before [`slave_wait_event`] if a read response is needed.
    pub fn slave_set_response(&mut self, bus: BusIndex, data: &[u8]) -> Result<(), I2cError> {
        let req_len =
            encode_slave_set_response_request(&mut self.request_buf, bus.value(), data)
                .map_err(wire_to_i2c_error)?;
        let resp_len = self.send_recv(req_len)?;
        let _ = self.decode_response(resp_len)?;
        Ok(())
    }

    /// Block until the next slave event on `bus`.
    ///
    /// Returns the event kind and, for `DataReceived`, the received bytes
    /// written into `rx_buf`. The returned `usize` is the number of bytes
    /// written.
    pub fn slave_wait_event(
        &mut self,
        bus: BusIndex,
        rx_buf: &mut [u8],
    ) -> Result<(SlaveEventKind, usize), I2cError> {
        let max_rx = rx_buf.len().min(wire::MAX_PAYLOAD_SIZE - 1);
        let req_len = encode_slave_wait_event_request(
            &mut self.request_buf,
            bus.value(),
            max_rx as u16,
        )
        .map_err(wire_to_i2c_error)?;

        let resp_len = self.send_recv(req_len)?;
        let data = self.decode_response(resp_len)?;

        if data.is_empty() {
            return Err(I2cError::from_code(ResponseCode::ServerError));
        }

        let kind = SlaveEventKind::from_u8(data[0])
            .ok_or_else(|| I2cError::from_code(ResponseCode::ServerError))?;

        let rx_len = if kind == SlaveEventKind::DataReceived {
            let n = (data.len() - 1).min(rx_buf.len());
            rx_buf[..n].copy_from_slice(&data[1..1 + n]);
            n
        } else {
            0
        };

        Ok((kind, rx_len))
    }
}

impl I2cTargetClient for IpcI2cClient {
    fn configure_target_address(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
    ) -> Result<(), Self::Error> {
        let req_len =
            encode_configure_slave_request(&mut self.request_buf, bus.value(), address.value())
                .map_err(wire_to_i2c_error)?;
        let resp_len = self.send_recv(req_len)?;
        let _ = self.decode_response(resp_len)?;
        Ok(())
    }

    fn enable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error> {
        let req_len = encode_enable_slave_request(&mut self.request_buf, bus.value())
            .map_err(wire_to_i2c_error)?;
        let resp_len = self.send_recv(req_len)?;
        let _ = self.decode_response(resp_len)?;
        Ok(())
    }

    fn disable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error> {
        let req_len = encode_disable_slave_request(&mut self.request_buf, bus.value())
            .map_err(wire_to_i2c_error)?;
        let resp_len = self.send_recv(req_len)?;
        let _ = self.decode_response(resp_len)?;
        Ok(())
    }

    fn wait_for_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
        _timeout: Option<core::time::Duration>,
    ) -> Result<usize, Self::Error> {
        let mut count = 0;
        for msg in messages.iter_mut() {
            let req_len = encode_slave_receive_request(
                &mut self.request_buf,
                bus.value(),
                TARGET_MESSAGE_MAX_LEN as u16,
            )
            .map_err(wire_to_i2c_error)?;
            let resp_len = self.send_recv(req_len)?;
            let data = self.decode_response(resp_len)?;
            if data.is_empty() {
                // No data (Stop or timeout) — no more messages pending.
                break;
            }
            let copy_len = core::cmp::min(data.len(), TARGET_MESSAGE_MAX_LEN);
            msg.data_mut()[..copy_len].copy_from_slice(&data[..copy_len]);
            msg.set_len(copy_len);
            count += 1;
        }
        Ok(count)
    }

    fn register_notification(
        &mut self,
        bus: BusIndex,
        _notification_mask: u32,
    ) -> Result<(), Self::Error> {
        let req_len =
            encode_enable_slave_notification_request(&mut self.request_buf, bus.value())
                .map_err(wire_to_i2c_error)?;
        let resp_len = self.send_recv(req_len)?;
        let _ = self.decode_response(resp_len)?;
        Ok(())
    }

    fn get_pending_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
    ) -> Result<usize, Self::Error> {
        // Non-blocking variant: same as wait_for_messages (hardware poll returns
        // immediately on timeout with 0 bytes).
        self.wait_for_messages(bus, messages, None)
    }
}
