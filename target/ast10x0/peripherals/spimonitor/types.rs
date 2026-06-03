// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI monitor public types.

/// Direction selector for address privilege programming.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivilegeDirection {
    Read,
    Write,
}

/// Whether a privilege region grants or denies access.
///
/// Mirrors Zephyr's `SPI_FILTER_PRIV_ENABLE` / `SPI_FILTER_PRIV_DISABLE`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivilegeOp {
    /// Grant access: the region is permitted for the given direction.
    Enable,
    /// Deny access: the region is blocked for the given direction.
    Disable,
}

/// Monitor filter passthrough mode.
///
/// When `Enabled`, SPI traffic bypasses the filter (used during mux ownership
/// transitions). When `Disabled`, the programmed policy is enforced.
/// Mirrors Zephyr's `spim_passthrough_config`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassthroughMode {
    Enabled,
    Disabled,
}

/// External mux selection for SPI path routing.
///
/// `Sel0` and `Sel1` are hardware-level values. Platform code is responsible
/// for mapping these to ROT vs BMC/PCH roles (with optional polarity inversion).
/// Mirrors Zephyr's `spim_ext_mux_sel`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtMuxSel {
    Sel0,
    Sel1,
}

/// Lock state observed from hardware policy tables.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockState {
    Unlocked,
    Locked,
}

/// High-level monitor lifecycle stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorState {
    Uninitialized,
    Configured,
    Locked,
}

/// Region policy entry used by higher-level policy/controller code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionPolicy {
    pub start: u32,
    pub length: u32,
    pub direction: PrivilegeDirection,
    /// Whether this region grants (`Enable`) or denies (`Disable`) access.
    pub op: PrivilegeOp,
}

impl RegionPolicy {
    /// Construct a region that grants access in the given direction.
    #[must_use]
    pub const fn allow(start: u32, length: u32, direction: PrivilegeDirection) -> Self {
        Self { start, length, direction, op: PrivilegeOp::Enable }
    }

    /// Construct a region that denies access in the given direction.
    #[must_use]
    pub const fn deny(start: u32, length: u32, direction: PrivilegeDirection) -> Self {
        Self { start, length, direction, op: PrivilegeOp::Disable }
    }
}

/// Decoded entry from the SPIPF violation log RAM.
///
/// The hardware log word encoding (bits[19:18]):
/// - `0b00` → blocked command opcode in bits[7:0]
/// - `0b01` → blocked write address: bits[17:0] << 14
/// - `0b10` → blocked read address:  bits[17:0] << 14
/// - other  → invalid/reserved
///
/// ISR callback installation, workqueue deferral, and log-pointer reset are
/// caller / platform responsibilities and are not part of this crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViolationLogEntry {
    BlockedCommand(u8),
    BlockedWriteAddr(u32),
    BlockedReadAddr(u32),
    Invalid(u32),
}

impl ViolationLogEntry {
    /// Decode a raw 32-bit hardware log word.
    #[must_use]
    pub fn parse(word: u32) -> Self {
        match (word >> 18) & 0x3 {
            0x0 => Self::BlockedCommand((word & 0xFF) as u8),
            0x1 => Self::BlockedWriteAddr((word & 0x3_FFFF) << 14),
            0x2 => Self::BlockedReadAddr((word & 0x3_FFFF) << 14),
            _ => Self::Invalid(word),
        }
    }
}

/// SPI monitor module error type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiMonitorError {
    InvalidRegion,
    InvalidSlot,
    Locked,
    InvalidTransition,
}

/// Result alias for SPI monitor APIs.
pub type Result<T> = core::result::Result<T, SpiMonitorError>;

// ============================================================================
// Boot-level types and abstractions
// ============================================================================

/// SPI Monitor instance identifier.
///
/// Maps to SPIPF1-4 hardware blocks on AST10x0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorInstance {
    /// SPIPF1 (0x7E79_1000) - typically BMC/SMC flash
    Spim0,
    /// SPIPF2 (0x7E79_2000) - typically BMC dual flash
    Spim1,
    /// SPIPF3 (0x7E79_3000) - typically PCH/FMC flash
    Spim2,
    /// SPIPF4 (0x7E79_4000) - typically PCH dual flash
    Spim3,
}

/// Mux routing selector for monitor path control.
///
/// Controls which SPI master (ROT or BMC/PCH) owns the flash access path
/// through the monitor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MuxSelect {
    /// Root of Trust has exclusive control
    RotControl,
    /// Host (BMC/PCH) has control
    HostControl,
}

impl From<MuxSelect> for ExtMuxSel {
    fn from(mux: MuxSelect) -> Self {
        match mux {
            MuxSelect::RotControl => ExtMuxSel::Sel0,
            MuxSelect::HostControl => ExtMuxSel::Sel1,
        }
    }
}

impl From<ExtMuxSel> for MuxSelect {
    fn from(ext: ExtMuxSel) -> Self {
        match ext {
            ExtMuxSel::Sel0 => MuxSelect::RotControl,
            ExtMuxSel::Sel1 => MuxSelect::HostControl,
        }
    }
}

/// Monitor status snapshot at a point in time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorStatus {
    /// Current mux routing
    pub mux: MuxSelect,
    /// Whether policy tables are write-locked
    pub policy_locked: bool,
    /// Whether enforcement is actively filtering
    pub enforcement_active: bool,
    /// Number of policy violations logged
    pub violation_count: u32,
}

/// Boot-level error types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootError {
    /// Monitor instance not found or unavailable
    MonitorNotFound,
    /// Mux switch operation failed
    MuxSwitchFailed,
    /// Flash reset/control operation failed
    FlashResetFailed,
    /// Policy load failed (region conflict, out-of-range, etc.)
    PolicyLoadFailed,
    /// Policy verification failed (readback mismatch)
    PolicyVerificationFailed,
    /// Region overlap detected
    RegionOverlap,
    /// Invalid address or size
    InvalidAddress,
    /// Timeout waiting for flash operation
    TimeoutWaitingForFlash,
    /// Verification failed (state not as expected)
    VerificationFailed,
    /// Attempted to modify locked policy
    LockedOutFromMonitor,
    /// Hardware error
    HardwareError,
}

/// Result type for boot operations.
pub type BootResult<T> = core::result::Result<T, BootError>;

/// Boot phase enumeration for state tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootPhase {
    /// Initial state
    Start,
    /// Hold phase - ROT exclusive access
    Hold,
    /// Policy configuration phase
    ConfigurePolicy,
    /// Release phase - host control
    Release,
    /// Runtime monitoring phase
    RuntimeMonitoring,
}

/// Boot configuration options.
#[derive(Clone, Copy, Debug)]
pub struct BootConfig {
    pub enable_hold: bool,
    pub enable_policy_config: bool,
    pub enable_release: bool,
    pub enable_verification: bool,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            enable_hold: true,
            enable_policy_config: true,
            enable_release: true,
            enable_verification: true,
        }
    }
}
