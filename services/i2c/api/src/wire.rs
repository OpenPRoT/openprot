// Licensed under the Apache-2.0 license

//! I2C IPC Wire Protocol
//!
//! This module defines the binary wire protocol for I2C operations over IPC.
//! Uses manual byte encoding for no_std compatibility without external dependencies.
//!
//! ## Wire Format
//!
//! ```text
//! Request (8 bytes header + payload):
//! ┌────┬─────┬──────┬─────┬──────────┬──────────┐
//! │ op │ bus │ addr │ res │ write_len│ read_len │  + [write data]
//! │ 1B │ 1B  │ 1B   │ 1B  │  2B LE   │  2B LE   │
//! └────┴─────┴──────┴─────┴──────────┴──────────┘
//!
//! Response (4 bytes header + payload):
//! ┌──────┬─────┬──────────┐
//! │ code │ res │ data_len │  + [read data]
//! │ 1B   │ 1B  │  2B LE   │
//! └──────┴─────┴──────────┘
//! ```

use crate::ResponseCode;

// ============================================================================
// Wire Error
// ============================================================================

/// Error type for wire protocol encoding/decoding operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Output buffer too small for the encoded message
    BufferTooSmall,
    /// Payload exceeds MAX_PAYLOAD_SIZE
    PayloadTooLarge,
    /// Unrecognized operation code during decode
    InvalidOpcode(u8),
    /// Input buffer too short to contain a complete header
    Truncated,
}

// ============================================================================
// Operation Codes
// ============================================================================

/// I2C operation codes for IPC requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum I2cOp {
    /// Write data to device
    Write = 0,
    /// Read data from device
    Read = 1,
    /// Write then read (combined transaction)
    WriteRead = 2,
    /// Transaction with multiple operations
    Transaction = 3,
    /// Probe for device presence
    Probe = 4,
    /// Configure bus speed
    ConfigureSpeed = 5,
    /// Bus recovery
    RecoverBus = 6,
}

impl I2cOp {
    /// Convert from u8
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Write),
            1 => Some(Self::Read),
            2 => Some(Self::WriteRead),
            3 => Some(Self::Transaction),
            4 => Some(Self::Probe),
            5 => Some(Self::ConfigureSpeed),
            6 => Some(Self::RecoverBus),
            _ => None,
        }
    }
}

// ============================================================================
// Request Header
// ============================================================================

/// I2C request header for IPC messages (8 bytes)
///
/// This header is followed by operation-specific payload data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cRequestHeader {
    /// Operation code
    pub op: u8,
    /// Bus index (0-15)
    pub bus: u8,
    /// 7-bit I2C address
    pub address: u8,
    /// Length of write data (for Write, WriteRead operations)
    pub write_len: u16,
    /// Length of read data (for Read, WriteRead operations)
    pub read_len: u16,
}

impl I2cRequestHeader {
    /// Size of the header in bytes
    pub const SIZE: usize = 8;

    /// Create a new Write request header
    pub const fn write(bus: u8, address: u8, write_len: u16) -> Self {
        Self {
            op: I2cOp::Write as u8,
            bus,
            address,
            write_len,
            read_len: 0,
        }
    }

    /// Create a new Read request header
    pub const fn read(bus: u8, address: u8, read_len: u16) -> Self {
        Self {
            op: I2cOp::Read as u8,
            bus,
            address,
            write_len: 0,
            read_len,
        }
    }

    /// Create a new WriteRead request header
    pub const fn write_read(bus: u8, address: u8, write_len: u16, read_len: u16) -> Self {
        Self {
            op: I2cOp::WriteRead as u8,
            bus,
            address,
            write_len,
            read_len,
        }
    }

    /// Create a new Probe request header
    pub const fn probe(bus: u8, address: u8) -> Self {
        Self {
            op: I2cOp::Probe as u8,
            bus,
            address,
            write_len: 0,
            read_len: 0,
        }
    }

    /// Create a new RecoverBus request header
    pub const fn recover_bus(bus: u8) -> Self {
        Self {
            op: I2cOp::RecoverBus as u8,
            bus,
            address: 0,
            write_len: 0,
            read_len: 0,
        }
    }

    /// Encode header to bytes (little-endian)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let write_le = self.write_len.to_le_bytes();
        let read_le = self.read_len.to_le_bytes();
        [
            self.op,
            self.bus,
            self.address,
            0, // reserved
            write_le[0],
            write_le[1],
            read_le[0],
            read_le[1],
        ]
    }

    /// Decode header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            op: bytes[0],
            bus: bytes[1],
            address: bytes[2],
            // bytes[3] is reserved
            write_len: u16::from_le_bytes([bytes[4], bytes[5]]),
            read_len: u16::from_le_bytes([bytes[6], bytes[7]]),
        })
    }

    /// Get the operation code
    pub fn operation(&self) -> Option<I2cOp> {
        I2cOp::from_u8(self.op)
    }
}

// ============================================================================
// Response Header
// ============================================================================

/// I2C response header for IPC messages (4 bytes)
///
/// This header is followed by response data (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cResponseHeader {
    /// Response code (0 = success, >0 = error)
    pub code: u8,
    /// Length of response data
    pub data_len: u16,
}

impl I2cResponseHeader {
    /// Size of the header in bytes
    pub const SIZE: usize = 4;

    /// Create a success response header
    pub const fn success(data_len: u16) -> Self {
        Self {
            code: ResponseCode::Success as u8,
            data_len,
        }
    }

    /// Create an error response header
    pub const fn error(code: ResponseCode) -> Self {
        Self {
            code: code as u8,
            data_len: 0,
        }
    }

    /// Check if the response indicates success
    pub fn is_success(&self) -> bool {
        self.code == ResponseCode::Success as u8
    }

    /// Get the response code
    pub fn response_code(&self) -> ResponseCode {
        ResponseCode::from_u8(self.code).unwrap_or(ResponseCode::ServerError)
    }

    /// Encode header to bytes (little-endian)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let len_le = self.data_len.to_le_bytes();
        [self.code, 0, len_le[0], len_le[1]]
    }

    /// Decode header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            code: bytes[0],
            // bytes[1] is reserved
            data_len: u16::from_le_bytes([bytes[2], bytes[3]]),
        })
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Maximum payload size for I2C IPC messages
pub const MAX_PAYLOAD_SIZE: usize = 256;

/// Maximum total request message size (header + payload)
pub const MAX_REQUEST_SIZE: usize = I2cRequestHeader::SIZE + MAX_PAYLOAD_SIZE;

/// Maximum total response size (header + payload)
pub const MAX_RESPONSE_SIZE: usize = I2cResponseHeader::SIZE + MAX_PAYLOAD_SIZE;

// ============================================================================
// Encoding Helpers
// ============================================================================

/// Encode a write request into a buffer
///
/// # Errors
/// - `WireError::PayloadTooLarge` if data exceeds MAX_PAYLOAD_SIZE
/// - `WireError::BufferTooSmall` if buffer cannot hold header + data
pub fn encode_write_request(buf: &mut [u8], bus: u8, address: u8, data: &[u8]) -> Result<usize, WireError> {
    if data.len() > MAX_PAYLOAD_SIZE {
        return Err(WireError::PayloadTooLarge);
    }
    let write_len = u16::try_from(data.len()).map_err(|_| WireError::PayloadTooLarge)?;
    let total_len = I2cRequestHeader::SIZE + data.len();
    if buf.len() < total_len {
        return Err(WireError::BufferTooSmall);
    }

    let header = I2cRequestHeader::write(bus, address, write_len);
    buf[..I2cRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    buf[I2cRequestHeader::SIZE..total_len].copy_from_slice(data);

    Ok(total_len)
}

/// Encode a read request into a buffer
///
/// # Errors
/// - `WireError::PayloadTooLarge` if read_len exceeds MAX_PAYLOAD_SIZE
/// - `WireError::BufferTooSmall` if buffer cannot hold header
pub fn encode_read_request(buf: &mut [u8], bus: u8, address: u8, read_len: u16) -> Result<usize, WireError> {
    if read_len as usize > MAX_PAYLOAD_SIZE {
        return Err(WireError::PayloadTooLarge);
    }
    if buf.len() < I2cRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }

    let header = I2cRequestHeader::read(bus, address, read_len);
    buf[..I2cRequestHeader::SIZE].copy_from_slice(&header.to_bytes());

    Ok(I2cRequestHeader::SIZE)
}

/// Encode a write-read request into a buffer
///
/// # Errors
/// - `WireError::PayloadTooLarge` if write_data or read_len exceeds MAX_PAYLOAD_SIZE
/// - `WireError::BufferTooSmall` if buffer cannot hold header + write_data
pub fn encode_write_read_request(
    buf: &mut [u8],
    bus: u8,
    address: u8,
    write_data: &[u8],
    read_len: u16,
) -> Result<usize, WireError> {
    if write_data.len() > MAX_PAYLOAD_SIZE || read_len as usize > MAX_PAYLOAD_SIZE {
        return Err(WireError::PayloadTooLarge);
    }
    let write_len = u16::try_from(write_data.len()).map_err(|_| WireError::PayloadTooLarge)?;
    let total_len = I2cRequestHeader::SIZE + write_data.len();
    if buf.len() < total_len {
        return Err(WireError::BufferTooSmall);
    }

    let header = I2cRequestHeader::write_read(bus, address, write_len, read_len);
    buf[..I2cRequestHeader::SIZE].copy_from_slice(&header.to_bytes());
    buf[I2cRequestHeader::SIZE..total_len].copy_from_slice(write_data);

    Ok(total_len)
}

/// Encode a probe request into a buffer
///
/// # Errors
/// - `WireError::BufferTooSmall` if buffer cannot hold header
pub fn encode_probe_request(buf: &mut [u8], bus: u8, address: u8) -> Result<usize, WireError> {
    if buf.len() < I2cRequestHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }

    let header = I2cRequestHeader::probe(bus, address);
    buf[..I2cRequestHeader::SIZE].copy_from_slice(&header.to_bytes());

    Ok(I2cRequestHeader::SIZE)
}

// ============================================================================
// Decoding Helpers
// ============================================================================

/// Decode a response header from a buffer
///
/// # Errors
/// - `WireError::Truncated` if buffer is too short
pub fn decode_response_header(buf: &[u8]) -> Result<I2cResponseHeader, WireError> {
    I2cResponseHeader::from_bytes(buf).ok_or(WireError::Truncated)
}

/// Get response data from a buffer (after header)
///
/// # Errors
/// - `WireError::Truncated` if buffer doesn't contain declared data length
pub fn get_response_data<'a>(buf: &'a [u8], header: &I2cResponseHeader) -> Result<&'a [u8], WireError> {
    let data_end = I2cResponseHeader::SIZE + header.data_len as usize;
    if buf.len() < data_end {
        return Err(WireError::Truncated);
    }
    Ok(&buf[I2cResponseHeader::SIZE..data_end])
}

/// Decode a request header from a buffer
///
/// # Errors
/// - `WireError::Truncated` if buffer is too short
pub fn decode_request_header(buf: &[u8]) -> Result<I2cRequestHeader, WireError> {
    I2cRequestHeader::from_bytes(buf).ok_or(WireError::Truncated)
}

/// Get request payload data from a buffer (after header)
///
/// # Errors
/// - `WireError::Truncated` if buffer doesn't contain declared write_len
pub fn get_request_payload<'a>(buf: &'a [u8], header: &I2cRequestHeader) -> Result<&'a [u8], WireError> {
    let data_end = I2cRequestHeader::SIZE + header.write_len as usize;
    if buf.len() < data_end {
        return Err(WireError::Truncated);
    }
    Ok(&buf[I2cRequestHeader::SIZE..data_end])
}

// ============================================================================
// Response Encoding (for server side)
// ============================================================================

/// Encode a success response with data
///
/// # Errors
/// - `WireError::PayloadTooLarge` if data exceeds MAX_PAYLOAD_SIZE
/// - `WireError::BufferTooSmall` if buffer cannot hold header + data
pub fn encode_success_response(buf: &mut [u8], data: &[u8]) -> Result<usize, WireError> {
    if data.len() > MAX_PAYLOAD_SIZE {
        return Err(WireError::PayloadTooLarge);
    }
    let data_len = u16::try_from(data.len()).map_err(|_| WireError::PayloadTooLarge)?;
    let total_len = I2cResponseHeader::SIZE + data.len();
    if buf.len() < total_len {
        return Err(WireError::BufferTooSmall);
    }

    let header = I2cResponseHeader::success(data_len);
    buf[..I2cResponseHeader::SIZE].copy_from_slice(&header.to_bytes());
    buf[I2cResponseHeader::SIZE..total_len].copy_from_slice(data);

    Ok(total_len)
}

/// Encode an error response
///
/// # Errors
/// - `WireError::BufferTooSmall` if buffer cannot hold header
pub fn encode_error_response(buf: &mut [u8], code: ResponseCode) -> Result<usize, WireError> {
    if buf.len() < I2cResponseHeader::SIZE {
        return Err(WireError::BufferTooSmall);
    }

    let header = I2cResponseHeader::error(code);
    buf[..I2cResponseHeader::SIZE].copy_from_slice(&header.to_bytes());

    Ok(I2cResponseHeader::SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_header_size() {
        assert_eq!(I2cRequestHeader::SIZE, 8);
    }

    #[test]
    fn test_response_header_size() {
        assert_eq!(I2cResponseHeader::SIZE, 4);
    }

    #[test]
    fn test_request_header_roundtrip() {
        let header = I2cRequestHeader::write_read(1, 0x48, 2, 4);
        let bytes = header.to_bytes();
        let decoded = I2cRequestHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.op, I2cOp::WriteRead as u8);
        assert_eq!(decoded.bus, 1);
        assert_eq!(decoded.address, 0x48);
        assert_eq!(decoded.write_len, 2);
        assert_eq!(decoded.read_len, 4);
    }

    #[test]
    fn test_response_header_roundtrip() {
        let header = I2cResponseHeader::success(10);
        let bytes = header.to_bytes();
        let decoded = I2cResponseHeader::from_bytes(&bytes).unwrap();

        assert!(decoded.is_success());
        assert_eq!(decoded.data_len, 10);
    }

    #[test]
    fn test_error_response() {
        let header = I2cResponseHeader::error(ResponseCode::NoDevice);
        let bytes = header.to_bytes();
        let decoded = I2cResponseHeader::from_bytes(&bytes).unwrap();

        assert!(!decoded.is_success());
        assert_eq!(decoded.response_code(), ResponseCode::NoDevice);
    }

    #[test]
    fn test_encode_write_request() {
        let mut buf = [0u8; 32];
        let data = [0xAA, 0xBB, 0xCC];

        let len = encode_write_request(&mut buf, 1, 0x48, &data).unwrap();

        assert_eq!(len, I2cRequestHeader::SIZE + 3);

        let header = I2cRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(header.op, I2cOp::Write as u8);
        assert_eq!(header.bus, 1);
        assert_eq!(header.address, 0x48);
        assert_eq!(header.write_len, 3);

        assert_eq!(&buf[I2cRequestHeader::SIZE..len], &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_encode_read_request() {
        let mut buf = [0u8; 16];
        let len = encode_read_request(&mut buf, 2, 0x50, 8).unwrap();

        assert_eq!(len, I2cRequestHeader::SIZE);

        let header = I2cRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(header.op, I2cOp::Read as u8);
        assert_eq!(header.bus, 2);
        assert_eq!(header.address, 0x50);
        assert_eq!(header.read_len, 8);
    }

    #[test]
    fn test_encode_probe_request() {
        let mut buf = [0u8; 16];
        let len = encode_probe_request(&mut buf, 0, 0x2E).unwrap();

        assert_eq!(len, I2cRequestHeader::SIZE);

        let header = I2cRequestHeader::from_bytes(&buf).unwrap();
        assert_eq!(header.op, I2cOp::Probe as u8);
        assert_eq!(header.address, 0x2E);
    }

    #[test]
    fn test_i2c_op_from_u8() {
        assert_eq!(I2cOp::from_u8(0), Some(I2cOp::Write));
        assert_eq!(I2cOp::from_u8(1), Some(I2cOp::Read));
        assert_eq!(I2cOp::from_u8(2), Some(I2cOp::WriteRead));
        assert_eq!(I2cOp::from_u8(4), Some(I2cOp::Probe));
        assert_eq!(I2cOp::from_u8(100), None);
    }

    #[test]
    fn test_encode_write_request_payload_too_large() {
        let mut buf = [0u8; 512];
        let oversized_data = [0u8; MAX_PAYLOAD_SIZE + 1];
        assert_eq!(encode_write_request(&mut buf, 1, 0x48, &oversized_data), Err(WireError::PayloadTooLarge));
    }

    #[test]
    fn test_encode_write_request_buffer_too_small() {
        let mut buf = [0u8; 4]; // smaller than header
        let data = [0xAA, 0xBB];
        assert_eq!(encode_write_request(&mut buf, 1, 0x48, &data), Err(WireError::BufferTooSmall));
    }

    #[test]
    fn test_encode_read_request_payload_too_large() {
        let mut buf = [0u8; 16];
        assert_eq!(encode_read_request(&mut buf, 1, 0x48, (MAX_PAYLOAD_SIZE + 1) as u16), Err(WireError::PayloadTooLarge));
    }

    #[test]
    fn test_encode_write_read_request_payload_too_large() {
        let mut buf = [0u8; 512];
        let oversized_data = [0u8; MAX_PAYLOAD_SIZE + 1];
        assert_eq!(encode_write_read_request(&mut buf, 1, 0x48, &oversized_data, 4), Err(WireError::PayloadTooLarge));
        
        // Also test read_len exceeding limit
        let small_data = [0u8; 4];
        assert_eq!(encode_write_read_request(&mut buf, 1, 0x48, &small_data, (MAX_PAYLOAD_SIZE + 1) as u16), Err(WireError::PayloadTooLarge));
    }

    #[test]
    fn test_encode_success_response_payload_too_large() {
        let mut buf = [0u8; 512];
        let oversized_data = [0u8; MAX_PAYLOAD_SIZE + 1];
        assert_eq!(encode_success_response(&mut buf, &oversized_data), Err(WireError::PayloadTooLarge));
    }

    #[test]
    fn test_encode_at_max_payload_size() {
        let mut buf = [0u8; MAX_REQUEST_SIZE];
        let max_data = [0u8; MAX_PAYLOAD_SIZE];
        // Should succeed at exactly MAX_PAYLOAD_SIZE
        assert!(encode_write_request(&mut buf, 1, 0x48, &max_data).is_ok());
    }
}
