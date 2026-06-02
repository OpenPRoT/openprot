// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The abstract seam.
//!
//! This seam covers both sides of I2C.
//! Clients use **`embedded_hal::i2c::I2c`** for controller-side transfers, and
//! target-side support is re-exported below from `openprot_hal_blocking`.
//! That keeps the API small and reuses the standard traits and platform
//! drivers we already have.
//!
//! Re-exported here so the server and per-target backend crates depend on this
//! one seam definition rather than pinning `embedded-hal` independently.

pub use embedded_hal::i2c::{
    AddressMode, Error as I2cBusError, ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation,
    SevenBitAddress, TenBitAddress,
};

// Target-side traits are re-exported from `openprot_hal_blocking`.
// The runtime stays generic, and each backend forwards to its platform driver.
pub use openprot_hal_blocking::i2c_hardware::slave::{
    I2cSEvent, I2cSlaveBuffer, I2cSlaveCore, I2cSlaveInterrupts, SlaveStatus,
};
pub use openprot_hal_blocking::i2c_hardware::{I2cBusRecovery, I2cHardwareCore};

use crate::protocol::I2cError;

/// Extension trait for fetching the next slave event with its full event kind.
/// Enables backends to return both the hardware event kind (ReadRequest, Stop, etc.)
/// alongside the rx length, so the server-runtime can propagate correct metadata.
/// Default impl delegates to `poll_slave_data()` for backward compatibility.
pub trait I2cSlaveEvent: I2cSlaveBuffer {
    /// Return the next slave event and rx length, if any.
    /// Default impl uses `poll_slave_data()` and reports DataReceived kind.
    fn try_next_slave_event(&mut self) -> Result<Option<(I2cSEvent, usize)>, Self::Error> {
        Ok(self
            .poll_slave_data()?
            .map(|n| (I2cSEvent::SlaveWrRecvd, n)))
    }
}

/// Blanket impl so all I2cSlaveBuffer types get the default behavior.
impl<T: I2cSlaveBuffer> I2cSlaveEvent for T {}

/// Map a wire status code onto the `embedded_hal::i2c::ErrorKind` taxonomy so
/// the client can satisfy `embedded_hal::i2c::Error`.
pub fn error_kind(err: I2cError) -> ErrorKind {
    match err {
        I2cError::AddressNack => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address),
        I2cError::DataNack => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data),
        I2cError::Nack => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
        I2cError::ArbitrationLoss => ErrorKind::ArbitrationLoss,
        I2cError::Bus => ErrorKind::Bus,
        I2cError::Overrun => ErrorKind::Overrun,
        _ => ErrorKind::Other,
    }
}
