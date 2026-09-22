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

// Wire discriminants.
const IDLE: u8 = 0;
const READY_XFER: u8 = 1;
const OFFER_PENDING: u8 = 2;
const VERIFY_PENDING: u8 = 3;
const APPLY_PENDING: u8 = 4;
const ACTIVATION_PENDING: u8 = 5;
const SVN_COMMIT_PENDING: u8 = 6;
const PHASE_FAILED: u8 = 7;
const CANCELLED: u8 = 8;

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
                buf[0] = IDLE;
                buf[1] = reason;
                Ok(2)
            }
            Self::ReadyXfer => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = READY_XFER;
                Ok(1)
            }
            Self::OfferPending {
                target,
                total,
                mode,
                svn_delayed,
            } => {
                if buf.len() < 9 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = OFFER_PENDING;
                let t = target.to_le_bytes();
                buf[1] = t[0];
                buf[2] = t[1];
                let s = total.to_le_bytes();
                buf[3] = s[0];
                buf[4] = s[1];
                buf[5] = s[2];
                buf[6] = s[3];
                buf[7] = mode as u8;
                buf[8] = svn_delayed as u8;
                Ok(9)
            }
            Self::VerifyPending => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = VERIFY_PENDING;
                Ok(1)
            }
            Self::ApplyPending => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = APPLY_PENDING;
                Ok(1)
            }
            Self::ActivationPending => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = ACTIVATION_PENDING;
                Ok(1)
            }
            Self::SvnCommitPending { component } => {
                if buf.len() < 3 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = SVN_COMMIT_PENDING;
                let c = component.to_le_bytes();
                buf[1] = c[0];
                buf[2] = c[1];
                Ok(3)
            }
            Self::PhaseFailed { phase, result_code } => {
                if buf.len() < 3 {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = PHASE_FAILED;
                buf[1] = phase;
                buf[2] = result_code;
                Ok(3)
            }
            Self::Cancelled => {
                if buf.is_empty() {
                    return Err(WireError::BufferTooSmall);
                }
                buf[0] = CANCELLED;
                Ok(1)
            }
        }
    }

    /// Decode from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        if buf.is_empty() {
            return Err(WireError::Truncated);
        }
        match buf[0] {
            IDLE => {
                if buf.len() < 2 {
                    return Err(WireError::Truncated);
                }
                Ok(Self::Idle { reason: buf[1] })
            }
            READY_XFER => Ok(Self::ReadyXfer),
            OFFER_PENDING => {
                if buf.len() < 9 {
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
            VERIFY_PENDING => Ok(Self::VerifyPending),
            APPLY_PENDING => Ok(Self::ApplyPending),
            ACTIVATION_PENDING => Ok(Self::ActivationPending),
            SVN_COMMIT_PENDING => {
                if buf.len() < 3 {
                    return Err(WireError::Truncated);
                }
                let component = u16::from_le_bytes([buf[1], buf[2]]);
                Ok(Self::SvnCommitPending { component })
            }
            PHASE_FAILED => {
                if buf.len() < 3 {
                    return Err(WireError::Truncated);
                }
                Ok(Self::PhaseFailed {
                    phase: buf[1],
                    result_code: buf[2],
                })
            }
            CANCELLED => Ok(Self::Cancelled),
            other => Err(WireError::InvalidValue(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_roundtrip() {
        let s = FdStatus::Idle { reason: 0x03 };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, 2);
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn offer_pending_roundtrip() {
        let s = FdStatus::OfferPending {
            target: 0x1234,
            total: 0x0010_0000,
            mode: TransferMode::InTransport,
            svn_delayed: false,
        };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, 9);
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
        let s = FdStatus::SvnCommitPending { component: 0x00FF };
        let mut buf = [0u8; 16];
        let len = s.encode(&mut buf).unwrap();
        assert_eq!(len, 3);
        assert_eq!(FdStatus::decode(&buf[..len]), Ok(s));
    }

    #[test]
    fn phase_failed_roundtrip() {
        let s = FdStatus::PhaseFailed {
            phase: 2,
            result_code: 0x0A,
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
            FdStatus::decode(&[OFFER_PENDING, 0, 0]),
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
