// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

/// ASPEED chip version information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspeedChipVersion {
    /// AST1030 A0 revision
    Ast1030A0,
    /// AST1030 A1 revision
    Ast1030A1,
    /// AST1035 A1 revision
    Ast1035A1,
    /// AST1060 A1 revision
    Ast1060A1,
    /// AST1060 A2 revision
    Ast1060A2,
    /// Unknown or unsupported version
    Unknown,
}

/// Memory region types in ASPEED OTP
/// Data region:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspeedOtpRegion {
    /// Data region (2048 double-words, 0x0000-0x0FFF)
    Data,
    /// Configuration region (32 double-words, 0x800-0x81F)
    Configuration,
    /// Strap region (64 bits, multiple programming options)
    Strap,
    /// SCU protection region (2 double-words, 0x1C-0x1D)
    ScuProtection,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[non_exhaustive]
pub enum OtpError {
    InvalidAddress,
    InvalidBufSize,
    MemoryLocked,
    WriteFailed,
    ReadFailed,
    LockFailed,
    VerificationFailed,
    WriteExhausted,
    NoSession,
    RegionProtected,
    AlignmentError,
    BoundaryError,
    Timeout,
    UnknownRevId,
    Unknown,
}

/// Protection status for different OTP regions
#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtectionStatus {
    /// Memory lock status (prevents all modifications)
    pub memory_locked: bool,
    /// Key return protection status
    pub key_protected: bool,
    /// Strap region protection status
    pub strap_protected: bool,
    /// Configuration region protection status
    pub config_protected: bool,
    /// User region protection status
    pub user_ecc_protected: bool,
    /// Security region protection status
    pub security_protected: bool,
    /// Security region size in bytes
    pub security_size: u32,
}

/// Strap bit programming status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrapStatus {
    /// Current strap bit value
    pub value: bool,
    /// Programming options available
    pub options: [u8; 7],
    /// Remaining write attempts
    pub remaining_writes: u8,
    /// Next writable option
    pub writable_option: u8,
    /// Protection status for this strap bit
    pub protected: bool,
}

/// Session information provided during OTP session establishment
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Chip version detected
    pub chip_version: AspeedChipVersion,
    /// Version name string
    pub version_name: [u8; 10],
    /// Current protection status
    pub protection_status: ProtectionStatus,
    /// Tool version information
    pub tool_version: [u8; 32],
    /// Software revision ID
    pub software_revision: u32,
    /// Number of cryptographic keys stored
    pub key_count: u32,
}

/// Sink for OTP diagnostic messages.
pub trait Logger {
    fn debug(&mut self, msg: &str);
    fn error(&mut self, msg: &str);
}

/// No-op implementation for production builds.
pub struct NoOpLogger;

impl Logger for NoOpLogger {
    fn debug(&mut self, _msg: &str) {}
    fn error(&mut self, _msg: &str) {}
}
