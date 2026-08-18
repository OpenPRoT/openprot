// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`PlatformDriver`]: one executor method per [`Effect`] variant, routed from
//! the SM through the [`Platform`] impl.

use openprot_orchestrator_sm::{ComponentId, Effect, EffectError, Event, Platform};

use crate::board::{Board, BoardTypes, ImageSource, Verdict, Verifier};

/// Queue bound. Executors produce at most one event per effect, the event loop
/// drains it after every dispatch, and the largest SM effect batch today is
/// two (ReadFirmware + VerifyFirmware) — 4 is that worst case with
/// headroom. Overflow is reported ([`DriverError::QueueFull`]), never
/// silent loss.
const EVENT_CAP: usize = 4;

/// Why the driver could not carry out an effect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DriverError {
    /// The executor for this effect has not been written yet.
    NotImplemented,
    /// The effect names a component the driver has no device for.
    UnknownComponent,
    /// The component's image source could not be opened.
    ImageUnavailable,
    /// Verify was asked for a component whose image was never staged.
    NotStaged,
    /// The verifier could not perform the check (a failed image is a
    /// [`Verdict`], not an error).
    VerifierFault,
    /// The event queue overflowed; dropping events breaks the SM's
    /// honest-feedback contract.
    QueueFull,
}

impl core::fmt::Display for DriverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            DriverError::NotImplemented => "executor not implemented",
            DriverError::UnknownComponent => "no device for this component id",
            DriverError::ImageUnavailable => "image source could not be opened",
            DriverError::NotStaged => "no image staged for this component",
            DriverError::VerifierFault => "verifier could not perform the check",
            DriverError::QueueFull => "event queue full",
        })
    }
}

impl core::error::Error for DriverError {}

/// The effect executors. Everything device-specific lives in the [`Board`];
/// the driver's own fields are bookkeeping.
pub struct PlatformDriver<B: BoardTypes, const N: usize> {
    board: Board<B, N>,
    /// Component whose image is staged (source opened) for verification.
    staged: Option<ComponentId>,
    /// Events awaiting [`take_event`](Self::take_event).
    pending: heapless::Deque<Event, EVENT_CAP>,
}

impl<B: BoardTypes, const N: usize> PlatformDriver<B, N> {
    pub fn new(board: Board<B, N>) -> Self {
        Self {
            board,
            staged: None,
            pending: heapless::Deque::new(),
        }
    }

    /// Next event owed to the SM; the event loop drains this after each
    /// dispatch.
    pub fn take_event(&mut self) -> Option<Event> {
        self.pending.pop_front()
    }

    fn enqueue(&mut self, event: Event) -> Result<(), DriverError> {
        self.pending
            .push_back(event)
            .map_err(|_| DriverError::QueueFull)
    }

    /// Stage `id`'s image: open its source so
    /// [`verify_firmware`](Self::verify_firmware) can read it.
    pub fn stage_firmware(&mut self, id: ComponentId) -> Result<(), DriverError> {
        self.staged = None;
        let source = self
            .board
            .images
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        source.open().map_err(|_| DriverError::ImageUnavailable)?;
        self.staged = Some(id);
        Ok(())
    }

    /// Judge the staged image via the [`Verifier`] and queue the verdict:
    /// `Event::VerificationPassed(id)` or `Event::VerificationFailed(id)`.
    pub fn verify_firmware(&mut self, id: ComponentId) -> Result<(), DriverError> {
        if self.staged != Some(id) {
            return Err(DriverError::NotStaged);
        }
        let source = self
            .board
            .images
            .get_mut(id.get() as usize)
            .ok_or(DriverError::UnknownComponent)?;
        let verdict = self
            .board
            .verifier
            .verify(id, source)
            .map_err(|_| DriverError::VerifierFault)?;
        self.enqueue(match verdict {
            Verdict::Authenticated => Event::VerificationPassed(id),
            Verdict::Rejected => Event::VerificationFailed(id),
        })
    }

    /// Release `id` from reset, then walk its boot checkpoints; feed back
    /// one `Event::ComponentReady(id)` (Active) or `Event::Booted(id)`
    /// (Passive), or `Event::Timeout(id)` on window expiry.
    pub fn release_reset(&mut self, _id: ComponentId) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Hold `id` in reset — a durable quiesce, not a pulse; at-rest
    /// verification and the recovery re-walk depend on it.
    pub fn assert_reset(&mut self, _id: ComponentId) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Restore `id` from its configured recovery source (the mechanism is
    /// board config); feed back `Event::Restored(id)` or
    /// `Event::RecoveryFailed`.
    pub fn recover_component(&mut self, _id: ComponentId) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Authenticate the staged update; feed back `Event::UpdateVerified` or
    /// `Event::UpdateRejected`.
    pub fn authenticate_update(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Write the incoming update image into the staging region.
    pub fn stage_update(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Trial-boot the staged image and arm the commit watchdog; feed back
    /// `Event::BootConfirmed(id)` or `Event::CommitTimeout`.
    pub fn activate_update(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Discard the staged image.
    pub fn discard_staged(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Advance the SVN floor past `id`'s confirmed image; cancels the
    /// commit watchdog armed by [`activate_update`](Self::activate_update).
    pub fn commit_svn_floor(&mut self, _id: ComponentId) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Produce a signed attestation for the pending challenge.
    pub fn sign_attestation(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Report `id` gated and the platform degraded (CSA degraded-mode
    /// clause).
    pub fn report_isolated(&mut self, _id: ComponentId) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Report that `id` exhausted recovery, immediately before the machine
    /// latches `Locked`.
    pub fn report_recovery_failed(&mut self, _id: ComponentId) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Answer the requester: update declined, machine busy (e.g. a PLDM
    /// "retry later" completion code).
    pub fn report_update_deferred(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Answer the requester: its in-flight update was superseded by
    /// recovery.
    pub fn report_update_aborted(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }

    /// Latch the terminal safe state. A failure here is a hard fault: the
    /// SM believes it is `Locked`, so the real executor must halt, not
    /// recover.
    pub fn latch_lockdown(&mut self) -> Result<(), DriverError> {
        Err(DriverError::NotImplemented)
    }
}

impl<B: BoardTypes, const N: usize> Platform for PlatformDriver<B, N> {
    /// Routes each effect to its executor. Exhaustive: a new [`Effect`]
    /// variant must get an executor before this compiles. Every executor
    /// error reports as [`EffectError`] — the SM treats all actuation
    /// failures the same, fail-closed.
    fn execute(&mut self, effect: Effect) -> Result<(), EffectError> {
        match effect {
            Effect::ReadFirmware(id) => self.stage_firmware(id),
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
            // Emit is consumed by the orchestrator; receiving one is a
            // driver bug.
            Effect::Emit(_) => return Err(EffectError),
        }
        .map_err(|_| EffectError)
    }
}
