// Licensed under the Apache-2.0 license

//! Core error types without OS dependencies

/// I2C error type
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum I2cError {
    /// Data overrun
    Overrun,
    /// No acknowledge from device
    NoAcknowledge,
    /// Operation timeout
    Timeout,
    /// Bus recovery failed
    BusRecoveryFailed,
    /// Bus error
    Bus,
    /// Bus busy
    Busy,
    /// Invalid parameter
    Invalid,
    /// Abnormal condition
    Abnormal,
    /// Arbitration loss (multi-master)
    ArbitrationLoss,
    /// Slave mode error
    SlaveError,
    /// Invalid address
    InvalidAddress,
}
