// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # I3C Hardware Abstraction Traits
//!
//! Platform-agnostic traits for I3C hardware controllers. These are the seams
//! that services and backends code against; no silicon type names appear here.
//!
//! ## Trait hierarchy
//!
//! ```text
//! I3cErrorType          — shared associated Error type
//! ├── I3cBusRecovery    — unstick a locked bus
//! ├── I3cController     — primary controller: DAA, private transfers, IBI, DAT
//! └── I3cTarget         — secondary target: hot-join, dynamic address query
//! ```
//!
//! All three traits are implemented on the platform's controller shell
//! (e.g. `I3cController<'c, H, Ready>` in `target/ast10x0`) rather than on
//! the raw hardware driver, so `I3cConfig` and other chip-specific state
//! remain internal.
//!
//! ## What does NOT belong here
//!
//! - Clock/timing configuration (`init_clock`, `calc_i2c_clk`) — chip-specific
//! - FIFO mechanics (`wr_tx_fifo`, `rd_rx_fifo`) — internal driver plumbing
//! - ISR handlers (`i3c_aspeed_isr`, `end_xfer`) — internal driver mechanics
//! - Queue/halt control (`start_xfer`, `enter_halt`, `reset_ctrl`) — internal
//! - Hardware init (`HardwareCore::init`, `init_pid`) — chip-specific bringup

/// Shared associated error type for all I3C HAL traits.
pub trait I3cErrorType {
    /// The error type returned by I3C hardware operations.
    type Error: core::fmt::Debug;
}

/// Target receive/transmit data-path operations.
pub trait I3cTargetRxTx: I3cErrorType {
    /// Returns true if at least one inbound write frame is queued.
    fn target_rx_pending(&self) -> bool;

    /// Drain one inbound write frame into `out`.
    ///
    /// Returns the number of bytes copied into `out`, or `None` if no frame
    /// is pending.
    fn target_rx_read(&mut self, out: &mut [u8]) -> Result<Option<usize>, Self::Error>;

    /// Queue the response payload for the next controller private-read.
    fn target_tx_write(&mut self, data: &[u8]) -> Result<(), Self::Error>;
}

/// Target RX interrupt control operations.
pub trait I3cTargetInterruptControl: I3cErrorType {
    /// Enable the target RX interrupt source.
    fn target_enable_rx_interrupt(&mut self) -> Result<(), Self::Error>;

    /// Disable the target RX interrupt source.
    fn target_disable_rx_interrupt(&mut self) -> Result<(), Self::Error>;
}

/// Target IBI operations.
pub trait I3cTargetIbi: I3cErrorType {
    /// Raise an IBI with the given MDB and optional payload.
    fn target_raise_ibi(&mut self, mdb: u8, payload: &[u8]) -> Result<(), Self::Error>;
}

/// Target address-query operations.
pub trait I3cTargetAddressInfo: I3cErrorType {
    /// Return the dynamic address assigned by the primary controller, if any.
    fn target_dynamic_address(&self) -> Result<Option<u8>, Self::Error>;
}

/// Target hot-join operations.
pub trait I3cTargetHotJoin: I3cErrorType {
    /// Raise a Hot-Join IBI to request bus mastership.
    fn target_raise_hot_join(&mut self) -> Result<(), Self::Error>;
}

/// Bus recovery: unstick a locked SCL/SDA line without a full controller reset.
///
/// Mirrors `I2cBusRecovery` from `i2c_hardware`. The server-runtime calls this
/// after a transfer error before surfacing the error to its client.
pub trait I3cBusRecovery: I3cErrorType {
    /// Toggle SCL the given number of times in software mode to release a held bus.
    ///
    /// On success the bus is idle and the next transaction can proceed.
    fn recover_bus(&mut self, scl_toggles: u32) -> Result<(), Self::Error>;
}

/// Primary controller operations: DAA, private transfers, IBI, DAT management.
///
/// Implemented on the controller shell (which already holds the config) so
/// callers never see `I3cConfig` or chip-specific setup.
pub trait I3cController: I3cErrorType {
    /// Read `out.len()` bytes from the device with the given Provisional ID.
    ///
    /// Returns the number of bytes actually received.
    fn priv_read(&mut self, pid: u64, out: &mut [u8]) -> Result<u32, Self::Error>;

    /// Write `data` to the device with the given Provisional ID.
    fn priv_write(&mut self, pid: u64, data: &mut [u8]) -> Result<(), Self::Error>;

    /// Run ENTDAA to assign dynamic addresses to all unaddressed devices.
    ///
    /// Returns the number of devices that received an address.
    fn bus_daa(&mut self) -> Result<u32, Self::Error>;

    /// Attach an I3C device entry at the given DAT slot.
    fn attach_i3c_dev(&mut self, pid: u64, desired_da: u8, slot: u8) -> Result<(), Self::Error>;

    /// Detach the device at the given DAT position.
    fn detach_i3c_dev(&mut self, pos: usize) -> Result<(), Self::Error>;

    /// Enable IBI (In-Band Interrupt) for the device at `addr`.
    ///
    /// `mdb` is the Mandatory Data Byte the device will send with each IBI.
    fn enable_ibi(&mut self, addr: u8, mdb: u8) -> Result<(), Self::Error>;

    /// Disable IBI for the device at `addr`.
    fn disable_ibi(&mut self, addr: u8) -> Result<(), Self::Error>;
}

/// Secondary (target) mode operations.
pub trait I3cTarget: I3cErrorType {
    /// Raise a Hot-Join IBI to request bus mastership.
    fn target_raise_hot_join(&mut self) -> Result<(), Self::Error>;

    /// Return the dynamic address assigned by the primary controller, if any.
    fn target_dynamic_address(&self) -> Option<u8>;
}
