// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Wire protocol for the i2c userspace driver.
//!
//! The unit marshalled across the IPC boundary is **one whole
//! `embedded_hal::i2c::I2c::transaction`** — an address plus an ordered list
//! of `Read`/`Write` operations the server replays atomically on the real bus
//! before replying. There is no per-operation round-trip and no bus
//! lock/unlock exposed across the boundary; that is what preserves the
//! exclusive-atomic contract of `I2c` between processes.
//!
//! ```text
//! request : [I2cRequestHeader] [I2cOpDesc; op_count] [write payloads...]
//! response: [I2cResponseHeader] [read payloads...]
//! ```
//!
//! `Write` ops carry their bytes inline (in operation order) after the
//! descriptor array. `Read` ops contribute nothing to the request; their
//! bytes come back, concatenated in operation order, after the response
//! header and are scattered into the caller's slices by the client.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Max total payload (sum of read or write bytes) in one transaction.
pub const MAX_PAYLOAD_SIZE: usize = 256;

/// Max number of `Operation`s in one transaction.
pub const MAX_OPS: usize = 16;

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cOp {
    /// One whole `I2c::transaction(address, &mut [Operation])`. Master mode.
    Transaction = 0x01,

    // ---- Target/slave mode (thin notification slice) ----
    // One IPC channel per bus, so none of these carry a bus field: the
    // server-runtime knows the bus from the channel the request arrived on
    // (same invariant as master). All use `I2cRequestHeader`; field meanings
    // are noted per op below.
    /// Set this bus's slave address. `address` = 7-bit slave address.
    ConfigureSlave = 0x02,
    /// Enter slave mode on this bus. No args.
    EnableSlave = 0x03,
    /// Leave slave mode on this bus. No args.
    DisableSlave = 0x04,
    /// Arm interrupt-driven slave-RX notification for this bus. No args.
    /// After this, the server raises `Signals::USER` on the bus channel when
    /// slave data has been latched.
    EnableSlaveNotification = 0x05,
    /// Disarm slave-RX notification for this bus. No args.
    DisableSlaveNotification = 0x06,
    /// Fetch the latched slave-RX buffer (non-blocking). `op_count` = caller's
    /// max byte count. Status `NoData` if nothing is latched.
    ///
    /// **Response payload layout** (on success):
    /// ```text
    /// [ kind (1) | source_addr (1) | data (0..max_len) ]
    /// ```
    /// - `kind`: [`SlaveEvent`] discriminant — what triggered the latch
    ///   (DataReceived, ReadRequest, Stop).
    /// - `source_addr`: 7-bit address of the sending master (`0x00..=0x7F`),
    ///   or `0xFF` if unavailable (message too short to extract it).
    /// - `data`: the received bytes, up to `op_count` (caller's max).
    SlaveReceive = 0x07,
    /// Pre-load the slave TX buffer for the next master read.
    /// `write_len` bytes from the request payload are written into the
    /// hardware TX buffer. The data is sent when the master issues a read
    /// to our slave address.
    ///
    /// NOTE: not required for MCTP-over-I2C (master-write only). Provided
    /// for completeness and testing of slave-TX / register-echo patterns.
    SlaveSetResponse = 0x08,
}

impl TryFrom<u8> for I2cOp {
    type Error = I2cError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Transaction),
            0x02 => Ok(Self::ConfigureSlave),
            0x03 => Ok(Self::EnableSlave),
            0x04 => Ok(Self::DisableSlave),
            0x05 => Ok(Self::EnableSlaveNotification),
            0x06 => Ok(Self::DisableSlaveNotification),
            0x07 => Ok(Self::SlaveReceive),
            0x08 => Ok(Self::SlaveSetResponse),
            _ => Err(I2cError::InvalidOperation),
        }
    }
}

/// Kind of event returned by `SlaveWaitEvent`.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlaveEvent {
    /// Master wrote data to our slave address.
    DataReceived = 0x00,
    /// Master issued a read from our slave address.
    ReadRequest = 0x01,
    /// Master stopped the transaction (stop condition).
    Stop = 0x02,
}

impl TryFrom<u8> for SlaveEvent {
    type Error = I2cError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::DataReceived),
            0x01 => Ok(Self::ReadRequest),
            0x02 => Ok(Self::Stop),
            _ => Err(I2cError::InvalidOperation),
        }
    }
}

/// Kind of a single bus operation within a transaction.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cOpKind {
    Write = 0x00,
    Read = 0x01,
}

impl TryFrom<u8> for I2cOpKind {
    type Error = I2cError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Write),
            0x01 => Ok(Self::Read),
            _ => Err(I2cError::InvalidOperation),
        }
    }
}

/// Status / error code carried in `I2cResponseHeader`.
///
/// The bus-failure variants map 1-to-1 onto `embedded_hal::i2c::ErrorKind`
/// (see [`crate::seam::error_kind`]).
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    InvalidOperation = 0x01,
    BufferTooSmall = 0x02,
    TooManyOperations = 0x03,
    /// NACK on the address phase.
    AddressNack = 0x04,
    /// NACK on a data byte.
    DataNack = 0x05,
    /// NACK, phase unknown.
    Nack = 0x06,
    ArbitrationLoss = 0x07,
    Bus = 0x08,
    Overrun = 0x09,
    Timeout = 0x0A,
    /// `SlaveReceive` found nothing latched (no slave data pending).
    NoData = 0x0B,
    InternalError = 0xFF,
}

impl From<u8> for I2cError {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::InvalidOperation,
            0x02 => Self::BufferTooSmall,
            0x03 => Self::TooManyOperations,
            0x04 => Self::AddressNack,
            0x05 => Self::DataNack,
            0x06 => Self::Nack,
            0x07 => Self::ArbitrationLoss,
            0x08 => Self::Bus,
            0x09 => Self::Overrun,
            0x0A => Self::Timeout,
            0x0B => Self::NoData,
            _ => Self::InternalError,
        }
    }
}

impl core::fmt::Display for I2cError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidOperation => f.write_str("invalid i2c operation"),
            Self::BufferTooSmall => f.write_str("buffer too small"),
            Self::TooManyOperations => f.write_str("too many operations"),
            Self::AddressNack => f.write_str("nack on address phase"),
            Self::DataNack => f.write_str("nack on data byte"),
            Self::Nack => f.write_str("nack (phase unknown)"),
            Self::ArbitrationLoss => f.write_str("arbitration loss"),
            Self::Bus => f.write_str("bus error"),
            Self::Overrun => f.write_str("overrun"),
            Self::Timeout => f.write_str("timeout"),
            Self::NoData => f.write_str("no slave data pending"),
            Self::InternalError => f.write_str("internal i2c server error"),
        }
    }
}

impl core::error::Error for I2cError {}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct I2cRequestHeader {
    pub(crate) op_code: u8,
    pub(crate) flags: u8,
    /// Target address. 7-bit address in the low 7 bits; 10-bit reserved.
    pub(crate) address: u16,
    /// Number of `I2cOpDesc` records that follow this header.
    pub(crate) op_count: u16,
    /// Total bytes after the header (op descriptors + inline write data).
    pub(crate) payload_len: u16,
}

impl I2cRequestHeader {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub fn new(op: I2cOp, address: u16, op_count: u16, payload_len: u16) -> Self {
        Self {
            op_code: op as u8,
            flags: 0,
            address: address.to_le(),
            op_count: op_count.to_le(),
            payload_len: payload_len.to_le(),
        }
    }

    pub fn operation(&self) -> Result<I2cOp, I2cError> {
        I2cOp::try_from(self.op_code)
    }

    pub fn address_value(&self) -> u16 {
        u16::from_le(self.address)
    }

    pub fn op_count_value(&self) -> usize {
        u16::from_le(self.op_count) as usize
    }

    pub fn payload_length(&self) -> usize {
        u16::from_le(self.payload_len) as usize
    }
}

/// One operation descriptor in the request body. For `Write`, `len` bytes of
/// inline payload follow (concatenated across ops, in order); for `Read`,
/// `len` is the number of bytes expected back in the response payload.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct I2cOpDesc {
    pub(crate) kind: u8,
    pub(crate) reserved: u8,
    pub(crate) len: u16,
}

impl I2cOpDesc {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub fn new(kind: I2cOpKind, len: u16) -> Self {
        Self {
            kind: kind as u8,
            reserved: 0,
            len: len.to_le(),
        }
    }

    pub fn op_kind(&self) -> Result<I2cOpKind, I2cError> {
        I2cOpKind::try_from(self.kind)
    }

    pub fn length(&self) -> usize {
        u16::from_le(self.len) as usize
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct I2cResponseHeader {
    pub(crate) status: u8,
    pub(crate) reserved: u8,
    /// Total read-payload bytes following this header.
    pub(crate) payload_len: u16,
}

impl I2cResponseHeader {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub fn success(payload_len: u16) -> Self {
        Self {
            status: 0,
            reserved: 0,
            payload_len: payload_len.to_le(),
        }
    }

    pub fn error(error: I2cError) -> Self {
        Self {
            status: error as u8,
            reserved: 0,
            payload_len: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == 0
    }

    pub fn error_code(&self) -> I2cError {
        I2cError::from(self.status)
    }

    pub fn payload_length(&self) -> usize {
        u16::from_le(self.payload_len) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::{FromBytes, IntoBytes};

    #[test]
    fn request_header_roundtrips_through_bytes() {
        let h = I2cRequestHeader::new(I2cOp::Transaction, 0x48, 3, 40);
        let bytes = h.as_bytes();
        assert_eq!(bytes.len(), I2cRequestHeader::SIZE);
        let decoded = I2cRequestHeader::ref_from_bytes(bytes).unwrap();
        assert_eq!(decoded.operation(), Ok(I2cOp::Transaction));
        assert_eq!(decoded.address_value(), 0x48);
        assert_eq!(decoded.op_count_value(), 3);
        assert_eq!(decoded.payload_length(), 40);
    }

    #[test]
    fn op_desc_roundtrips_and_decodes_kind() {
        for (kind, raw) in [(I2cOpKind::Write, 0u8), (I2cOpKind::Read, 1u8)] {
            let d = I2cOpDesc::new(kind, 17);
            let bytes = d.as_bytes();
            assert_eq!(bytes.len(), I2cOpDesc::SIZE);
            let decoded = I2cOpDesc::ref_from_bytes(bytes).unwrap();
            assert_eq!(decoded.length(), 17);
            assert_eq!(decoded.kind, raw);
            assert_eq!(decoded.op_kind(), Ok(kind));
        }
    }

    #[test]
    fn response_header_success_and_error() {
        let ok = I2cResponseHeader::success(12);
        let ok = I2cResponseHeader::ref_from_bytes(ok.as_bytes()).unwrap();
        assert!(ok.is_success());
        assert_eq!(ok.payload_length(), 12);

        let err = I2cResponseHeader::error(I2cError::AddressNack);
        let err = I2cResponseHeader::ref_from_bytes(err.as_bytes()).unwrap();
        assert!(!err.is_success());
        assert_eq!(err.error_code(), I2cError::AddressNack);
        assert_eq!(err.payload_length(), 0);
    }

    #[test]
    fn error_and_op_byte_mapping_is_stable() {
        for raw in 0x01u8..=0x0B {
            assert_eq!(I2cError::from(raw) as u8, raw);
        }
        // 0x00 is the success sentinel on the wire — it is not an I2cError variant.
        assert_eq!(I2cError::from(0x00), I2cError::InternalError);
        assert_eq!(I2cError::from(0x0B), I2cError::NoData);
        assert_eq!(I2cError::from(0xFF), I2cError::InternalError);
        assert_eq!(I2cError::from(0x42), I2cError::InternalError);
        assert_eq!(I2cOp::try_from(0x01), Ok(I2cOp::Transaction));
        assert_eq!(I2cOp::try_from(0x99), Err(I2cError::InvalidOperation));
    }

    #[test]
    fn slave_opcodes_roundtrip() {
        for (raw, op) in [
            (0x02u8, I2cOp::ConfigureSlave),
            (0x03, I2cOp::EnableSlave),
            (0x04, I2cOp::DisableSlave),
            (0x05, I2cOp::EnableSlaveNotification),
            (0x06, I2cOp::DisableSlaveNotification),
            (0x07, I2cOp::SlaveReceive),
            (0x08, I2cOp::SlaveSetResponse),
        ] {
            assert_eq!(I2cOp::try_from(raw), Ok(op));
            assert_eq!(op as u8, raw);
        }
    }

    #[test]
    fn slave_event_kinds_roundtrip() {
        for (raw, kind) in [
            (0x00u8, SlaveEvent::DataReceived),
            (0x01, SlaveEvent::ReadRequest),
            (0x02, SlaveEvent::Stop),
        ] {
            assert_eq!(SlaveEvent::try_from(raw), Ok(kind));
            assert_eq!(kind as u8, raw);
        }
        assert_eq!(SlaveEvent::try_from(0xFF), Err(I2cError::InvalidOperation));
    }

    #[test]
    fn configure_slave_header_carries_address() {
        // ConfigureSlave: address field = 7-bit slave address, no payload.
        let h = I2cRequestHeader::new(I2cOp::ConfigureSlave, 0x39, 0, 0);
        let h = I2cRequestHeader::ref_from_bytes(h.as_bytes()).unwrap();
        assert_eq!(h.operation(), Ok(I2cOp::ConfigureSlave));
        assert_eq!(h.address_value(), 0x39);
        assert_eq!(h.payload_length(), 0);
    }

    #[test]
    fn slave_receive_header_carries_max_len_in_op_count() {
        // SlaveReceive: op_count field = caller's max RX length.
        let h = I2cRequestHeader::new(I2cOp::SlaveReceive, 0, 64, 0);
        let h = I2cRequestHeader::ref_from_bytes(h.as_bytes()).unwrap();
        assert_eq!(h.operation(), Ok(I2cOp::SlaveReceive));
        assert_eq!(h.op_count_value(), 64);

        // NoData status round-trips for the empty-latch case.
        let nd = I2cResponseHeader::error(I2cError::NoData);
        let nd = I2cResponseHeader::ref_from_bytes(nd.as_bytes()).unwrap();
        assert!(!nd.is_success());
        assert_eq!(nd.error_code(), I2cError::NoData);
    }
}
