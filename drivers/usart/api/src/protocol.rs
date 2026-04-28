// Licensed under the Apache-2.0 license

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub const MAX_PAYLOAD_SIZE: usize = 256;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsartOp {
    Configure = 0x01,
    Write = 0x02,
    Read = 0x03,
    GetLineStatus = 0x04,
    EnableInterrupts = 0x05,
    DisableInterrupts = 0x06,
    /// Non-blocking read attempt.  Returns data immediately if available;
    /// the server queues the request and completes it when the RX IRQ fires
    /// if no data is ready yet.
    TryRead = 0x07,
}

impl TryFrom<u8> for UsartOp {
    type Error = UsartError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Configure),
            0x02 => Ok(Self::Write),
            0x03 => Ok(Self::Read),
            0x04 => Ok(Self::GetLineStatus),
            0x05 => Ok(Self::EnableInterrupts),
            0x06 => Ok(Self::DisableInterrupts),
            0x07 => Ok(Self::TryRead),
            _ => Err(UsartError::InvalidOperation),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsartError {
    Success = 0x00,
    InvalidOperation = 0x01,
    InvalidConfiguration = 0x02,
    BufferTooSmall = 0x03,
    Busy = 0x04,
    Timeout = 0x05,
    /// No data available right now; the server has queued the request and will
    /// complete it when data arrives via RX interrupt.
    WouldBlock = 0x06,
    InternalError = 0xFF,
}

impl From<u8> for UsartError {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Success,
            0x01 => Self::InvalidOperation,
            0x02 => Self::InvalidConfiguration,
            0x03 => Self::BufferTooSmall,
            0x04 => Self::Busy,
            0x05 => Self::Timeout,
            0x06 => Self::WouldBlock,
            _ => Self::InternalError,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct UsartRequestHeader {
    pub op_code: u8,
    pub flags: u8,
    pub arg0: u16,
    pub arg1: u16,
    pub payload_len: u16,
}

impl UsartRequestHeader {
    pub const SIZE: usize = 8;

    pub fn new(op: UsartOp, arg0: u16, arg1: u16, payload_len: u16) -> Self {
        Self {
            op_code: op as u8,
            flags: 0,
            arg0: arg0.to_le(),
            arg1: arg1.to_le(),
            payload_len: payload_len.to_le(),
        }
    }

    pub fn operation(&self) -> Result<UsartOp, UsartError> {
        UsartOp::try_from(self.op_code)
    }

    pub fn payload_length(&self) -> usize {
        u16::from_le(self.payload_len) as usize
    }

    pub fn arg0_value(&self) -> u16 {
        u16::from_le(self.arg0)
    }

    pub fn arg1_value(&self) -> u16 {
        u16::from_le(self.arg1)
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct UsartResponseHeader {
    pub status: u8,
    pub reserved: u8,
    pub payload_len: u16,
}

impl UsartResponseHeader {
    pub const SIZE: usize = 4;

    pub fn success(payload_len: u16) -> Self {
        Self {
            status: UsartError::Success as u8,
            reserved: 0,
            payload_len: payload_len.to_le(),
        }
    }

    pub fn error(error: UsartError) -> Self {
        Self {
            status: error as u8,
            reserved: 0,
            payload_len: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == UsartError::Success as u8
    }

    pub fn error_code(&self) -> UsartError {
        UsartError::from(self.status)
    }

    pub fn payload_length(&self) -> usize {
        u16::from_le(self.payload_len) as usize
    }
}
