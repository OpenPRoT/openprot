// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The orchestrator's side of the PLDM update gate.
//!
//! The firmware device asks before it acts: it parks at each phase and
//! raises a nudge, the orchestrator reads [`FdStatus`] and answers with
//! one operation. This crate turns a status into that answer.
//!
//! [`AlwaysGrant`] is the policy that permits everything. It exists so the
//! update path can run end to end before any real policy is written, and
//! so tests have a gate that never blocks. It makes no checks: no
//! isolation, no SVN floor, no component identity. Nothing here belongs on
//! a shipping device.

#![no_std]

use pldm_ipc_api::{FdStatus, PldmOp};

/// What the orchestrator sends next.
///
/// One variant per operation a decision can produce, carrying that
/// operation's arguments. `Idle` is not an operation: it means this status
/// needs no answer, so the orchestrator sends nothing and waits for the
/// next nudge.
///
/// Only the permitting operations are here. The refusals (`RejectOffer`,
/// `DenyVerify` and the rest) arrive with the first policy that refuses
/// something.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Nothing to answer: the FD is not waiting on the orchestrator.
    Idle,
    /// Approve the offer and name the staging region to write into.
    AcceptOffer { staging_base: u32 },
    /// Let the FD verify the staged image.
    GrantVerify,
    /// Let the FD apply the verified image.
    GrantApply,
    /// Let the FD activate.
    GrantActivate,
    /// Tell the FD the SVN floor is raised.
    GrantSvnCommit { component: u16 },
    /// Release the FD from a cancel it is parked on.
    AckCancel,
}

impl Decision {
    /// The operation this decision sends, or `None` for [`Decision::Idle`].
    pub fn op(&self) -> Option<PldmOp> {
        match self {
            Self::Idle => None,
            Self::AcceptOffer { .. } => Some(PldmOp::AcceptOffer),
            Self::GrantVerify => Some(PldmOp::GrantVerify),
            Self::GrantApply => Some(PldmOp::GrantApply),
            Self::GrantActivate => Some(PldmOp::GrantActivate),
            Self::GrantSvnCommit { .. } => Some(PldmOp::GrantSvnCommit),
            Self::AckCancel => Some(PldmOp::AckCancel),
        }
    }
}

/// A gate that permits every phase.
///
/// Answers each waiting status with its permitting operation and every
/// other status with [`Decision::Idle`]. The staging base it hands out at
/// `AcceptOffer` is the one it was built with, because that address is
/// board wiring rather than a decision.
///
/// This is a stand-in, not a policy. A real gate refuses an isolated
/// component, an image below the SVN floor, and an offer for a component
/// it does not manage. This one refuses nothing, so the update path runs
/// unattended and a test can drive every phase without writing a policy
/// first.
pub struct AlwaysGrant {
    staging_base: u32,
}

impl AlwaysGrant {
    /// Build a gate that stages every image at `staging_base`.
    pub const fn new(staging_base: u32) -> Self {
        Self { staging_base }
    }

    /// Answer one status.
    ///
    /// `PhaseFailed` is `Idle`: verify or apply already failed and the FD
    /// has told the update agent, so there is nothing left to permit.
    pub fn decide(&self, status: &FdStatus) -> Decision {
        match status {
            FdStatus::OfferPending { .. } => Decision::AcceptOffer {
                staging_base: self.staging_base,
            },
            FdStatus::VerifyPending => Decision::GrantVerify,
            FdStatus::ApplyPending => Decision::GrantApply,
            FdStatus::ActivationPending => Decision::GrantActivate,
            FdStatus::SvnCommitPending { component } => Decision::GrantSvnCommit {
                component: *component,
            },
            FdStatus::Cancelled => Decision::AckCancel,
            FdStatus::Idle { .. } | FdStatus::ReadyXfer | FdStatus::PhaseFailed { .. } => {
                Decision::Idle
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pldm_ipc_api::status::TransferMode;

    const STAGING: u32 = 0x2000_0000;

    fn gate() -> AlwaysGrant {
        AlwaysGrant::new(STAGING)
    }

    #[test]
    fn an_offer_is_accepted_at_the_configured_staging_base() {
        let offer = FdStatus::OfferPending {
            target: 1,
            total: 0x10_0000,
            mode: TransferMode::InTransport,
            svn_delayed: false,
        };

        assert_eq!(
            gate().decide(&offer),
            Decision::AcceptOffer {
                staging_base: STAGING
            }
        );
    }

    #[test]
    fn every_waiting_phase_is_permitted() {
        let g = gate();

        assert_eq!(g.decide(&FdStatus::VerifyPending), Decision::GrantVerify);
        assert_eq!(g.decide(&FdStatus::ApplyPending), Decision::GrantApply);
        assert_eq!(
            g.decide(&FdStatus::ActivationPending),
            Decision::GrantActivate
        );
        assert_eq!(
            g.decide(&FdStatus::SvnCommitPending { component: 7 }),
            Decision::GrantSvnCommit { component: 7 }
        );
    }

    #[test]
    fn a_cancel_is_acknowledged() {
        assert_eq!(gate().decide(&FdStatus::Cancelled), Decision::AckCancel);
    }

    #[test]
    fn a_status_that_is_not_waiting_gets_no_answer() {
        let g = gate();

        assert_eq!(g.decide(&FdStatus::Idle { reason: 0 }), Decision::Idle);
        assert_eq!(g.decide(&FdStatus::ReadyXfer), Decision::Idle);
        assert_eq!(
            g.decide(&FdStatus::PhaseFailed {
                phase: 6,
                result_code: 2
            }),
            Decision::Idle
        );
    }

    #[test]
    fn a_decision_names_the_operation_it_sends() {
        assert_eq!(Decision::Idle.op(), None);
        assert_eq!(
            Decision::AcceptOffer { staging_base: 0 }.op(),
            Some(PldmOp::AcceptOffer)
        );
        assert_eq!(Decision::GrantVerify.op(), Some(PldmOp::GrantVerify));
        assert_eq!(Decision::AckCancel.op(), Some(PldmOp::AckCancel));
    }

    /// The gate never refuses: no status produces a Reject or Deny
    /// operation. This is the property that makes it a stand-in and not a
    /// policy, so it is worth pinning.
    #[test]
    fn no_status_produces_a_refusal() {
        let g = gate();
        let every_status = [
            FdStatus::Idle { reason: 0 },
            FdStatus::ReadyXfer,
            FdStatus::OfferPending {
                target: 1,
                total: 16,
                mode: TransferMode::InTransport,
                svn_delayed: true,
            },
            FdStatus::VerifyPending,
            FdStatus::ApplyPending,
            FdStatus::ActivationPending,
            FdStatus::SvnCommitPending { component: 0 },
            FdStatus::PhaseFailed {
                phase: 6,
                result_code: 1,
            },
            FdStatus::Cancelled,
        ];

        for status in every_status {
            let refused = matches!(
                g.decide(&status).op(),
                Some(
                    PldmOp::RejectOffer
                        | PldmOp::DenyVerify
                        | PldmOp::DenyApply
                        | PldmOp::DenyActivate
                        | PldmOp::DenySvnCommit
                )
            );
            assert!(!refused, "refused {status:?}");
        }
    }
}
