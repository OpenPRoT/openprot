// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C core types
//!
//! This module contains the core data types used throughout the I3C subsystem.

use core::sync::atomic::{AtomicBool, Ordering};

// =============================================================================
// Speed Enumerations
// =============================================================================

/// I3C transfer speed modes
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedI3c {
    /// SDR0 - Standard Data Rate 0 (12.5 `MHz` max)
    Sdr0 = 0x0,
    /// SDR1 - Standard Data Rate 1 (8 `MHz` max)
    Sdr1 = 0x1,
    /// SDR2 - Standard Data Rate 2 (6 `MHz` max)
    Sdr2 = 0x2,
    /// SDR3 - Standard Data Rate 3 (4 `MHz` max)
    Sdr3 = 0x3,
    /// SDR4 - Standard Data Rate 4 (2 `MHz` max)
    Sdr4 = 0x4,
    /// HDR-TS - High Data Rate Ternary Symbol
    HdrTs = 0x5,
    /// HDR-DDR - High Data Rate Double Data Rate
    HdrDdr = 0x6,
    /// I2C FM as I3C fallback
    I2cFmAsI3c = 0x7,
}

/// I2C transfer speed modes
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedI2c {
    /// Fast Mode (400 kHz)
    Fm = 0x0,
    /// Fast Mode Plus (1 `MHz`)
    Fmp = 0x1,
}

// =============================================================================
// Transaction ID
// =============================================================================

/// Transaction ID for tracking transfers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tid {
    /// Target IBI transaction
    TargetIbi = 0x1,
    /// Target read data transaction
    TargetRdData = 0x2,
    /// Target master write transaction
    TargetMasterWr = 0x8,
    /// Target master default transaction
    TargetMasterDef = 0xF,
}

// =============================================================================
// Transfer Status
// =============================================================================

/// I3C operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I3cStatus {
    /// Operation completed successfully
    Ok,
    /// Operation timed out
    Timeout,
    /// Bus is busy
    Busy,
    /// Operation is pending
    /// Invalid operation or parameter
    Invalid,
    /// Pending status
    Pending,
}

// =============================================================================
// Transfer Structures
// =============================================================================

/// I3C command descriptor
#[derive(Debug)]
pub struct I3cCmd<'a> {
    /// Lower 32 bits of command
    pub cmd_lo: u32,
    /// Upper 32 bits of command
    pub cmd_hi: u32,
    /// Transmit data buffer (optional)
    pub tx: Option<&'a [u8]>,
    /// Receive data buffer (optional)
    pub rx: Option<&'a mut [u8]>,
    /// Transmit length in bytes
    pub tx_len: u32,
    /// Receive length in bytes
    pub rx_len: u32,
    /// Return code from hardware
    pub ret: i32,
}

impl I3cCmd<'_> {
    /// Create a new command with default values
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cmd_lo: 0,
            cmd_hi: 0,
            tx: None,
            rx: None,
            tx_len: 0,
            rx_len: 0,
            ret: 0,
        }
    }
}

impl Default for I3cCmd<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// I3C message descriptor
pub struct I3cMsg<'a> {
    /// Data buffer.
    ///
    /// **Consumed by the transfer**: `priv_xfer_build_cmds` moves the reborrow
    /// into the command descriptor (`Option::take`), so this is `None` after a
    /// transfer. Read the result through `actual_len`/`num_xfer`; the caller
    /// still owns the underlying buffer.
    pub buf: Option<&'a mut [u8]>,
    /// Actual bytes transferred
    pub actual_len: u32,
    /// Number of transfers completed
    pub num_xfer: u32,
    /// Message flags (read/write/stop)
    pub flags: u8,
    /// HDR mode
    pub hdr_mode: u8,
    /// HDR command mode
    pub hdr_cmd_mode: u8,
}

impl I3cMsg<'_> {
    /// Create a new message with default values
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: None,
            actual_len: 0,
            num_xfer: 0,
            flags: 0,
            hdr_mode: 0,
            hdr_cmd_mode: 0,
        }
    }

    /// Check if this is a read message
    #[inline]
    #[must_use]
    pub const fn is_read(&self) -> bool {
        (self.flags & super::constants::I3C_MSG_READ) != 0
    }

    /// Check if this message should terminate with STOP
    #[inline]
    #[must_use]
    pub const fn has_stop(&self) -> bool {
        (self.flags & super::constants::I3C_MSG_STOP) != 0
    }
}

impl Default for I3cMsg<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// I3C transfer descriptor with multiple commands
pub struct I3cXfer<'cmds, 'buf> {
    /// Array of commands for this transfer
    pub cmds: &'cmds mut [I3cCmd<'buf>],
    /// Return code from transfer
    pub ret: i32,
}

impl<'cmds, 'buf> I3cXfer<'cmds, 'buf> {
    /// Create a new transfer with the given commands
    #[must_use]
    pub fn new(cmds: &'cmds mut [I3cCmd<'buf>]) -> Self {
        Self { cmds, ret: 0 }
    }

    /// Get the number of commands in this transfer
    #[inline]
    #[must_use]
    pub fn ncmds(&self) -> usize {
        self.cmds.len()
    }
}

// =============================================================================
// Legacy I2C Operations
// =============================================================================

/// One leg of a legacy-I2C transaction on the I3C bus.
///
/// Mirrors `embedded_hal::i2c::Operation` without pulling that type into the
/// hardware trait. Consecutive operations are joined by repeated START; the
/// last one ends with STOP.
pub enum I2cOp<'a> {
    /// Write the bytes to the device.
    Write(&'a [u8]),
    /// Read into the buffer (filled completely on success).
    Read(&'a mut [u8]),
}

// =============================================================================
// Device Identification
// =============================================================================

/// I3C Provisional ID (48-bit)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I3cPid(pub u64);

impl I3cPid {
    /// Create a new PID from raw value
    #[must_use]
    pub const fn new(pid: u64) -> Self {
        Self(pid)
    }

    /// Get the manufacturer ID (bits 47:33)
    #[must_use]
    pub const fn manuf_id(self) -> u16 {
        ((self.0 >> 33) & 0x1FFF) as u16
    }

    /// Check if lower 32 bits are random (bit 32)
    #[must_use]
    pub const fn has_random_lower32(self) -> bool {
        (self.0 & (1u64 << 32)) != 0
    }

    /// Get raw PID value
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// I3C device identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I3cDeviceId {
    /// Provisional ID
    pub pid: I3cPid,
}

impl I3cDeviceId {
    /// Create a new device ID from raw PID
    #[must_use]
    pub const fn new(pid: u64) -> Self {
        Self { pid: I3cPid(pid) }
    }
}

// =============================================================================
// IBI (In-Band Interrupt) Types
// =============================================================================

/// Type of In-Band Interrupt
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I3cIbiType {
    /// Target-initiated interrupt
    TargetIntr,
    /// Controller role request
    ControllerRoleRequest,
    /// Hot-join request
    HotJoin,
    /// Workqueue callback
    WorkqueueCb,
}

/// In-Band Interrupt descriptor
#[derive(Clone, Copy, Debug)]
pub struct I3cIbi<'a> {
    /// Type of IBI
    pub ibi_type: I3cIbiType,
    /// Optional payload data
    pub payload: Option<&'a [u8]>,
}

impl<'a> I3cIbi<'a> {
    /// Create a new IBI descriptor
    #[must_use]
    pub const fn new(ibi_type: I3cIbiType) -> Self {
        Self {
            ibi_type,
            payload: None,
        }
    }

    /// Create an IBI with payload
    #[must_use]
    pub const fn with_payload(ibi_type: I3cIbiType, payload: &'a [u8]) -> Self {
        Self {
            ibi_type,
            payload: Some(payload),
        }
    }

    /// Get payload length
    #[inline]
    #[must_use]
    pub fn payload_len(&self) -> u8 {
        self.payload.map_or(0, |p| {
            u8::try_from(p.len().min(u8::MAX as usize)).unwrap_or(u8::MAX)
        })
    }

    /// Get first byte of payload
    #[must_use]
    pub fn first_byte(&self) -> Option<u8> {
        self.payload.and_then(|p| p.first().copied())
    }
}

// =============================================================================
// Completion Primitive
// =============================================================================

/// Synchronization primitive for signaling completion
pub struct Completion {
    done: AtomicBool,
}

impl Default for Completion {
    fn default() -> Self {
        Self::new()
    }
}

impl Completion {
    /// Create a new completion in non-signaled state
    #[must_use]
    pub const fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
        }
    }

    /// Reset to non-signaled state
    #[inline]
    pub fn reset(&self) {
        self.done.store(false, Ordering::Release);
    }

    /// Signal completion
    #[inline]
    pub fn complete(&self) {
        self.done.store(true, Ordering::Release);
        // Wake any waiting cores
        cortex_m::asm::sev();
    }

    /// Check if completed
    #[inline]
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Wait for completion with timeout.
    ///
    /// Returns `true` if completed, `false` if timed out.
    ///
    /// Delta D2 (Cooperative-Yield Bounded-Poll Device): the reference took a
    /// `&mut D: DelayNs`; here the wait policy is the caller-injected,
    /// type-erased `yield_fn`, invoked once per non-completing poll with an
    /// advisory wait window in nanoseconds (1 µs, mirroring the reference's
    /// `delay.delay_us(1)`). A bare-metal caller passes
    /// `|_| core::hint::spin_loop()`.
    pub fn wait_for_us(&self, timeout_us: u32, yield_fn: &mut dyn FnMut(u32)) -> bool {
        let mut left = timeout_us;
        while !self.is_completed() {
            if left == 0 {
                return false;
            }
            yield_fn(1_000);
            left -= 1;
        }
        true
    }
}

// =============================================================================
// Device Kind
// =============================================================================

/// Device type on the I3C bus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevKind {
    /// Native I3C device
    I3c,
    /// Legacy I2C device
    I2c,
}
