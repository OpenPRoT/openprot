// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`UpdateIntake`] seam: what an update source calls on the
//! orchestrator.

use crate::{IntakeStatus, Reject, TargetId};

/// Why an [`UpdateIntake`] call did not return an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntakeError {
    /// The orchestrator answered, and the answer is a refusal. The request
    /// had no effect.
    Rejected(Reject),
    /// The call did not reach the orchestrator, or its answer did not decode.
    /// The request may or may not have taken effect, so a source that cares
    /// re-reads the phase with [`poll`](UpdateIntake::poll).
    Transport,
}

impl core::fmt::Display for IntakeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IntakeError::Rejected(_) => f.write_str("the orchestrator refused the request"),
            IntakeError::Transport => f.write_str("the intake call did not complete"),
        }
    }
}

impl core::error::Error for IntakeError {}

/// The orchestrator's update intake, as an update source calls it.
///
/// A source carries bytes and holds no authority: it never names a slot,
/// never verifies, never decides that an update runs. It offers a candidate,
/// fills the staging region, says when the candidate is complete, and reports
/// the outcome to its own peer.
///
/// One update: [`offer`](Self::offer) the target and length,
/// [`write`](Self::write) the payload at whatever offsets the source's
/// protocol delivers, [`complete`](Self::complete) once every offered byte is
/// written, then [`poll`](Self::poll) until [`IntakeStatus::Activated`] or
/// [`IntakeStatus::Failed`]. [`abort`](Self::abort) drops the job from any
/// phase. Activation needs no call: the state machine activates on its own
/// verdict once the candidate authenticates.
///
/// The source is the channel's initiator and the orchestrator its handler,
/// with no channel the other way, so the orchestrator never waits on the
/// source and a wedged source cannot delay a boot window or a recovery. See
/// the crate docs for what follows from that.
///
/// The methods take `&self` so a source can call through a shared reference
/// from its protocol code (`FdOps` and friends are all `&self`); an
/// implementation over a kernel channel keeps its buffers in a `RefCell`, as
/// the MCTP IPC client does.
pub trait UpdateIntake {
    /// Offers a candidate of `total` bytes for `target`, reserving the
    /// staging region.
    ///
    /// The orchestrator validates the target against its device table and
    /// the length against the region. It does not consult the state machine,
    /// so acceptance is not a promise that the update will run. An offer
    /// while a job is in flight is [`Reject::Busy`] and leaves that job
    /// untouched; a zero-length offer is [`Reject::Incomplete`].
    ///
    /// A fresh accepted offer collects the previous job's terminal phase.
    fn offer(&self, target: TargetId, total: u64) -> Result<(), IntakeError>;

    /// Writes `bytes` into the staging region at `offset`, relative to the
    /// start of the offered payload.
    ///
    /// One call is one staging write of at most
    /// [`MAX_CHUNK`](crate::wire::MAX_CHUNK) bytes. Ranges outside the
    /// payload are [`Reject::OutOfRange`]; a repeated range overwrites, so a
    /// retransmitting transfer needs no bookkeeping here.
    fn write(&self, offset: u64, bytes: &[u8]) -> Result<(), IntakeError>;

    /// Declares the candidate complete, which is what starts the update.
    ///
    /// The orchestrator checks that every offered byte was written and hands
    /// the state machine an update request. The verdict is not part of the
    /// answer: it arrives through [`poll`](Self::poll). A state machine that
    /// refuses the request surfaces as
    /// [`FailureCause::Deferred`](crate::FailureCause::Deferred), not as a
    /// [`Reject`] here.
    fn complete(&self) -> Result<(), IntakeError>;

    /// Drops the job: the staging region is released and any in-flight
    /// staging is abandoned.
    ///
    /// Legal in every phase, including with no job, so a source that lost
    /// track can always get back to [`IntakeStatus::Idle`]. The active image
    /// is untouched whenever this lands: the staging region is inactive by
    /// construction.
    fn abort(&self) -> Result<(), IntakeError>;

    /// Reads the current phase. One bounded read of a latched value, with no
    /// device or crypto work behind it; cadence is the source's choice.
    fn poll(&self) -> Result<IntakeStatus, IntakeError>;
}
