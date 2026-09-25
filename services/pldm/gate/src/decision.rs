// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the orchestrator answers a parked firmware device with.

use orchestrator_config::Region;
use pldm_api::PldmOp;

/// What the orchestrator sends next.
///
/// One variant per operation a decision can produce, carrying that
/// operation's arguments. `Idle` is not an operation: it means this status
/// needs no answer, so the orchestrator sends nothing and waits for the
/// next nudge.
///
/// Only the operations that tell the FD to proceed are here. The refusals
/// (`RejectOffer`, `RejectVerify` and the rest) arrive with the first
/// policy that refuses something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Nothing to answer: the FD is not waiting on the orchestrator.
    Idle,
    /// Approve the offer and name the staging region: where the image
    /// lands in transport, where it already sits out of transport.
    ///
    /// The region is store-relative, like every [`Region`]. Which store,
    /// and so which address the firmware device sees, is board wiring.
    AcceptOffer { staging: Region },
    /// Tell the FD to verify the staged image.
    PerformVerify,
    /// Tell the FD to apply the verified image.
    PerformApply,
    /// Tell the FD to activate.
    PerformActivate,
    /// Tell the FD the SVN floor is raised. `component` names whose floor
    /// the orchestrator advances first; the wire op carries no arguments.
    PerformSvnCommit { component: u16 },
    /// Release the FD from a cancel it is parked on.
    AckCancel,
}

impl Decision {
    /// The operation this decision sends, or `None` for [`Decision::Idle`].
    pub fn op(&self) -> Option<PldmOp> {
        match self {
            Self::Idle => None,
            Self::AcceptOffer { .. } => Some(PldmOp::AcceptOffer),
            Self::PerformVerify => Some(PldmOp::PerformVerify),
            Self::PerformApply => Some(PldmOp::PerformApply),
            Self::PerformActivate => Some(PldmOp::PerformActivate),
            Self::PerformSvnCommit { .. } => Some(PldmOp::PerformSvnCommit),
            Self::AckCancel => Some(PldmOp::AckCancel),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decision_names_the_operation_it_sends() {
        assert_eq!(Decision::Idle.op(), None);
        assert_eq!(
            Decision::AcceptOffer {
                staging: Region::new(0x2000_0000, 0x10_0000)
            }
            .op(),
            Some(PldmOp::AcceptOffer)
        );
        assert_eq!(Decision::PerformVerify.op(), Some(PldmOp::PerformVerify));
        assert_eq!(Decision::AckCancel.op(), Some(PldmOp::AckCancel));
    }
}
