// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The stand-in gate: every phase is told to proceed.

use crate::Decision;
use orchestrator_config::Region;
use pldm_api::FdStatus;

/// A gate that answers every waiting phase with proceed.
///
/// Answers each waiting status with the operation that tells the FD to
/// proceed, and every other status with [`Decision::Idle`]. The staging base it hands out at
/// `AcceptOffer` is the one it was built with, because that address is
/// board wiring rather than a decision.
///
/// This is a stand-in, not a policy. A real gate refuses an isolated
/// component, an image below the SVN floor, and an offer for a component
/// it does not manage. This one refuses nothing, so the update path runs
/// unattended and a test can drive every phase without writing a policy
/// first.
pub struct AlwaysPerform {
    staging: Region,
}

impl AlwaysPerform {
    /// Build a gate that stages every image in `staging`.
    pub const fn new(staging: Region) -> Self {
        Self { staging }
    }

    /// Answer one status.
    ///
    /// `PhaseFailed` is `Idle`: verify or apply already failed and the FD
    /// has told the update agent, so there is nothing left to answer.
    ///
    /// A policy that refuses a phase sends the matching `Reject` op. This
    /// one never refuses, so those ops never appear here.
    pub fn decide(&self, status: FdStatus) -> Decision {
        match status {
            FdStatus::OfferPending { .. } => Decision::AcceptOffer {
                staging: self.staging,
            },
            FdStatus::VerifyPending => Decision::PerformVerify,
            FdStatus::ApplyPending => Decision::PerformApply,
            FdStatus::ActivationPending => Decision::PerformActivate,
            FdStatus::SvnCommitPending { component } => Decision::PerformSvnCommit { component },
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
    use pldm_api::status::TransferMode;
    use pldm_api::PldmOp;

    const STAGING: Region = Region::new(0x2000_0000, 0x10_0000);

    fn gate() -> AlwaysPerform {
        AlwaysPerform::new(STAGING)
    }

    #[test]
    fn an_offer_is_accepted_into_the_configured_staging_region() {
        let offer = FdStatus::OfferPending {
            target: 1,
            total: 0x10_0000,
            mode: TransferMode::InTransport,
            svn_delayed: false,
        };

        assert_eq!(
            gate().decide(offer),
            Decision::AcceptOffer { staging: STAGING }
        );
    }

    #[test]
    fn every_waiting_phase_is_told_to_proceed() {
        let g = gate();

        assert_eq!(g.decide(FdStatus::VerifyPending), Decision::PerformVerify);
        assert_eq!(g.decide(FdStatus::ApplyPending), Decision::PerformApply);
        assert_eq!(
            g.decide(FdStatus::ActivationPending),
            Decision::PerformActivate
        );
        assert_eq!(
            g.decide(FdStatus::SvnCommitPending { component: 7 }),
            Decision::PerformSvnCommit { component: 7 }
        );
    }

    #[test]
    fn a_cancel_is_acknowledged() {
        assert_eq!(gate().decide(FdStatus::Cancelled), Decision::AckCancel);
    }

    #[test]
    fn a_status_that_is_not_waiting_gets_no_answer() {
        let g = gate();

        assert_eq!(g.decide(FdStatus::Idle { reason: 0 }), Decision::Idle);
        assert_eq!(g.decide(FdStatus::ReadyXfer), Decision::Idle);
        assert_eq!(
            g.decide(FdStatus::PhaseFailed {
                phase: 6,
                result_code: 2
            }),
            Decision::Idle
        );
    }

    /// The gate never refuses: no status produces a `Reject` operation.
    /// That is what makes it a stand-in and not a policy, so it is worth
    /// pinning.
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
                g.decide(status).op(),
                Some(
                    PldmOp::RejectOffer
                        | PldmOp::RejectVerify
                        | PldmOp::RejectApply
                        | PldmOp::RejectActivate
                        | PldmOp::RejectSvnCommit
                )
            );
            assert!(!refused, "refused {status:?}");
        }
    }
}
