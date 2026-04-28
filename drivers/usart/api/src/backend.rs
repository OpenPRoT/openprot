// Licensed under the Apache-2.0 license

use bitflags::bitflags;

use crate::protocol::UsartError;

bitflags! {
    /// Hardware-agnostic interrupt sources exposed by the USART backend.
    ///
    /// The bit layout is the contract between the wire protocol
    /// (`UsartOp::EnableInterrupts` / `DisableInterrupts` carry these in
    /// `arg0`), the dispatcher, and every backend implementation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct IrqMask: u16 {
        const RX_DATA_AVAILABLE = 0x0001;
        const TX_IDLE           = 0x0002;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendError {
    InvalidOperation,
    InvalidConfiguration,
    BufferTooSmall,
    Busy,
    Timeout,
    /// No data in the RX FIFO right now; retry after RX interrupt.
    WouldBlock,
    InternalError,
}

impl From<BackendError> for UsartError {
    fn from(value: BackendError) -> Self {
        match value {
            BackendError::InvalidOperation => UsartError::InvalidOperation,
            BackendError::InvalidConfiguration => UsartError::InvalidConfiguration,
            BackendError::BufferTooSmall => UsartError::BufferTooSmall,
            BackendError::Busy => UsartError::Busy,
            BackendError::Timeout => UsartError::Timeout,
            BackendError::WouldBlock => UsartError::WouldBlock,
            BackendError::InternalError => UsartError::InternalError,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsartConfig {
    pub baud_rate: u32,
    pub parity: Parity,
    pub stop_bits: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineStatus(pub u8);

pub trait UsartBackend {
    fn configure(&mut self, config: UsartConfig) -> Result<(), BackendError>;
    fn write(&mut self, data: &[u8]) -> Result<usize, BackendError>;
    fn read(&mut self, out: &mut [u8]) -> Result<usize, BackendError>;
    /// Non-blocking read.  Drains whatever bytes are available in the RX FIFO
    /// right now and returns them.  Returns `Err(BackendError::WouldBlock)` if
    /// the FIFO is empty; the caller is responsible for enabling the
    /// `RX_DATA_AVAILABLE` interrupt and retrying when it fires.
    fn try_read(&mut self, out: &mut [u8]) -> Result<usize, BackendError>;
    fn line_status(&self) -> Result<LineStatus, BackendError>;
    fn enable_interrupts(&mut self, mask: IrqMask) -> Result<(), BackendError>;
    fn disable_interrupts(&mut self, mask: IrqMask) -> Result<(), BackendError>;
}
