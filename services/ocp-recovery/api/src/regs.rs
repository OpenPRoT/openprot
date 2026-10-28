// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The register layouts the device serves to the Recovery Agent.
//!
//! Every struct matches the byte layout of one command, so the handler hands
//! the bytes straight to the transport. Fields are private: the accessors are
//! what decode a byte into a named value, and what refuse a value the spec
//! reserves.
//!
//! Three commands carry a field the agent writes and the device clears:
//! `RECOVERY_CTRL` byte 2, `DEVICE_RESET` byte 0 and `INDIRECT_FIFO_CTRL`
//! byte 1. The agent writes the command value and the device zeroes the field
//! once it has acted, so each of the three has an `apply_agent_write` that
//! takes the write and a `clear_*` the device calls afterwards. Zeroing is not
//! a bitwise clear of the written bits: writing 0x1 to the activate field is
//! not how activation ends, and `apply_agent_write` refuses the value.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::command::RecoveryCommand;
use crate::error::WireError;

/// The magic string that opens `PROT_CAP`, "OCP RECV" in ASCII.
pub const PROT_CAP_MAGIC: [u8; 8] = *b"OCP RECV";

/// Major version of the recovery protocol this crate implements.
pub const PROT_CAP_VERSION_MAJOR: u8 = 1;

/// Minor version of the recovery protocol this crate implements.
pub const PROT_CAP_VERSION_MINOR: u8 = 1;

/// Value the Recovery Agent writes to `RECOVERY_CTRL` byte 2 to activate the
/// image it pushed.
pub const ACTIVATE_RECOVERY_IMAGE: u8 = 0xf;

/// Value `RECOVERY_CTRL` byte 2 holds while no activation is in flight. The
/// device writes it back after the activation it was asked for.
pub const ACTIVATE_IDLE: u8 = 0x0;

/// Value the Recovery Agent writes to `INDIRECT_FIFO_CTRL` byte 1 to reset the
/// FIFO indices.
pub const FIFO_RESET: u8 = 0x1;

/// Value `INDIRECT_FIFO_CTRL` byte 1 holds while no reset is in flight.
pub const FIFO_IDLE: u8 = 0x0;

fn unsupported(command: RecoveryCommand, offset: usize, value: u8) -> WireError {
    WireError::UnsupportedParameter {
        command,
        offset,
        value,
    }
}

/// Capability bits of `PROT_CAP` bytes 10 and 11.
pub mod capability {
    /// The device serves `DEVICE_ID`.
    pub const IDENTIFICATION: u16 = 1 << 0;
    /// The device enters recovery when the agent commands forced recovery.
    pub const FORCED_RECOVERY: u16 = 1 << 1;
    /// The device supports the management reset of `DEVICE_RESET`.
    pub const MGMT_RESET: u16 = 1 << 2;
    /// The device supports the device reset of `DEVICE_RESET`.
    pub const DEVICE_RESET: u16 = 1 << 3;
    /// The device serves `DEVICE_STATUS`.
    pub const DEVICE_STATUS: u16 = 1 << 4;
    /// The device serves the indirect memory window.
    pub const MEMORY_ACCESS: u16 = 1 << 5;
    /// The device holds a recovery image of its own.
    pub const LOCAL_C_IMAGE: u16 = 1 << 6;
    /// The agent may push a recovery image to the device.
    pub const PUSH_C_IMAGE: u16 = 1 << 7;
    /// The device can isolate the interface.
    pub const INTERFACE_ISOLATION: u16 = 1 << 8;
    /// The device serves `HW_STATUS`.
    pub const HARDWARE_STATUS: u16 = 1 << 9;
    /// The device serves the vendor command.
    pub const VENDOR_COMMAND: u16 = 1 << 10;
    /// The device supports flashless boot.
    pub const FLASHLESS_BOOT: u16 = 1 << 11;
    /// The device serves the FIFO component memory space.
    pub const FIFO_CMS: u16 = 1 << 12;
}

/// `PROT_CAP`, the capabilities the device advertises.
///
/// The device fills this at init, not when recovery starts: the spec has the
/// agent read it whenever the recovery interface is up.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ProtCap {
    magic: [u8; 8],
    version_major: u8,
    version_minor: u8,
    capabilities: u16,
    cms_count: u8,
    max_response_time: u8,
    heartbeat_period: u8,
}

impl ProtCap {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Builds the capability block for a device.
    ///
    /// `max_response_time` and `heartbeat_period` are exponents: the value is
    /// 2^x microseconds, and a heartbeat period of 0 says the device sends no
    /// heartbeat.
    pub fn new(
        capabilities: u16,
        cms_count: u8,
        max_response_time: u8,
        heartbeat_period: u8,
    ) -> Self {
        Self {
            magic: PROT_CAP_MAGIC,
            version_major: PROT_CAP_VERSION_MAJOR,
            version_minor: PROT_CAP_VERSION_MINOR,
            capabilities: capabilities.to_le(),
            cms_count,
            max_response_time,
            heartbeat_period,
        }
    }

    /// The advertised capability bits, as in [`capability`].
    pub fn capabilities(&self) -> u16 {
        u16::from_le(self.capabilities)
    }

    /// Whether the device advertises every bit in `bits`.
    pub fn advertises(&self, bits: u16) -> bool {
        self.capabilities() & bits == bits
    }

    /// How many component memory space regions the device holds.
    pub fn cms_count(&self) -> u8 {
        self.cms_count
    }
}

/// The device status codes of `DEVICE_STATUS` byte 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum DeviceState {
    /// The device is booting and has not populated a reason code.
    StatusPending = 0x0,
    /// The device is running its operational firmware.
    Healthy = 0x1,
    /// The device hit an error it could not classify.
    DeviceError = 0x2,
    /// The device is in recovery mode and will accept an image.
    RecoveryModeReady = 0x3,
    /// The device holds an image and waits for the agent to activate it.
    RecoveryPending = 0x4,
    /// The device is running the image it recovered.
    RunningRecoveryImage = 0x5,
    /// The boot halted, and the reason code says why. Recoverable, which is
    /// what separates it from [`DeviceState::FatalError`].
    BootFailure = 0xe,
    /// The device cannot be recovered through the OCP flow.
    FatalError = 0xf,
}

impl DeviceState {
    /// Decodes byte 0 of `DEVICE_STATUS`, or `None` if the spec reserves the
    /// value.
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::StatusPending),
            0x1 => Some(Self::Healthy),
            0x2 => Some(Self::DeviceError),
            0x3 => Some(Self::RecoveryModeReady),
            0x4 => Some(Self::RecoveryPending),
            0x5 => Some(Self::RunningRecoveryImage),
            0xe => Some(Self::BootFailure),
            0xf => Some(Self::FatalError),
            _ => None,
        }
    }

    /// Whether the agent reads a recovery reason code alongside this status.
    ///
    /// The agent reads the reason code in recovery mode, while an activation
    /// is pending, and on a boot failure. The other states leave it stale, so
    /// the device writes 0 with them.
    pub fn populates_reason_code(self) -> bool {
        matches!(
            self,
            Self::RecoveryModeReady | Self::RecoveryPending | Self::BootFailure
        )
    }
}

/// The recovery reason codes of `DEVICE_STATUS` bytes 2 and 3.
///
/// The names follow the spec abbreviations. Vendor codes 0x80 to 0xff are
/// legal and this enum does not name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
#[non_exhaustive]
pub enum RecoveryReason {
    /// No boot failure detected.
    NoFailure = 0x0,
    /// Generic hardware error.
    GenericHardwareError = 0x1,
    /// Generic hardware soft error, which may be recoverable.
    GenericSoftError = 0x2,
    /// A self-test failed.
    SelfTestFailure = 0x3,
    /// Critical data is missing or corrupt.
    CorruptedCriticalData = 0x4,
    /// The key manifest is missing or corrupt.
    KeyManifestMissing = 0x5,
    /// The key manifest failed authentication.
    KeyManifestAuthFailure = 0x6,
    /// The key manifest failed the anti-rollback check.
    KeyManifestRollback = 0x7,
    /// The boot loader image is missing or corrupt.
    BootLoaderMissing = 0x8,
    /// The boot loader image failed authentication.
    BootLoaderAuthFailure = 0x9,
    /// The boot loader image failed the anti-rollback check.
    BootLoaderRollback = 0xa,
    /// The main firmware image is missing or corrupt.
    MainFirmwareMissing = 0xb,
    /// The main firmware image failed authentication.
    MainFirmwareAuthFailure = 0xc,
    /// The main firmware image failed the anti-rollback check.
    MainFirmwareRollback = 0xd,
    /// The recovery firmware is missing or corrupt.
    RecoveryFirmwareMissing = 0xe,
    /// The recovery firmware failed authentication.
    RecoveryFirmwareAuthFailure = 0xf,
    /// The recovery firmware failed the anti-rollback check.
    RecoveryFirmwareRollback = 0x10,
    /// The agent commanded forced recovery.
    ForcedRecovery = 0x11,
    /// The device boots with no local image and streams one over the
    /// recovery interface.
    FlashlessStreamingBoot = 0x12,
}

/// `DEVICE_STATUS`, the fixed head the vendor status follows.
///
/// Byte 1 holds the protocol error and the device clears it when the agent
/// reads the register, which is why [`DeviceStatus::take_protocol_error`] is
/// the only way to read it.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct DeviceStatus {
    status: u8,
    protocol_error: u8,
    reason_code: u16,
    heartbeat: u16,
    vendor_status_len: u8,
}

impl DeviceStatus {
    /// Encoded size of the head in bytes. A vendor status tail follows.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// The status a device reports before it has decided anything.
    pub fn pending() -> Self {
        Self {
            status: DeviceState::StatusPending as u8,
            protocol_error: crate::error::ProtocolError::None as u8,
            reason_code: 0,
            heartbeat: 0,
            vendor_status_len: 0,
        }
    }

    /// The device status, or `None` if the byte holds a reserved value.
    pub fn state(&self) -> Option<DeviceState> {
        DeviceState::from_byte(self.status)
    }

    /// Reports a state and the reason code that belongs with it.
    ///
    /// States that carry no reason code store 0, so a stale code never
    /// reaches the agent.
    pub fn set_state(&mut self, state: DeviceState, reason: RecoveryReason) {
        self.status = state as u8;
        self.reason_code = if state.populates_reason_code() {
            (reason as u16).to_le()
        } else {
            0
        };
    }

    /// The recovery reason code, as the raw value: vendor codes are legal and
    /// [`RecoveryReason`] does not name them.
    pub fn reason_code(&self) -> u16 {
        u16::from_le(self.reason_code)
    }

    /// Latches a protocol error for the agent to read.
    ///
    /// A second error before the agent reads overwrites the first, which is
    /// what the spec's single error byte allows.
    pub fn set_protocol_error(&mut self, error: crate::error::ProtocolError) {
        self.protocol_error = error as u8;
    }

    /// Reads the protocol error and clears it, the clear-on-read the spec
    /// defines for byte 1.
    pub fn take_protocol_error(&mut self) -> u8 {
        let error = self.protocol_error;
        self.protocol_error = crate::error::ProtocolError::None as u8;
        error
    }

    /// Advances the heartbeat counter, which wraps at 4095.
    pub fn tick_heartbeat(&mut self) {
        const HEARTBEAT_MODULUS: u16 = 4096;
        let next = (u16::from_le(self.heartbeat) + 1) % HEARTBEAT_MODULUS;
        self.heartbeat = next.to_le();
    }

    /// The heartbeat counter.
    pub fn heartbeat(&self) -> u16 {
        u16::from_le(self.heartbeat)
    }
}

/// The reset the Recovery Agent commands in `DEVICE_RESET` byte 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum ResetControl {
    /// No reset.
    None = 0x0,
    /// Reset the device, which disrupts the bus.
    ResetDevice = 0x1,
    /// Reset the management subsystem without disrupting the bus.
    ResetManagement = 0x2,
}

impl ResetControl {
    /// Decodes byte 0 of `DEVICE_RESET`, or `None` if the spec reserves the
    /// value.
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0x1 => Some(Self::ResetDevice),
            0x2 => Some(Self::ResetManagement),
            _ => None,
        }
    }
}

/// The mode the Recovery Agent arms in `DEVICE_RESET` byte 1, which the device
/// enters at the next platform reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum ForcedRecovery {
    /// Boot normally.
    None = 0x0,
    /// Boot with no local image and stream one over the recovery interface.
    FlashlessBoot = 0xe,
    /// Enter recovery mode.
    RecoveryMode = 0xf,
}

impl ForcedRecovery {
    /// Decodes byte 1 of `DEVICE_RESET`, or `None` if the spec reserves the
    /// value.
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0xe => Some(Self::FlashlessBoot),
            0xf => Some(Self::RecoveryMode),
            _ => None,
        }
    }
}

/// `DEVICE_RESET`, the reset and forced-recovery control.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct DeviceReset {
    reset_control: u8,
    forced_recovery: u8,
    interface_control: u8,
}

impl DeviceReset {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Checks a write off the wire and hands back the bytes at the width this
    /// register takes.
    ///
    /// This is the one step between the wire and
    /// [`DeviceReset::apply_agent_write`]: the length is resolved here, and what
    /// comes back is the fixed-size array the write takes, so no later step
    /// re-checks it. `transport_max` is the largest payload one access of the
    /// transport carries.
    pub fn from_wire(bytes: &[u8], transport_max: usize) -> Result<&[u8; Self::SIZE], WireError> {
        let command = RecoveryCommand::DeviceReset;
        command.check_write(bytes.len(), transport_max)?;
        bytes.try_into().ok().ok_or(WireError::LengthMismatch {
            command,
            got: bytes.len(),
        })
    }

    /// Byte 2 value that disables interface mastering, which is what the
    /// device powers on with.
    pub const INTERFACE_MASTERING_DISABLED: u8 = 0x0;

    /// Byte 2 value that enables interface mastering.
    pub const INTERFACE_MASTERING_ENABLED: u8 = 0x1;

    /// Takes a write from the Recovery Agent.
    ///
    /// The argument type settles the length, which [`Self::from_wire`]
    /// resolves where the bytes arrive off the wire.
    /// Every byte is validated before any is stored, so a refused write
    /// leaves the register as it was.
    pub fn apply_agent_write(&mut self, bytes: &[u8; Self::SIZE]) -> Result<(), WireError> {
        let command = RecoveryCommand::DeviceReset;
        let reset =
            ResetControl::from_byte(bytes[0]).ok_or_else(|| unsupported(command, 0, bytes[0]))?;
        let forced =
            ForcedRecovery::from_byte(bytes[1]).ok_or_else(|| unsupported(command, 1, bytes[1]))?;
        if bytes[2] > Self::INTERFACE_MASTERING_ENABLED {
            return Err(unsupported(command, 2, bytes[2]));
        }
        self.reset_control = reset as u8;
        self.forced_recovery = forced as u8;
        self.interface_control = bytes[2];
        Ok(())
    }

    /// The reset the agent commanded, if the value names one.
    pub fn reset_control(&self) -> Option<ResetControl> {
        ResetControl::from_byte(self.reset_control)
    }

    /// The mode the agent armed for the next platform reset.
    pub fn forced_recovery(&self) -> Option<ForcedRecovery> {
        ForcedRecovery::from_byte(self.forced_recovery)
    }

    /// Whether the agent enabled interface mastering.
    pub fn interface_mastering(&self) -> bool {
        self.interface_control == Self::INTERFACE_MASTERING_ENABLED
    }

    /// Zeroes byte 0 after the device performed the reset the agent asked
    /// for. The forced-recovery and interface bytes keep their values: the
    /// spec has the device clear only the reset control.
    pub fn clear_reset_control(&mut self) {
        self.reset_control = ResetControl::None as u8;
    }
}

/// How the Recovery Agent points the device at an image, in `RECOVERY_CTRL`
/// byte 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum ImageSelection {
    /// No operation.
    None = 0x0,
    /// Use the image the agent pushed into the memory window.
    FromMemoryWindow = 0x1,
    /// Use the image the device holds.
    FromDevice = 0x2,
}

impl ImageSelection {
    /// Decodes byte 1 of `RECOVERY_CTRL`, or `None` if the spec reserves the
    /// value.
    pub fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0x1 => Some(Self::FromMemoryWindow),
            0x2 => Some(Self::FromDevice),
            _ => None,
        }
    }
}

/// `RECOVERY_CTRL`, which selects the image and activates it.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RecoveryCtrl {
    cms: u8,
    image_selection: u8,
    activate: u8,
}

impl RecoveryCtrl {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Checks a write off the wire and hands back the bytes at the width this
    /// register takes.
    ///
    /// This is the one step between the wire and
    /// [`RecoveryCtrl::apply_agent_write`]: the length is resolved here, and what
    /// comes back is the fixed-size array the write takes, so no later step
    /// re-checks it. `transport_max` is the largest payload one access of the
    /// transport carries.
    pub fn from_wire(bytes: &[u8], transport_max: usize) -> Result<&[u8; Self::SIZE], WireError> {
        let command = RecoveryCommand::RecoveryCtrl;
        command.check_write(bytes.len(), transport_max)?;
        bytes.try_into().ok().ok_or(WireError::LengthMismatch {
            command,
            got: bytes.len(),
        })
    }

    /// Takes a write from the Recovery Agent.
    ///
    /// The argument type settles the length, which [`Self::from_wire`]
    /// resolves where the bytes arrive off the wire.
    ///
    /// Byte 2 accepts [`ACTIVATE_RECOVERY_IMAGE`] and [`ACTIVATE_IDLE`] and
    /// nothing else. A device that treated byte 2 as a bitwise clear would
    /// accept 0x1 here and leave 0xe behind, a value the spec does not
    /// define, so the write is refused instead.
    pub fn apply_agent_write(&mut self, bytes: &[u8; Self::SIZE]) -> Result<(), WireError> {
        let command = RecoveryCommand::RecoveryCtrl;
        let selection =
            ImageSelection::from_byte(bytes[1]).ok_or_else(|| unsupported(command, 1, bytes[1]))?;
        if bytes[2] != ACTIVATE_RECOVERY_IMAGE && bytes[2] != ACTIVATE_IDLE {
            return Err(unsupported(command, 2, bytes[2]));
        }
        self.cms = bytes[0];
        self.image_selection = selection as u8;
        self.activate = bytes[2];
        Ok(())
    }

    /// The component memory space the agent selected.
    pub fn cms(&self) -> u8 {
        self.cms
    }

    /// Where the agent pointed the device for the image.
    pub fn image_selection(&self) -> Option<ImageSelection> {
        ImageSelection::from_byte(self.image_selection)
    }

    /// Whether the agent asked for an activation the device has not finished.
    pub fn activation_requested(&self) -> bool {
        self.activate == ACTIVATE_RECOVERY_IMAGE
    }

    /// Zeroes byte 2 after the device activated the image. The agent reads
    /// [`ACTIVATE_IDLE`] from then on.
    pub fn clear_activation(&mut self) {
        self.activate = ACTIVATE_IDLE;
    }
}

/// The recovery states of `RECOVERY_STATUS` byte 0, bits 0 to 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum RecoveryState {
    /// The device is not in recovery mode.
    NotInRecovery = 0x0,
    /// The device waits for the agent to push the image.
    AwaitingImage = 0x1,
    /// The device is booting the image it received.
    BootingImage = 0x2,
    /// The image booted.
    Successful = 0x3,
    /// A stage failed to activate. Distinct from
    /// [`RecoveryState::AuthenticationError`], which names why.
    Failed = 0xc,
    /// The image failed authentication.
    AuthenticationError = 0xd,
    /// The device could not enter recovery mode, which an administrator may
    /// have disabled.
    EnterRecoveryError = 0xe,
    /// The agent named a component address space the device does not hold.
    InvalidComponentAddressSpace = 0xf,
}

impl RecoveryState {
    /// Decodes bits 0 to 3 of `RECOVERY_STATUS` byte 0, or `None` if the spec
    /// reserves the value.
    pub fn from_bits(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::NotInRecovery),
            0x1 => Some(Self::AwaitingImage),
            0x2 => Some(Self::BootingImage),
            0x3 => Some(Self::Successful),
            0xc => Some(Self::Failed),
            0xd => Some(Self::AuthenticationError),
            0xe => Some(Self::EnterRecoveryError),
            0xf => Some(Self::InvalidComponentAddressSpace),
            _ => None,
        }
    }
}

/// Which image a [`RecoveryStatus`] refers to, bounded to the 4-bit field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageIndex(u8);

impl ImageIndex {
    /// Largest index the field holds.
    pub const MAX: u8 = 0xf;

    /// The index of the first image, which the device reports before the
    /// Recovery Agent has pushed anything.
    pub const FIRST: Self = Self(0);

    /// Names an image, or `None` if the index does not fit the field.
    pub const fn new(index: u8) -> Option<Self> {
        if index > Self::MAX {
            return None;
        }
        Some(Self(index))
    }

    /// The index as the field stores it.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// `RECOVERY_STATUS`, which reports how far the device has come.
///
/// The register is read-only to the agent. The image index in bits 4 to 7
/// reports which stage the device is recovering, and the device keeps it set
/// to the stage a status refers to so a failure names the image it belongs to.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RecoveryStatus {
    status: u8,
    vendor_status: u8,
}

impl RecoveryStatus {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    const STATE_MASK: u8 = 0x0f;
    const INDEX_SHIFT: u8 = 4;

    /// Reports a state for one image.
    ///
    /// [`ImageIndex`] bounds the index to the 4 bits the field holds, so no
    /// index truncates into the state bits and the call cannot fail.
    pub fn set(&mut self, state: RecoveryState, image: ImageIndex) {
        self.status = (image.get() << Self::INDEX_SHIFT) | (state as u8);
    }

    /// The reported state, or `None` if the bits hold a reserved value.
    pub fn state(&self) -> Option<RecoveryState> {
        RecoveryState::from_bits(self.status & Self::STATE_MASK)
    }

    /// Which image the reported state refers to.
    pub fn image(&self) -> ImageIndex {
        ImageIndex(self.status >> Self::INDEX_SHIFT)
    }
}

/// Length of the vendor hardware status tail of `HW_STATUS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VendorStatusLen(u8);

impl VendorStatusLen {
    /// Longest tail the register carries, which the command table states as
    /// 0 to 251 bytes.
    pub const MAX: u8 = 251;

    /// No vendor hardware status follows the head.
    pub const NONE: Self = Self(0);

    /// Names a tail length, or `None` if it does not fit the register.
    pub const fn new(len: u8) -> Option<Self> {
        if len > Self::MAX {
            return None;
        }
        Some(Self(len))
    }

    /// The length in bytes.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// `HW_STATUS`, the fixed head the vendor hardware status follows.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct HwStatus {
    hw_status: u8,
    vendor_hw_status: u8,
    composite_temp: u8,
    vendor_status_len: u8,
}

impl HwStatus {
    /// Encoded size of the head in bytes. A vendor status tail follows.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Byte 0 bit 0: the device temperature is critical.
    pub const TEMPERATURE_CRITICAL: u8 = 1 << 0;
    /// Byte 0 bit 1: a hardware soft error, which a reset may clear.
    pub const SOFT_ERROR: u8 = 1 << 1;
    /// Byte 0 bit 2: a hardware fatal error.
    pub const FATAL_ERROR: u8 = 1 << 2;

    /// Byte 2 value that says the device has no temperature reading, or the
    /// reading is older than 5 seconds.
    pub const TEMPERATURE_UNAVAILABLE: u8 = 0x80;

    /// The hardware status bit mask.
    pub fn status_bits(&self) -> u8 {
        self.hw_status
    }

    /// Sets the hardware status bit mask.
    pub fn set_status_bits(&mut self, bits: u8) {
        self.hw_status = bits;
    }

    /// The composite temperature byte, in the NVMe-MI encoding.
    pub fn composite_temperature(&self) -> u8 {
        self.composite_temp
    }

    /// Sets the composite temperature byte.
    pub fn set_composite_temperature(&mut self, temperature: u8) {
        self.composite_temp = temperature;
    }

    /// How many bytes of vendor hardware status follow the head.
    pub fn vendor_status_len(&self) -> VendorStatusLen {
        VendorStatusLen(self.vendor_status_len)
    }

    /// Declares how many bytes of vendor hardware status follow the head.
    pub fn set_vendor_status_len(&mut self, len: VendorStatusLen) {
        self.vendor_status_len = len.get();
    }
}

/// `INDIRECT_FIFO_CTRL`, which selects the FIFO region and sizes the image.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct IndirectFifoCtrl {
    cms: u8,
    reset: u8,
    image_size_dwords: u32,
}

impl IndirectFifoCtrl {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Checks a write off the wire and hands back the bytes at the width this
    /// register takes.
    ///
    /// This is the one step between the wire and
    /// [`IndirectFifoCtrl::apply_agent_write`]: the length is resolved here, and what
    /// comes back is the fixed-size array the write takes, so no later step
    /// re-checks it. `transport_max` is the largest payload one access of the
    /// transport carries.
    pub fn from_wire(bytes: &[u8], transport_max: usize) -> Result<&[u8; Self::SIZE], WireError> {
        let command = RecoveryCommand::IndirectFifoCtrl;
        command.check_write(bytes.len(), transport_max)?;
        bytes.try_into().ok().ok_or(WireError::LengthMismatch {
            command,
            got: bytes.len(),
        })
    }

    /// Takes a write from the Recovery Agent.
    ///
    /// The argument type settles the length, which [`Self::from_wire`]
    /// resolves where the bytes arrive off the wire.
    ///
    /// Byte 1 is the reset the agent asks for and the device clears. Only the
    /// agent writes this register: a device that reset the FIFO on its own
    /// would race an agent that had already programmed the region and the
    /// image size for the next image.
    pub fn apply_agent_write(&mut self, bytes: &[u8; Self::SIZE]) -> Result<(), WireError> {
        let command = RecoveryCommand::IndirectFifoCtrl;
        if bytes[1] != FIFO_RESET && bytes[1] != FIFO_IDLE {
            return Err(unsupported(command, 1, bytes[1]));
        }
        let size = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        self.cms = bytes[0];
        self.reset = bytes[1];
        self.image_size_dwords = size.to_le();
        Ok(())
    }

    /// The component memory space the agent selected.
    pub fn cms(&self) -> u8 {
        self.cms
    }

    /// Whether the agent asked for a reset the device has not performed.
    pub fn reset_requested(&self) -> bool {
        self.reset == FIFO_RESET
    }

    /// Zeroes byte 1 after the device reset the FIFO indices.
    pub fn clear_reset(&mut self) {
        self.reset = FIFO_IDLE;
    }

    /// Size of the image the agent announced, in bytes.
    ///
    /// The register counts 4-byte units, so the byte count is four times the
    /// stored value and cannot overflow a `u64`.
    pub fn image_size_bytes(&self) -> u64 {
        u64::from(u32::from_le(self.image_size_dwords)) * 4
    }
}

/// `INDIRECT_FIFO_STATUS`, the FIFO indices and sizes.
///
/// The indices count 4-byte units. For the write-only code region the agent
/// advances the write index and the device advances the read index.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct IndirectFifoStatus {
    status: u8,
    region_type: u8,
    reserved: u16,
    write_index: u32,
    read_index: u32,
    fifo_size_dwords: u32,
    max_transfer_dwords: u32,
}

impl IndirectFifoStatus {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Byte 0 bit 0: the FIFO is empty.
    pub const EMPTY: u8 = 1 << 0;
    /// Byte 0 bit 1: the FIFO is full.
    pub const FULL: u8 = 1 << 1;

    /// Byte 1 value for a code region the agent writes the image into.
    pub const REGION_CODE: u8 = 0b000;
    /// Byte 1 value for a log region the agent reads.
    pub const REGION_LOG: u8 = 0b001;
    /// Byte 1 value for a vendor-defined region the agent writes.
    pub const REGION_VENDOR_WRITE_ONLY: u8 = 0b100;
    /// Byte 1 value for a vendor-defined region the agent reads.
    pub const REGION_VENDOR_READ_ONLY: u8 = 0b101;
    /// Byte 1 value the device reports for an address space it does not hold.
    pub const REGION_UNSUPPORTED: u8 = 0b111;

    /// Describes the code region the agent pushes an image into.
    ///
    /// Sizes are in bytes and the register stores 4-byte units, so both are
    /// refused unless they divide by 4.
    pub fn code_region(fifo_size_bytes: u32, max_transfer_bytes: u32) -> Option<Self> {
        const DWORD: u32 = 4;
        if fifo_size_bytes % DWORD != 0 || max_transfer_bytes % DWORD != 0 {
            return None;
        }
        Some(Self {
            status: Self::EMPTY,
            region_type: Self::REGION_CODE,
            reserved: 0,
            write_index: 0,
            read_index: 0,
            fifo_size_dwords: (fifo_size_bytes / DWORD).to_le(),
            max_transfer_dwords: (max_transfer_bytes / DWORD).to_le(),
        })
    }

    /// The status bit mask.
    pub fn status_bits(&self) -> u8 {
        self.status
    }

    /// How many bytes the agent has written.
    pub fn written_bytes(&self) -> u64 {
        u64::from(u32::from_le(self.write_index)) * 4
    }

    /// How many bytes the device has consumed.
    pub fn read_bytes(&self) -> u64 {
        u64::from(u32::from_le(self.read_index)) * 4
    }

    /// Largest payload one `INDIRECT_FIFO_DATA` access may carry, in bytes.
    pub fn max_transfer_bytes(&self) -> u64 {
        u64::from(u32::from_le(self.max_transfer_dwords)) * 4
    }

    /// Resets both indices and reports the FIFO empty, which is what the
    /// device does when the agent asks for a reset in `INDIRECT_FIFO_CTRL`.
    pub fn reset_indices(&mut self) {
        self.write_index = 0;
        self.read_index = 0;
        self.status = Self::EMPTY;
    }

    /// Records one agent write the device consumed on the spot.
    ///
    /// The eRoT drains each `INDIRECT_FIFO_DATA` payload into the staging
    /// region before answering, so both indices advance together and the FIFO
    /// reports empty again. The indices count 4-byte units and the spec rounds
    /// a shorter transfer up to the next unit, which covers the last chunk of
    /// an image whose size does not divide by 4.
    pub fn record_drained_write(&mut self, bytes: usize) {
        const DWORD: usize = 4;
        let dwords = bytes.div_ceil(DWORD) as u32;
        let next = u32::from_le(self.write_index).wrapping_add(dwords);
        self.write_index = next.to_le();
        self.read_index = next.to_le();
        self.status = Self::EMPTY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Length;
    use crate::error::ProtocolError;
    use zerocopy::IntoBytes;

    /// The encoded size of every fixed-layout register, against the length
    /// the command table states.
    #[test]
    fn register_sizes_match_the_command_table() {
        let fixed = [
            (RecoveryCommand::ProtCap, ProtCap::SIZE),
            (RecoveryCommand::DeviceReset, DeviceReset::SIZE),
            (RecoveryCommand::RecoveryCtrl, RecoveryCtrl::SIZE),
            (RecoveryCommand::RecoveryStatus, RecoveryStatus::SIZE),
            (RecoveryCommand::IndirectFifoCtrl, IndirectFifoCtrl::SIZE),
            (
                RecoveryCommand::IndirectFifoStatus,
                IndirectFifoStatus::SIZE,
            ),
        ];
        for (command, size) in fixed {
            assert_eq!(command.length(), Length::Fixed(size), "{command:?}");
        }

        // The variable-length registers start at the head this crate defines.
        assert_eq!(
            RecoveryCommand::DeviceStatus.length(),
            Length::Range {
                min: DeviceStatus::SIZE,
                max: 255
            }
        );
        assert_eq!(
            RecoveryCommand::HwStatus.length(),
            Length::Range {
                min: HwStatus::SIZE,
                max: 255
            }
        );
    }

    #[test]
    fn from_wire_resolves_the_length_once_for_the_register_that_takes_it() {
        let wire = [0u8, ImageSelection::FromMemoryWindow as u8, ACTIVATE_IDLE];
        let sized = RecoveryCtrl::from_wire(&wire, crate::SMBUS_MAX_PAYLOAD).unwrap();
        assert_eq!(sized, &wire);

        let short = [0u8, ImageSelection::FromMemoryWindow as u8];
        let err = RecoveryCtrl::from_wire(&short, crate::SMBUS_MAX_PAYLOAD).unwrap_err();
        assert_eq!(
            crate::ProtocolError::from(err),
            crate::ProtocolError::LengthWriteError
        );
    }

    #[test]
    fn writing_one_to_the_activate_field_is_refused_and_changes_nothing() {
        let mut ctrl = RecoveryCtrl::default();
        ctrl.apply_agent_write(&[0, ImageSelection::FromMemoryWindow as u8, ACTIVATE_IDLE])
            .unwrap();

        // A device that cleared byte 2 bitwise would accept 0x1 here and hold
        // 0xe afterwards, which the spec does not define for the field.
        let err = ctrl
            .apply_agent_write(&[0, ImageSelection::FromMemoryWindow as u8, 0x1])
            .unwrap_err();
        assert_eq!(
            err,
            WireError::UnsupportedParameter {
                command: RecoveryCommand::RecoveryCtrl,
                offset: 2,
                value: 0x1,
            }
        );
        assert_eq!(
            ProtocolError::from(err),
            ProtocolError::UnsupportedParameter
        );
        assert!(!ctrl.activation_requested());
        assert_eq!(ctrl.as_bytes()[2], ACTIVATE_IDLE);
    }

    #[test]
    fn the_agent_activates_and_the_device_clears_the_field() {
        let mut ctrl = RecoveryCtrl::default();
        ctrl.apply_agent_write(&[
            2,
            ImageSelection::FromMemoryWindow as u8,
            ACTIVATE_RECOVERY_IMAGE,
        ])
        .unwrap();
        assert!(ctrl.activation_requested());
        assert_eq!(ctrl.cms(), 2);
        assert_eq!(
            ctrl.image_selection(),
            Some(ImageSelection::FromMemoryWindow)
        );

        ctrl.clear_activation();
        assert!(!ctrl.activation_requested());
        assert_eq!(ctrl.as_bytes()[2], ACTIVATE_IDLE);
    }

    #[test]
    fn a_reserved_image_selection_leaves_the_register_as_it_was() {
        let mut ctrl = RecoveryCtrl::default();
        ctrl.apply_agent_write(&[7, ImageSelection::FromDevice as u8, ACTIVATE_IDLE])
            .unwrap();

        let err = ctrl
            .apply_agent_write(&[9, 0x3, ACTIVATE_RECOVERY_IMAGE])
            .unwrap_err();
        assert_eq!(
            err,
            WireError::UnsupportedParameter {
                command: RecoveryCommand::RecoveryCtrl,
                offset: 1,
                value: 0x3,
            }
        );
        assert_eq!(ctrl.cms(), 7);
        assert_eq!(ctrl.image_selection(), Some(ImageSelection::FromDevice));
        assert!(!ctrl.activation_requested());
    }

    #[test]
    fn device_reset_clears_only_the_reset_control_byte() {
        let mut reset = DeviceReset::default();
        reset
            .apply_agent_write(&[
                ResetControl::ResetManagement as u8,
                ForcedRecovery::RecoveryMode as u8,
                DeviceReset::INTERFACE_MASTERING_DISABLED,
            ])
            .unwrap();
        assert_eq!(reset.reset_control(), Some(ResetControl::ResetManagement));
        assert_eq!(reset.forced_recovery(), Some(ForcedRecovery::RecoveryMode));

        reset.clear_reset_control();
        assert_eq!(reset.reset_control(), Some(ResetControl::None));
        assert_eq!(reset.forced_recovery(), Some(ForcedRecovery::RecoveryMode));
        assert!(!reset.interface_mastering());
    }

    #[test]
    fn a_reserved_forced_recovery_value_is_refused() {
        let mut reset = DeviceReset::default();
        let err = reset
            .apply_agent_write(&[
                ResetControl::None as u8,
                0x5,
                DeviceReset::INTERFACE_MASTERING_DISABLED,
            ])
            .unwrap_err();
        assert_eq!(
            err,
            WireError::UnsupportedParameter {
                command: RecoveryCommand::DeviceReset,
                offset: 1,
                value: 0x5,
            }
        );
        assert_eq!(reset.forced_recovery(), Some(ForcedRecovery::None));
    }

    #[test]
    fn prot_cap_opens_with_the_magic_string_and_the_protocol_version() {
        let cap = ProtCap::new(
            capability::IDENTIFICATION | capability::PUSH_C_IMAGE | capability::FIFO_CMS,
            1,
            0,
            0,
        );
        let bytes = cap.as_bytes();
        assert_eq!(bytes.len(), ProtCap::SIZE);
        assert_eq!(&bytes[0..8], &PROT_CAP_MAGIC);
        assert_eq!(bytes[8], PROT_CAP_VERSION_MAJOR);
        assert_eq!(bytes[9], PROT_CAP_VERSION_MINOR);
        assert!(cap.advertises(capability::PUSH_C_IMAGE | capability::FIFO_CMS));
        assert!(!cap.advertises(capability::LOCAL_C_IMAGE));
    }

    #[test]
    fn reading_the_protocol_error_clears_it() {
        let mut status = DeviceStatus::pending();
        status.set_protocol_error(ProtocolError::LengthWriteError);
        assert_eq!(
            status.take_protocol_error(),
            ProtocolError::LengthWriteError as u8
        );
        assert_eq!(status.take_protocol_error(), ProtocolError::None as u8);
    }

    #[test]
    fn a_state_that_carries_no_reason_code_reports_zero() {
        let mut status = DeviceStatus::pending();
        status.set_state(
            DeviceState::BootFailure,
            RecoveryReason::MainFirmwareAuthFailure,
        );
        assert_eq!(status.state(), Some(DeviceState::BootFailure));
        assert_eq!(
            status.reason_code(),
            RecoveryReason::MainFirmwareAuthFailure as u16
        );

        // Healthy carries no reason code, so the stale one must not survive.
        status.set_state(DeviceState::Healthy, RecoveryReason::NoFailure);
        assert_eq!(status.reason_code(), 0);
    }

    #[test]
    fn the_heartbeat_wraps_at_the_twelve_bit_limit() {
        let mut status = DeviceStatus::pending();
        for _ in 0..4095 {
            status.tick_heartbeat();
        }
        assert_eq!(status.heartbeat(), 4095);
        status.tick_heartbeat();
        assert_eq!(status.heartbeat(), 0);
    }

    #[test]
    fn recovery_status_packs_the_image_index_above_the_state() {
        let mut status = RecoveryStatus::default();
        let image = ImageIndex::new(2).unwrap();
        status.set(RecoveryState::AuthenticationError, image);
        assert_eq!(status.state(), Some(RecoveryState::AuthenticationError));
        assert_eq!(status.image(), image);
        assert_eq!(status.as_bytes()[0], 0x2d);
    }

    #[test]
    fn an_image_index_wider_than_the_field_has_no_value() {
        assert_eq!(ImageIndex::new(ImageIndex::MAX + 1), None);
        assert_eq!(
            ImageIndex::new(ImageIndex::MAX).map(ImageIndex::get),
            Some(ImageIndex::MAX)
        );
    }

    #[test]
    fn the_largest_image_index_does_not_reach_the_state_bits() {
        let mut status = RecoveryStatus::default();
        let last = ImageIndex::new(ImageIndex::MAX).unwrap();
        status.set(RecoveryState::Failed, last);
        assert_eq!(status.state(), Some(RecoveryState::Failed));
        assert_eq!(status.image(), last);
    }

    #[test]
    fn the_fifo_control_counts_the_image_in_four_byte_units() {
        let mut ctrl = IndirectFifoCtrl::default();
        let size_bytes: u32 = 0x4000;
        let mut write = [0u8; IndirectFifoCtrl::SIZE];
        write[0] = 0;
        write[1] = FIFO_RESET;
        write[2..].copy_from_slice(&(size_bytes / 4).to_le_bytes());
        ctrl.apply_agent_write(&write).unwrap();

        assert!(ctrl.reset_requested());
        assert_eq!(ctrl.image_size_bytes(), u64::from(size_bytes));
        ctrl.clear_reset();
        assert!(!ctrl.reset_requested());
    }

    #[test]
    fn a_fifo_region_that_does_not_divide_by_four_is_refused() {
        assert!(IndirectFifoStatus::code_region(0x4000, 250).is_none());
        assert!(IndirectFifoStatus::code_region(0x4002, 252).is_none());

        // The SMBus payload cap divides by 4, so a FIFO sized to it is legal.
        let region =
            IndirectFifoStatus::code_region(0x4000, crate::SMBUS_MAX_PAYLOAD as u32).unwrap();
        assert_eq!(region.status_bits(), IndirectFifoStatus::EMPTY);
        assert_eq!(region.max_transfer_bytes(), crate::SMBUS_MAX_PAYLOAD as u64);
        assert_eq!(region.as_bytes().len(), IndirectFifoStatus::SIZE);
    }

    #[test]
    fn a_vendor_hardware_status_longer_than_the_register_has_no_value() {
        let mut hw = HwStatus::default();
        hw.set_status_bits(HwStatus::SOFT_ERROR);
        hw.set_composite_temperature(HwStatus::TEMPERATURE_UNAVAILABLE);
        let tail = VendorStatusLen::new(8).unwrap();
        hw.set_vendor_status_len(tail);
        assert_eq!(hw.vendor_status_len(), tail);
        assert_eq!(hw.status_bits(), HwStatus::SOFT_ERROR);

        assert_eq!(VendorStatusLen::new(VendorStatusLen::MAX + 1), None);
    }

    #[test]
    fn resetting_the_fifo_zeroes_both_indices_and_reports_empty() {
        let mut region = IndirectFifoStatus::code_region(0x4000, 248).unwrap();
        region.reset_indices();
        assert_eq!(region.written_bytes(), 0);
        assert_eq!(region.read_bytes(), 0);
        assert_eq!(
            region.status_bits() & IndirectFifoStatus::EMPTY,
            IndirectFifoStatus::EMPTY
        );
    }
}
