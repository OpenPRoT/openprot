// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The recovery command codes and the access rules that come with them.

use crate::error::WireError;

/// Whether the Recovery Agent may write a command, or only read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// The Recovery Agent reads the command. A write raises
    /// [`WireError::WriteToReadOnly`].
    ReadOnly,
    /// The Recovery Agent reads and writes the command.
    ReadWrite,
}

/// How many bytes a command carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// The command carries exactly this many bytes.
    Fixed(usize),
    /// The command carries a fixed head followed by a vendor-defined tail, so
    /// the length runs from `min` to `max` inclusive.
    Range {
        /// Smallest accepted length, the fixed head on its own.
        min: usize,
        /// Largest accepted length.
        max: usize,
    },
    /// The command is a data aperture and carries as many bytes as the
    /// transport allows. SMBus caps it at
    /// [`SMBUS_MAX_PAYLOAD`](crate::SMBUS_MAX_PAYLOAD); I3C and USB
    /// negotiate their own cap.
    Payload,
}

impl Length {
    /// Whether a transfer of `len` bytes fits the command.
    ///
    /// `transport_max` is the largest payload the transport carries in one
    /// access, and only [`Length::Payload`] consults it: the fixed and ranged
    /// lengths come from the spec and hold on every transport.
    pub fn accepts(self, len: usize, transport_max: usize) -> bool {
        match self {
            Self::Fixed(n) => len == n,
            Self::Range { min, max } => (min..=max).contains(&len),
            Self::Payload => (1..=transport_max).contains(&len),
        }
    }
}

/// When the device must answer a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The device answers whenever the recovery interface is up, so the
    /// contents are populated at init rather than when recovery starts.
    Always,
    /// The device answers once the recovery interface is in recovery mode.
    RecoveryMode,
}

/// A recovery command, named by the command byte of the transaction.
///
/// Decoding the byte once through [`RecoveryCommand::try_from`] gives the
/// handler a value whose access rule, length and scope are settled, so no
/// later step re-validates the byte. The spec closes the command set at
/// 0x2f, so the enum is exhaustive and a consumer match covers every command
/// without a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecoveryCommand {
    /// Device capabilities, magic string and protocol version.
    ProtCap = 0x22,
    /// Device identity, in the form the DMTF firmware update standard uses.
    DeviceId = 0x23,
    /// Accumulated device status, protocol error and recovery reason code.
    DeviceStatus = 0x24,
    /// Device reset, forced recovery and interface mastering control.
    DeviceReset = 0x25,
    /// Selects the memory region to recover from and activates the image.
    RecoveryCtrl = 0x26,
    /// How far the device has come with the image it is recovering.
    RecoveryStatus = 0x27,
    /// Hardware status, including the composite temperature.
    HwStatus = 0x28,
    /// Selects a memory region for indirect access.
    IndirectCtrl = 0x29,
    /// Status of the region the indirect window selects.
    IndirectStatus = 0x2a,
    /// Read and write aperture of the indirect window.
    IndirectData = 0x2b,
    /// Vendor-defined command.
    Vendor = 0x2c,
    /// Selects a FIFO region and carries the size of the image to come.
    IndirectFifoCtrl = 0x2d,
    /// FIFO indices, region size and maximum transfer size.
    IndirectFifoStatus = 0x2e,
    /// Write aperture the Recovery Agent pushes the image through.
    IndirectFifoData = 0x2f,
}

impl RecoveryCommand {
    /// Whether the Recovery Agent may write the command.
    pub fn access(self) -> Access {
        match self {
            Self::ProtCap
            | Self::DeviceId
            | Self::DeviceStatus
            | Self::RecoveryStatus
            | Self::HwStatus
            | Self::IndirectStatus
            | Self::IndirectFifoStatus => Access::ReadOnly,
            Self::DeviceReset
            | Self::RecoveryCtrl
            | Self::IndirectCtrl
            | Self::IndirectData
            | Self::Vendor
            | Self::IndirectFifoCtrl
            | Self::IndirectFifoData => Access::ReadWrite,
        }
    }

    /// How many bytes the command carries, as the command table states them.
    pub fn length(self) -> Length {
        match self {
            Self::ProtCap => Length::Fixed(15),
            Self::DeviceId => Length::Range { min: 24, max: 255 },
            Self::DeviceStatus => Length::Range { min: 7, max: 255 },
            Self::DeviceReset | Self::RecoveryCtrl => Length::Fixed(3),
            Self::RecoveryStatus => Length::Fixed(2),
            Self::HwStatus => Length::Range { min: 4, max: 255 },
            Self::IndirectCtrl | Self::IndirectStatus | Self::IndirectFifoCtrl => Length::Fixed(6),
            Self::IndirectFifoStatus => Length::Fixed(20),
            Self::IndirectData | Self::Vendor | Self::IndirectFifoData => Length::Payload,
        }
    }

    /// When the device must answer the command.
    pub fn scope(self) -> Scope {
        match self {
            Self::ProtCap
            | Self::DeviceId
            | Self::DeviceStatus
            | Self::DeviceReset
            | Self::RecoveryCtrl
            | Self::RecoveryStatus => Scope::Always,
            Self::HwStatus
            | Self::IndirectCtrl
            | Self::IndirectStatus
            | Self::IndirectData
            | Self::Vendor
            | Self::IndirectFifoCtrl
            | Self::IndirectFifoStatus
            | Self::IndirectFifoData => Scope::RecoveryMode,
        }
    }

    /// Checks an incoming write against the access rule and the length.
    ///
    /// The device calls this before touching any register state, so a refused
    /// write leaves the register file as it was. `transport_max` is the
    /// largest payload one access of the transport carries, which SMBus sets
    /// to [`SMBUS_MAX_PAYLOAD`](crate::SMBUS_MAX_PAYLOAD).
    pub fn check_write(self, len: usize, transport_max: usize) -> Result<(), WireError> {
        if self.access() == Access::ReadOnly {
            return Err(WireError::WriteToReadOnly(self));
        }
        if !self.length().accepts(len, transport_max) {
            return Err(WireError::LengthMismatch {
                command: self,
                got: len,
            });
        }
        Ok(())
    }
}

impl TryFrom<u8> for RecoveryCommand {
    type Error = WireError;

    fn try_from(value: u8) -> Result<Self, WireError> {
        match value {
            0x22 => Ok(Self::ProtCap),
            0x23 => Ok(Self::DeviceId),
            0x24 => Ok(Self::DeviceStatus),
            0x25 => Ok(Self::DeviceReset),
            0x26 => Ok(Self::RecoveryCtrl),
            0x27 => Ok(Self::RecoveryStatus),
            0x28 => Ok(Self::HwStatus),
            0x29 => Ok(Self::IndirectCtrl),
            0x2a => Ok(Self::IndirectStatus),
            0x2b => Ok(Self::IndirectData),
            0x2c => Ok(Self::Vendor),
            0x2d => Ok(Self::IndirectFifoCtrl),
            0x2e => Ok(Self::IndirectFifoStatus),
            0x2f => Ok(Self::IndirectFifoData),
            other => Err(WireError::UnknownCommand(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ProtocolError;

    #[test]
    fn command_bytes_outside_the_recovery_range_are_refused() {
        assert_eq!(
            RecoveryCommand::try_from(0x21),
            Err(WireError::UnknownCommand(0x21))
        );
        assert_eq!(
            RecoveryCommand::try_from(0x30),
            Err(WireError::UnknownCommand(0x30))
        );
    }

    #[test]
    fn every_command_byte_in_the_recovery_range_decodes() {
        for byte in 0x22..=0x2fu8 {
            let command = RecoveryCommand::try_from(byte).expect("command in range decodes");
            assert_eq!(command as u8, byte);
        }
    }

    #[test]
    fn writing_a_read_only_command_reports_unsupported_command() {
        let err = RecoveryCommand::ProtCap
            .check_write(15, crate::SMBUS_MAX_PAYLOAD)
            .unwrap_err();
        assert_eq!(err, WireError::WriteToReadOnly(RecoveryCommand::ProtCap));
        assert_eq!(ProtocolError::from(err), ProtocolError::UnsupportedCommand);
    }

    #[test]
    fn a_write_of_the_wrong_length_reports_length_write_error() {
        let err = RecoveryCommand::RecoveryCtrl
            .check_write(2, crate::SMBUS_MAX_PAYLOAD)
            .unwrap_err();
        assert_eq!(
            err,
            WireError::LengthMismatch {
                command: RecoveryCommand::RecoveryCtrl,
                got: 2,
            }
        );
        assert_eq!(ProtocolError::from(err), ProtocolError::LengthWriteError);
        assert!(RecoveryCommand::RecoveryCtrl
            .check_write(3, crate::SMBUS_MAX_PAYLOAD)
            .is_ok());
    }

    #[test]
    fn variable_length_commands_accept_the_head_alone_and_a_vendor_tail() {
        let device_status = RecoveryCommand::DeviceStatus.length();
        let cap = crate::SMBUS_MAX_PAYLOAD;
        assert!(device_status.accepts(7, cap));
        assert!(device_status.accepts(255, cap));
        assert!(!device_status.accepts(6, cap));
        assert!(!device_status.accepts(256, cap));
    }

    #[test]
    fn a_data_aperture_is_capped_by_the_transport_and_not_by_the_spec() {
        let aperture = RecoveryCommand::IndirectFifoData.length();
        assert!(aperture.accepts(crate::SMBUS_MAX_PAYLOAD, crate::SMBUS_MAX_PAYLOAD));
        assert!(!aperture.accepts(crate::SMBUS_MAX_PAYLOAD + 1, crate::SMBUS_MAX_PAYLOAD));
        assert!(!aperture.accepts(0, crate::SMBUS_MAX_PAYLOAD));

        // A transport that carries more carries more: the cap is the
        // transport's, not a constant this crate applies.
        assert!(aperture.accepts(crate::SMBUS_MAX_PAYLOAD + 1, 1024));
    }

    #[test]
    fn prot_cap_answers_before_recovery_starts() {
        assert_eq!(RecoveryCommand::ProtCap.scope(), Scope::Always);
        assert_eq!(
            RecoveryCommand::IndirectFifoCtrl.scope(),
            Scope::RecoveryMode
        );
    }
}
