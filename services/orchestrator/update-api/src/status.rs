// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the orchestrator answers: the phase the source polls
//! ([`IntakeStatus`]), plus one failure vocabulary for a request refused on
//! the spot ([`Reject`]) and one for a job that ran and failed
//! ([`FailureCause`]).

/// Bytes moved so far out of the total the offer declared.
///
/// The wire form of `StageProgress::Transferring`: `written` is monotonic
/// and may hold still across polls, so a source watching for a stall keys on
/// the value, not on the poll count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes moved so far, at or below `total`.
    pub written: u64,
    /// Total payload bytes, from the accepted offer.
    pub total: u64,
}

/// The phase of the update in flight.
///
/// The source maps it onto what its own protocol owes its peer: a PLDM
/// firmware device turns [`Receiving`](Self::Receiving) into a transfer in
/// progress, [`Authenticating`](Self::Authenticating) into `VerifyPending`,
/// [`Staging`](Self::Staging) into apply progress, and
/// [`Failed`](Self::Failed) into the matching result code.
///
/// The order is not fixed and a source must not assume one. A board that
/// writes through to the target's inactive slot authenticates by reading
/// that slot back, so it runs [`Receiving`](Self::Receiving),
/// [`Authenticating`](Self::Authenticating), [`Activated`](Self::Activated).
/// A target whose gate is a signed manifest is authenticated before any byte
/// moves, which swaps the first two.
///
/// Intentionally exhaustive: adding a phase is a breaking change, so every
/// source handles each one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntakeStatus {
    /// No job: none offered, or the last one collected by a fresh offer.
    Idle,
    /// The source is writing the candidate into the staging region, which
    /// on a write-through board is the target's inactive slot. `written`
    /// counts what the orchestrator took, so a source that lost track can
    /// resume.
    Receiving(Progress),
    /// The orchestrator is authenticating the complete candidate. No
    /// counters: authentication is one verdict, not a transfer.
    Authenticating,
    /// The candidate authenticated; the orchestrator is pushing it to the
    /// target device, one polled step at a time.
    ///
    /// Only a board that stages into its own region ever reports this. Under
    /// write-through there is nothing left to push, so the phase goes
    /// straight from [`Authenticating`](Self::Authenticating) to
    /// [`Activated`](Self::Activated).
    Staging(Progress),
    /// The staged image is the device's boot candidate, tentatively. The
    /// commit gate is orchestrator policy, so this is the last phase a
    /// successful update shows the source.
    Activated,
    /// The job ended without activating. Terminal: it holds until the next
    /// accepted offer, so a source that polls late still learns the outcome.
    Failed(FailureCause),
}

/// Why a job that started did not activate.
///
/// Coarse on purpose, mirroring `UpdateError`: the source reports an outcome
/// and retries or does not, and the orchestrator logs the concrete fault
/// while it is still in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCause {
    /// The candidate failed authentication. Not retriable with these bytes.
    Authentication,
    /// The state machine was mid-walk, mid-recovery, or mid-update when the
    /// candidate completed, so it refused the update. The candidate is
    /// discarded, not judged: offering it again later may succeed.
    Deferred,
    /// Recovery preempted the update after it started. Retriable like
    /// [`Deferred`](Self::Deferred), different cause: the platform took the
    /// update away, not the timing.
    Superseded,
    /// The device or the staging region failed the transfer, or activation
    /// failed. Offering again may succeed.
    Device,
}

/// Why the orchestrator refused a request outright.
///
/// A [`Reject`] answers the request itself and never leaves a job
/// half-started: nothing was written, no phase changed. Contrast
/// [`FailureCause`], which is a job that ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// The offer named a target the board's device table does not declare.
    UnknownTarget,
    /// An offer while a job is in flight. That job is untouched: a second
    /// source cannot write into a region that is mid-authenticate.
    Busy,
    /// A write or complete with no accepted offer, or after the job left
    /// [`IntakeStatus::Receiving`].
    NoJob,
    /// A write range outside the offered payload, or an offer larger than
    /// the staging region.
    OutOfRange,
    /// An offer of zero bytes, or a complete before every offered byte was
    /// written. There is no candidate, so authentication must not run.
    Incomplete,
    /// The staging write failed. Possibly transient.
    Storage,
    /// The request did not decode (see [`wire::WireError`](crate::wire::WireError)).
    /// Two sides built from this crate never see it; the request side is
    /// another process and is treated as untrusted.
    Malformed,
}
