// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`Shell`] and its effect executors, one method per [`Effect`] variant,
//! routed from the SM through the [`Platform`] impl.

use openprot_orchestrator_sm::{ComponentId, Effect, EffectError, Event, Platform};

use crate::board::{Board, BoardTypes, ImageSource, Verdict, Verifier};

/// Bound on events queued between two driver rounds. Executors produce at
/// most one event per effect and the driver drains after every dispatch.
const EVENT_CAP: usize = 4;

/// Why the shell could not carry out an effect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShellError {
    /// The executor for this effect has not been written yet.
    NotImplemented,
    /// The effect names a component the shell has no device for.
    UnknownComponent,
    /// The component's image source could not be opened.
    ImageUnavailable,
    /// Verify was asked for a component whose image was never staged.
    NoImage,
    /// The verifier could not perform the check (not a failed image — that
    /// is a [`Verdict`], reported as an event).
    VerifierFault,
    /// The event queue overflowed; the verdict would have been lost, and
    /// dropping events breaks the SM's honest-feedback contract.
    QueueFull,
}

impl core::fmt::Display for ShellError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            ShellError::NotImplemented => "executor not implemented",
            ShellError::UnknownComponent => "no device for this component id",
            ShellError::ImageUnavailable => "image source could not be opened",
            ShellError::NoImage => "no image staged for this component",
            ShellError::VerifierFault => "verifier could not perform the check",
            ShellError::QueueFull => "event queue full",
        })
    }
}

impl core::error::Error for ShellError {}

/// The effect executors, one method per [`Effect`] variant. Everything
/// device-specific lives in the [`Board`] bundle the composition crate hands
/// to [`new`](Self::new); the shell's own fields are pure bookkeeping.
pub struct Shell<B: BoardTypes, const N: usize> {
    board: Board<B, N>,
    /// Which component's image is staged (its source opened) for the
    /// verification that follows.
    staged: Option<ComponentId>,
    /// Events produced by executors, awaiting [`take_event`](Self::take_event).
    pending: heapless::Deque<Event, EVENT_CAP>,
}

impl<B: BoardTypes, const N: usize> Shell<B, N> {
    pub fn new(board: Board<B, N>) -> Self {
        Self {
            board,
            staged: None,
            pending: heapless::Deque::new(),
        }
    }

    /// Next event owed to the SM, if any. The driver loop drains this with
    /// `dispatch` after each event settles.
    pub fn take_event(&mut self) -> Option<Event> {
        self.pending.pop_front()
    }

    fn enqueue(&mut self, event: Event) -> Result<(), ShellError> {
        self.pending
            .push_back(event)
            .map_err(|_| ShellError::QueueFull)
    }

    /// Stage `id`'s active image: open its source (claim the interposed
    /// flash, start the transfer session) so
    /// [`verify_firmware`](Self::verify_firmware) can stream it.
    pub fn read_firmware(&mut self, id: ComponentId) -> Result<(), ShellError> {
        self.staged = None;
        let source = self
            .board
            .images
            .get_mut(id.get() as usize)
            .ok_or(ShellError::UnknownComponent)?;
        source.open().map_err(|_| ShellError::ImageUnavailable)?;
        self.staged = Some(id);
        Ok(())
    }

    /// Have the [`Verifier`] judge the image staged for `id` and queue the
    /// verdict — `Event::VerificationPassed(id)` or
    /// `Event::VerificationFailed(id)`. An image the shell could not stage or
    /// check is a failed *actuation* (an error here, latching the SM), never
    /// a forged verdict.
    pub fn verify_firmware(&mut self, id: ComponentId) -> Result<(), ShellError> {
        if self.staged != Some(id) {
            return Err(ShellError::NoImage);
        }
        let source = self
            .board
            .images
            .get_mut(id.get() as usize)
            .ok_or(ShellError::UnknownComponent)?;
        let verdict = self
            .board
            .verifier
            .verify(id, source)
            .map_err(|_| ShellError::VerifierFault)?;
        self.enqueue(match verdict {
            Verdict::Authentic => Event::VerificationPassed(id),
            Verdict::Rejected => Event::VerificationFailed(id),
        })
    }

    /// Release `id` from reset, then supervise its boot: walk the device's
    /// checkpoints and feed back one `Event::ComponentReady(id)` (Active) or
    /// `Event::Booted(id)` (Passive), or `Event::Timeout(id)` when a
    /// checkpoint window expires. Aggregation is the shell's job — the SM
    /// sees a single readiness event per component.
    pub fn release_reset(&mut self, _id: ComponentId) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Hold `id` in reset — durable quiesce, not a pulse: `id` must not
    /// execute until its next release. At-rest verification and the recovery
    /// re-walk depend on this.
    pub fn assert_reset(&mut self, _id: ComponentId) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Restore `id` from its configured recovery source (golden image, A/B
    /// slot, streamed image — a config decision, not the SM's); feed back
    /// `Event::Restored(id)`, or `Event::RecoveryFailed` when restore fails.
    pub fn recover_component(&mut self, _id: ComponentId) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Authenticate the staged update image; feed back
    /// `Event::UpdateVerified` or `Event::UpdateRejected`.
    pub fn authenticate_update(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Write the incoming update image into the staging region.
    pub fn stage_update(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Boot the staged image tentatively (trial boot) and arm the commit
    /// watchdog; feed back `Event::BootConfirmed(id)` on proven health or
    /// `Event::CommitTimeout` when the policy window expires.
    pub fn activate_update(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Discard the staged image (rejected or orphaned by recovery).
    pub fn discard_staged(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Advance the anti-rollback (SVN) floor past `id`'s now-confirmed image
    /// and cancel the commit watchdog armed by
    /// [`activate_update`](Self::activate_update).
    pub fn commit_svn_floor(&mut self, _id: ComponentId) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Produce a signed attestation for the pending challenge.
    pub fn sign_attestation(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Report through platform management that `id` is gated and the
    /// platform runs degraded (CSA degraded-mode clause).
    pub fn report_isolated(&mut self, _id: ComponentId) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Report that `id` exhausted recovery and forced the halt, immediately
    /// before the machine latches `Locked`.
    pub fn report_recovery_failed(&mut self, _id: ComponentId) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Answer the requester that its update was declined because the machine
    /// is busy (e.g. a PLDM "retry later" completion code).
    pub fn report_update_deferred(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Answer the requester that its in-flight update was superseded by
    /// recovery and may be retried once the platform is whole.
    pub fn report_update_aborted(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }

    /// Latch the terminal safe state. A failure here is a hard fault: the SM
    /// has nothing stronger to emit and will believe it is `Locked`, so the
    /// real executor must treat failure as terminal (halt), not recoverable.
    pub fn latch_lockdown(&mut self) -> Result<(), ShellError> {
        Err(ShellError::NotImplemented)
    }
}

impl<B: BoardTypes, const N: usize> Platform for Shell<B, N> {
    /// Routes each effect to its executor. Exhaustive on purpose: a new
    /// [`Effect`] variant must be given an executor before this compiles.
    /// Any executor error is reported as [`EffectError`] — the SM treats
    /// every actuation failure the same, fail-closed.
    fn execute(&mut self, effect: Effect) -> Result<(), EffectError> {
        match effect {
            Effect::ReadFirmware(id) => self.read_firmware(id),
            Effect::VerifyFirmware(id) => self.verify_firmware(id),
            Effect::ReleaseReset(id) => self.release_reset(id),
            Effect::AssertReset(id) => self.assert_reset(id),
            Effect::RecoverComponent(id) => self.recover_component(id),
            Effect::AuthenticateUpdate => self.authenticate_update(),
            Effect::StageUpdate => self.stage_update(),
            Effect::ActivateUpdate => self.activate_update(),
            Effect::DiscardStaged => self.discard_staged(),
            Effect::CommitSvnFloor(id) => self.commit_svn_floor(id),
            Effect::SignAttestation => self.sign_attestation(),
            Effect::ReportIsolated(id) => self.report_isolated(id),
            Effect::ReportRecoveryFailed(id) => self.report_recovery_failed(id),
            Effect::ReportUpdateDeferred => self.report_update_deferred(),
            Effect::ReportUpdateAborted => self.report_update_aborted(),
            Effect::LatchLockdown => self.latch_lockdown(),
            // The orchestrator consumes `Emit` internally; receiving one is a
            // driver bug — fail closed.
            Effect::Emit(_) => return Err(EffectError),
        }
        .map_err(|_| EffectError)
    }
}
