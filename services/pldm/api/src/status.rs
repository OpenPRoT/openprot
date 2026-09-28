// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! FD status for QueryStatus responses.
//!
//! The status payload follows the response header and carries the FD's
//! current condition. The orchestrator always follows a nudge with
//! QueryStatus to learn what happened, so the status is the primary
//! communication channel from the FD.

use crate::error::WireError;

/// In-transport vs out-of-transport image transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransferMode {
    /// FD pulls firmware chunks from the UA over MCTP.
    InTransport = 0,
    /// A third party writes the image to staging before verify.
    OutOfTransport = 1,
}

impl TransferMode {
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::InTransport),
            1 => Some(Self::OutOfTransport),
            _ => None,
        }
    }
}

/// Current condition of the FD, returned by QueryStatus.
///
/// Some variants map to DSP0267 states (Idle, ReadyXfer), some to
/// pending decisions the orchestrator owes the FD (OfferPending,
/// VerifyPending, ApplyPending, ActivationPending, SvnCommitPending),
/// and PhaseFailed is a verify/apply failure the UA has not yet
/// cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdStatus {
    /// No update in progress. `reason` is the DSP0267
    /// GetStatusReasonCode (0 = Initialization, others per spec).
    Idle { reason: u8 },

    /// UA has sent UpdateComponent, FD is ready for transfer.
    ReadyXfer,

    /// FD has an offer the orchestrator has not yet accepted or
    /// rejected. `target` is the PLDM component identifier, `total`
    /// is the image size in bytes. `svn_delayed` is true when the UA
    /// requested delayed SVN update (DSP0267 bit 9).
    OfferPending {
        target: u16,
        total: u32,
        mode: TransferMode,
        svn_delayed: bool,
    },

    /// Transfer complete, FD waiting for GrantVerify.
    VerifyPending,

    /// Verify complete, FD waiting for GrantApply.
    ApplyPending,

    /// Apply complete, FD waiting for GrantActivate.
    ActivationPending,

    /// UA sent UpdateSecurityRevision, FD waiting for
    /// GrantSvnCommit. `component` is the target identifier.
    SvnCommitPending { component: u16 },

    /// Verify or apply failed. `phase` and `result_code` are the
    /// DSP0267 values the FD already sent the UA.
    PhaseFailed { phase: u8, result_code: u8 },

    /// UA sent CancelUpdate, FD waiting for AckCancel.
    Cancelled,
}

/// Wire tag for [`FdStatus`] variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Tag {
    Idle = 0,
    ReadyXfer = 1,
    OfferPending = 2,
    VerifyPending = 3,
    ApplyPending = 4,
    ActivationPending = 5,
    SvnCommitPending = 6,
    PhaseFailed = 7,
    Cancelled = 8,
}

impl Tag {
    const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Idle),
            1 => Some(Self::ReadyXfer),
            2 => Some(Self::OfferPending),
            3 => Some(Self::VerifyPending),
            4 => Some(Self::ApplyPending),
            5 => Some(Self::ActivationPending),
            6 => Some(Self::SvnCommitPending),
            7 => Some(Self::PhaseFailed),
            8 => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Encode a single-byte (tag-only) variant.
fn encode_tag(buf: &mut [u8], tag: Tag) -> Result<usize, WireError> {
    if buf.is_empty() {
        return Err(WireError::BufferTooSmall);
    }
    buf[0] = tag as u8;
    Ok(1)
}

impl FdStatus {
    /// Maximum encoded size of a status payload (OfferPending: 9 bytes).
    pub const MAX_SIZE: usize = 9;

    /// Encode into `buf`, returning the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        match *self {
            Self::Idle { reason } => {
                if buf.len() < 2 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = Tag::Idle as u8;
                buf[1] = reason; // DSP0267 GetStatusReasonCode
                Ok(2)
            }
            Self::ReadyXfer => encode_tag(buf, Tag::ReadyXfer),
            Self::OfferPending {
                target,
                total,
                mode,
                svn_delayed,
            } => {
                if buf.len() < Self::MAX_SIZE {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = Tag::OfferPending as u8;
                buf[1..3].copy_from_slice(&target.to_le_bytes()); // component id
                buf[3..7].copy_from_slice(&total.to_le_bytes()); // image size
                buf[7] = mode as u8;
                buf[8] = svn_delayed as u8;
                Ok(Self::MAX_SIZE)
            }
            Self::VerifyPending => encode_tag(buf, Tag::VerifyPending),
            Self::ApplyPending => encode_tag(buf, Tag::ApplyPending),
            Self::ActivationPending => encode_tag(buf, Tag::ActivationPending),
            Self::SvnCommitPending { component } => {
                if buf.len() < 3 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = Tag::SvnCommitPending as u8;
                buf[1..3].copy_from_slice(&component.to_le_bytes()); // component id
                Ok(3)
            }
            Self::PhaseFailed { phase, result_code } => {
                if buf.len() < 3 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = Tag::PhaseFailed as u8;
                buf[1] = phase; // DSP0267 phase code
                buf[2] = result_code; // DSP0267 result code
                Ok(3)
            }
            Self::Cancelled => encode_tag(buf, Tag::Cancelled),
        }
    }

    /// Decode from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        if buf.is_empty() {
            return Err(WireError::Truncated);
        }
        let tag = Tag::from_u8(buf[0]).ok_or(WireError::InvalidValue(buf[0]))?;
        match tag {
            Tag::Idle => {
                if buf.len() < 2 {
                    return Err(WireError::Truncated);
                }
                Ok(Self::Idle { reason: buf[1] })
            }
            Tag::ReadyXfer => Ok(Self::ReadyXfer),
            Tag::OfferPending => {
                if buf.len() < Self::MAX_SIZE {
                    return Err(WireError::Truncated);
                }
                let target = u16::from_le_bytes([buf[1], buf[2]]);
                let total = u32::from_le_bytes([buf[3], buf[4], buf[5], buf[6]]);
                let mode = TransferMode::from_u8(buf[7]).ok_or(WireError::InvalidValue(buf[7]))?;
                let svn_delayed = buf[8] != 0;
                Ok(Self::OfferPending {
                    target,
                    total,
                    mode,
                    svn_delayed,
                })
            }
            Tag::VerifyPending => Ok(Self::VerifyPending),
            Tag::ApplyPending => Ok(Self::ApplyPending),
            Tag::ActivationPending => Ok(Self::ActivationPending),
            Tag::SvnCommitPending => {
                if buf.len() < 3 {
                    return Err(WireError::Truncated);
                }
                let component = u16::from_le_bytes([buf[1], buf[2]]);
                Ok(Self::SvnCommitPending { component })
            }
            Tag::PhaseFailed => {
                if buf.len() < 3 {
                    return Err(WireError::Truncated);
                }
                Ok(Self::PhaseFailed {
                    phase: buf[1],
                    result_code: buf[2],
                })
            }
            Tag::Cancelled => Ok(Self::Cancelled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_roundtrip() {
        // reason is a DSP0267 GetStatusReasonCode, arbitrary nonzero value
        let s = FdStatus::Idle { reason: 3 };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, 2);
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn offer_pending_roundtrip() {
        let s = FdStatus::OfferPending {
            target: 0x1234,     // component identifier
            total: 0x0010_0000, // 1 MiB image
            mode: TransferMode::InTransport,
            svn_delayed: false,
        };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, FdStatus::MAX_SIZE);
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn offer_pending_svn_delayed() {
        let s = FdStatus::OfferPending {
            target: 1,
            total: 4096,
            mode: TransferMode::OutOfTransport,
            svn_delayed: true,
        };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn simple_variants_roundtrip() {
        for s in [
            FdStatus::ReadyXfer,
            FdStatus::VerifyPending,
            FdStatus::ApplyPending,
            FdStatus::ActivationPending,
            FdStatus::Cancelled,
        ] {
            let mut buf = [0u8; 16];
            let len = s.encode(&mut buf).unwrap();
            assert_eq!(len, 1);
            assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
        }
    }

    #[test]
    fn svn_commit_pending_roundtrip() {
        let s = FdStatus::SvnCommitPending { component: 255 };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, 3);
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn phase_failed_roundtrip() {
        // phase and result_code are DSP0267 values, arbitrary here
        let s = FdStatus::PhaseFailed {
            phase: 2,
            result_code: 10,
        };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, 3);
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn decode_empty_is_truncated() {
        assert_eq!(FdStatus::decode(&[]), Err(WireError::Truncated));
    }

    #[test]
    fn decode_unknown_discriminant() {
        assert_eq!(
            FdStatus::decode(&[0xFF]),
            Err(WireError::InvalidValue(0xFF))
        );
    }

    #[test]
    fn decode_offer_pending_truncated() {
        assert_eq!(
            FdStatus::decode(&[Tag::OfferPending as u8, 0, 0]),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn encode_offer_pending_buffer_too_small() {
        let s = FdStatus::OfferPending {
            target: 1,
            total: 1,
            mode: TransferMode::InTransport,
            svn_delayed: false,
        };
        let mut buf = [0u8; 4];
        assert_eq!(s.encode(&mut buf), Err(WireError::BufferTooSmall));
    }
}
