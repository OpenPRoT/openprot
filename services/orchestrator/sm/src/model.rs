// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Domain vocabulary for the orchestrator state machine: the component model,
//! events, effects, states, and the validated [`Chain`] of trust. These types
//! carry no reducer behavior; the state machine itself lives in the crate root.

pub use orchestrator_config::ComponentId;
use orchestrator_config::DeviceConfig;

/// How a component in the trust chain is classified. The board supplies one
/// [`ComponentKind`] per [`ComponentId`] when building the chain.
///
/// Corresponds directly to the two-tier model in the CSA architecture document:
/// `Active` = eRoT gate + iRoT gate; `Passive` = eRoT gate only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComponentKind {
    /// Has an integrated iRoT (e.g. Caliptra). Both eRoT-side (signature + SVN)
    /// and iRoT-side (local self-verification) checks apply. The machine waits in
    /// [`State::AwaitingReady`] for [`Event::ComponentReady`] before advancing.
    Active,
    /// No integrated iRoT. The eRoT's signature + SVN check is the only *trust*
    /// gate, so the chain walk advances speculatively after `ReleaseReset`
    /// without blocking in [`State::AwaitingReady`]. The released component is
    /// still watched for boot-progress liveness ([`Event::Booted`]) under the
    /// same per-component watchdog as an `Active` component's
    /// [`Event::ComponentReady`]: a passive device that never reports in before
    /// its [`Event::Timeout`] is recovered like any other boot failure. CSA
    /// boot-progress checkpointing is device-agnostic — every released device
    /// owes a boot-progress signal, iRoT or not.
    Passive,
}

/// Recovery-failure classification: what the machine does once a required
/// component's restore attempts are **exhausted** (its per-component retry
/// count reaches `max_retry`). Every verification or corruption failure enters
/// [`State::Recovering`] and is retried first, regardless of this
/// classification — CSA's "recover first" principle. This value is consulted
/// only after retries are exhausted.
///
/// (The narrative design docs sometimes call the `Required` outcome "platform
/// halt" — same behavior, this is the type-level name.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailurePolicy {
    /// Stop the boot sequence entirely: self-emits [`Event::RecoveryFailed`],
    /// which drives the machine to [`State::Locked`].
    Required,
    /// Hold this component in reset (added to `Rot.gated`) and continue
    /// booting the rest of the platform.
    Isolable,
    /// Hold this component **and** any component whose `depends_on` names it
    /// (transitively), then continue booting the rest of the platform.
    Cascading,
}

/// Opaque recovery-region key supplied by the board at chain-build time.
/// Components sharing a `RegionId` are restored together: when any region
/// member enters [`State::Recovering`], the shell resolves and restores the
/// whole region. The core treats this as an equality key only and never
/// inspects membership itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegionId(u8);

impl RegionId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Per-component attributes supplied by the board at chain-build time.
///
/// Three orthogonal axes:
/// - [`kind`](ComponentAttrs::kind): controls the iRoT gate (Active vs Passive).
/// - [`failure_policy`](ComponentAttrs::failure_policy): controls what happens
///   once this component's recovery is exhausted.
/// - [`recovery_region`](ComponentAttrs::recovery_region) /
///   [`depends_on`](ComponentAttrs::depends_on): control restore grouping and
///   cascade-skip on exhaustion.
///
/// A component that fails verification is never released from reset —
/// running untrusted firmware would break the trust invariant regardless of
/// `failure_policy`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ComponentAttrs {
    pub kind: ComponentKind,
    pub failure_policy: FailurePolicy,
    pub recovery_region: RegionId,
    pub depends_on: Option<ComponentId>,
}

impl ComponentAttrs {
    pub const fn active_required() -> Self {
        Self {
            kind: ComponentKind::Active,
            failure_policy: FailurePolicy::Required,
            recovery_region: RegionId::new(0),
            depends_on: None,
        }
    }
    pub const fn passive_required() -> Self {
        Self {
            kind: ComponentKind::Passive,
            failure_policy: FailurePolicy::Required,
            recovery_region: RegionId::new(0),
            depends_on: None,
        }
    }
    pub const fn active_isolable() -> Self {
        Self {
            kind: ComponentKind::Active,
            failure_policy: FailurePolicy::Isolable,
            recovery_region: RegionId::new(0),
            depends_on: None,
        }
    }
    pub const fn passive_isolable() -> Self {
        Self {
            kind: ComponentKind::Passive,
            failure_policy: FailurePolicy::Isolable,
            recovery_region: RegionId::new(0),
            depends_on: None,
        }
    }
    pub const fn active_cascading() -> Self {
        Self {
            kind: ComponentKind::Active,
            failure_policy: FailurePolicy::Cascading,
            recovery_region: RegionId::new(0),
            depends_on: None,
        }
    }
    pub const fn passive_cascading() -> Self {
        Self {
            kind: ComponentKind::Passive,
            failure_policy: FailurePolicy::Cascading,
            recovery_region: RegionId::new(0),
            depends_on: None,
        }
    }

    /// Builder: assign a non-default recovery region (default is region `0`).
    pub const fn with_region(mut self, region: RegionId) -> Self {
        self.recovery_region = region;
        self
    }

    /// Builder: mark this component as cascade-held whenever `dependency` is
    /// held. Only meaningful in combination with `FailurePolicy::Cascading`
    /// on the *dependency*, not on this component.
    pub const fn with_depends_on(mut self, dependency: ComponentId) -> Self {
        self.depends_on = Some(dependency);
        self
    }
}

/// The result of the board's power-on checks, delivered inside [`Event::PowerGood`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerOnResult {
    /// Self-verified and provisioned.
    Provisioned,
    /// Self-verified but not provisioned — cannot act as a RoT.
    Unprovisioned,
    /// Self-verification failed — latches immediately to [`State::Locked`].
    SelfVerificationFailed,
}

/// Everything the outside world can tell the state machine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// Power-on, carrying the shell's self-verification and provisioning result.
    PowerGood(PowerOnResult),
    /// The eRoT's signature + SVN check on this component passed.
    VerificationPassed(ComponentId),
    /// The eRoT's signature + SVN check on this component failed.
    VerificationFailed(ComponentId),
    /// An `Active` component's iRoT has finished local verification and is ready.
    ComponentReady(ComponentId),
    /// A `Passive` component reported boot-progress liveness — its firmware came
    /// up. The passive-tier counterpart to [`Event::ComponentReady`]: a passive
    /// component has no iRoT to self-verify, so "it booted" is the only
    /// post-release signal it can produce. Clears that component's boot-progress
    /// watchdog. Mirrors orchestrator-capabilities's `BootProgress::Booted`.
    Booted(ComponentId),
    /// A challenger has requested a signed attestation.
    AttestationChallenge,
    /// A firmware update has been requested.
    UpdateRequest,
    /// The staged update authenticated successfully.
    UpdateVerified,
    /// The staged update failed authentication.
    UpdateRejected,
    /// The activated image proved itself healthy at runtime (supervised
    /// health / attestation pass — not mere boot). Gates the anti-rollback
    /// commit: only now is it safe to advance the SVN floor past this image,
    /// because it has demonstrated it runs, not merely that it authenticated.
    BootConfirmed(ComponentId),
    /// This component was found corrupt at runtime.
    CorruptionDetected(ComponentId),
    /// This component has been restored from its configured recovery source.
    Restored(ComponentId),
    /// A required component's recovery was exhausted.
    RecoveryFailed,
    /// The shell's boot-progress watchdog fired: `id` did not report its
    /// boot-progress signal ([`Event::ComponentReady`] for an `Active`
    /// component, [`Event::Booted`] for a `Passive` one) within its configured
    /// boot timeout. Treated as a verification failure — a component still
    /// awaiting boot-progress enters recovery. A timeout for a component that is
    /// not awaiting boot (never released, already reported in, or already gated)
    /// is stale/spurious and dropped. The watchdog is per component and
    /// device-agnostic, matching CSA boot-progress checkpointing.
    Timeout(ComponentId),
    /// The driver's *commit* watchdog fired: an update was activated
    /// ([`Effect::ActivateUpdate`]) but did not report [`Event::BootConfirmed`]
    /// within the policy window. Distinct from [`Event::Timeout`], which is the
    /// per-component boot-progress watchdog; this one bounds the single
    /// activated-but-not-committed window in [`State::Ready`]. Handled only while
    /// that window is open: it latches [`State::Locked`] (commit-or-lock — the
    /// anti-rollback floor is never advanced for an unproven image, and the
    /// downgrade window may not stay open indefinitely). Outside the window
    /// (nothing pending) it is stale and dropped. The driver arms this watchdog
    /// when it executes [`Effect::ActivateUpdate`] and cancels it on
    /// [`Effect::CommitSvnFloor`].
    CommitTimeout,
    /// The shell could not carry out an emitted [`Effect`]; fail-closed, it
    /// latches to [`State::Locked`] from any state. Injected by the driver when
    /// a [`Platform::execute`](crate::Platform::execute) call fails; never
    /// produced by a handler.
    EffectFailed,
}

impl Event {
    /// The component this event is about, or `None` for events that name no
    /// component (`PowerGood`, `UpdateRequest`, `CommitTimeout`, …).
    ///
    /// This is the single enumeration of the id-carrying events, consulted once
    /// at the dispatch boundary ([`Orchestrator::step`](crate::Orchestrator::step))
    /// to drop any event that names a component outside the configured chain
    /// before a handler ever sees it. A new id-carrying variant must be listed
    /// here, or it will bypass that membership check.
    pub(crate) fn component_id(&self) -> Option<ComponentId> {
        match self {
            Event::VerificationPassed(id)
            | Event::VerificationFailed(id)
            | Event::ComponentReady(id)
            | Event::Booted(id)
            | Event::BootConfirmed(id)
            | Event::CorruptionDetected(id)
            | Event::Restored(id)
            | Event::Timeout(id) => Some(*id),
            Event::PowerGood(_)
            | Event::AttestationChallenge
            | Event::UpdateRequest
            | Event::UpdateVerified
            | Event::UpdateRejected
            | Event::RecoveryFailed
            | Event::CommitTimeout
            | Event::EffectFailed => None,
        }
    }
}

/// Everything the state machine can ask the outside world to do.
///
/// [`Effect::Emit`] is the sole internal effect: the orchestrator catches it and
/// queues the carried event for immediate handling, making follow-up events
/// visible in the effect trace instead of hidden state changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    ReadFirmware(ComponentId),
    VerifyFirmware(ComponentId),
    ReleaseReset(ComponentId),
    /// Assert reset on a component that is already running — the inverse of
    /// [`ReleaseReset`]. Emitted when an `Isolable`/`Cascading` component is
    /// found corrupt at runtime, or is held after recovery is exhausted: the
    /// component is gated without triggering (or continuing) a recovery cycle.
    AssertReset(ComponentId),
    SignAttestation,
    AuthenticateUpdate,
    StageUpdate,
    ActivateUpdate,
    DiscardStaged,
    /// Advance the anti-rollback (SVN) floor past `id`'s now-confirmed image.
    /// Deliberately decoupled from [`ActivateUpdate`]: activation happens on
    /// authentication (`UpdateVerified`), but the floor may only move once the
    /// image has proven healthy ([`Event::BootConfirmed`]). Committing the
    /// floor earlier would burn anti-rollback on an image that authenticated
    /// but has not yet demonstrated it can boot and run.
    CommitSvnFloor(ComponentId),
    /// Recover `id` from its configured recovery source. The mechanism —
    /// golden image, A/B slot, streamed image, or vendor-specific scheme — is
    /// deferred to the [`Platform`](crate::Platform) driver, which resolves it
    /// per configuration policy. The reducer only names the component to
    /// recover; it does not encode how recovery is performed.
    RecoverComponent(ComponentId),
    /// Report that a component has been isolated (held in reset and removed
    /// from the trust chain) so management software is aware the platform is
    /// running degraded. Emitted once per component, at the moment it is
    /// gated — CSA's degraded-mode clause requires the failure to be reported
    /// through the platform management interface, not just contained.
    ReportIsolated(ComponentId),
    /// Report that a `Required` component exhausted its recovery attempts,
    /// immediately before the machine latches to [`Locked`](crate::State::Locked).
    /// Names the component that forced the halt.
    ReportRecoveryFailed(ComponentId),
    /// Report that an [`Event::UpdateRequest`] was declined because the machine
    /// is busy in a supervised state other than [`Ready`](crate::State::Ready)
    /// — a chain walk is finishing ([`AwaitingReady`](crate::State::AwaitingReady)),
    /// an update is already staged ([`Updating`](crate::State::Updating)), or a
    /// recovery is in flight ([`Recovering`](crate::State::Recovering)). Emitted
    /// instead of silently dropping the request, so the refusal is visible in
    /// the effect trace and the driver can answer the requester (e.g. a PLDM
    /// "retry later" completion code). Advisory only: it changes no state and
    /// does not perturb the in-flight update or recovery. `Ready` never produces
    /// this — it accepts the request and transitions to `Updating`.
    ReportUpdateDeferred,
    /// Report that an update *already in flight* ([`Updating`](crate::State::Updating))
    /// was aborted because a `Required` component was found corrupt and recovery
    /// preempted it (the machine left `Updating` for
    /// [`Recovering`](crate::State::Recovering)). Its counterpart on the intake
    /// side is [`ReportUpdateDeferred`]: `Deferred` refuses an update that never
    /// started, `Aborted` announces the loss of one that had. Paired with the
    /// [`DiscardStaged`] that cleans up the orphaned image — `DiscardStaged`
    /// disposes of the *image*, this answers the *request*, so the requester
    /// (which was awaiting `UpdateVerified`/`UpdateRejected`) learns the update
    /// was superseded by recovery and can retry once the platform is whole.
    ReportUpdateAborted,
    LatchLockdown,
    /// Internal only — tells the orchestrator to handle this event next.
    /// Never forwarded to a [`Platform`](crate::Platform).
    Emit(Event),
}

/// The states the machine can be in. A variant carries exactly the data that is
/// meaningful only while in that state; everything that spans states (the
/// cursor, the gate set, retry counts) lives in [`Rot`](crate::Rot) shared
/// storage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    PowerOnReset,
    PreSupervision,
    /// eRoT has released an `Active` component; the payload is the component
    /// whose iRoT readiness is outstanding (INV9). Only the active-tier readiness
    /// checkpoint lives on this payload; per-component boot-progress liveness
    /// (for both tiers) is tracked in each component's status, not here.
    ///
    /// - `AwaitingReady(Some(id))` — waiting for `id`'s
    ///   [`Event::ComponentReady`].
    /// - `AwaitingReady(None)` — that readiness has been satisfied but the chain
    ///   walk is not finished; the machine stays supervised while draining the
    ///   remaining speculative verifications, and any further `ComponentReady`
    ///   is spurious.
    AwaitingReady(Option<ComponentId>),
    Ready,
    Updating,
    /// A component failed verification (or was found corrupt under a
    /// non-gating policy) and is being restored from its configured recovery
    /// source. The payload is
    /// that component — always present, since the machine only enters this state
    /// with a recovery target in hand.
    Recovering(ComponentId),
    Locked,
}

/// A validated **chain of trust**: the ordered list of components the eRoT
/// walks, verifies, and supervises, in walk order.
///
/// Board integrations should use [`Chain::from_device_configs`] so device
/// count, order, and ids come directly from their `DeviceConfig` array. Tests
/// and lower-level callers can also use [`TryFrom`]/[`TryInto`] with a
/// `heapless::Vec` of `(ComponentId, ComponentAttrs)` pairs. Both paths enforce
/// the reducer's structural invariants at the boundary, so a malformed chain
/// fails closed instead of misbehaving later:
///
/// - the chain is non-empty,
/// - every [`ComponentId`] is unique,
/// - every `depends_on` names a component that exists and appears *strictly
///   earlier* in the chain (no dangling, forward, or self dependencies — a
///   dependency is always walked before its dependents),
/// - the length fits `u8`, the `cursor` index type.
///
/// ```ignore
/// let mut v = heapless::Vec::<_, 4>::new();
/// v.push((ComponentId::new(0), ComponentAttrs::passive_required())).unwrap();
/// let chain: Chain<4> = v.try_into()?;
/// ```
#[derive(Clone, Debug)]
pub struct Chain<const N: usize> {
    entries: heapless::Vec<(ComponentId, ComponentAttrs), N>,
}

/// Why a `heapless::Vec` of components is not a valid [`Chain`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChainError {
    /// The chain has no components.
    Empty,
    /// The chain is longer than `u8::MAX`, so `cursor` could not index it.
    TooLong,
    /// The same [`ComponentId`] appears more than once.
    DuplicateId(ComponentId),
    /// A `depends_on` names an id that is not in the chain.
    UnknownDependency {
        component: ComponentId,
        depends_on: ComponentId,
    },
    /// A `depends_on` names a component that does not appear strictly earlier
    /// in the chain (a forward reference or a self-reference). A dependency
    /// must be walked before its dependents.
    ForwardDependency {
        component: ComponentId,
        depends_on: ComponentId,
    },
}

impl<const N: usize> Chain<N> {
    /// Build the chain directly from the board's device table.
    ///
    /// Device count, order, and ids come from the same table used by the board
    /// drivers. `attrs_for` supplies policy by stable id without introducing a
    /// second ordered list that could drift from the hardware configuration.
    pub fn from_device_configs<R, G>(
        devices: &[DeviceConfig<R, G>; N],
        mut attrs_for: impl FnMut(ComponentId) -> ComponentAttrs,
    ) -> Result<Self, ChainError> {
        let mut entries = heapless::Vec::new();
        for device in devices {
            entries
                .push((device.id, attrs_for(device.id)))
                .expect("device array length matches chain capacity");
        }
        entries.try_into()
    }

    /// Consume the validated chain, yielding its components in walk order.
    pub(crate) fn into_entries(self) -> heapless::Vec<(ComponentId, ComponentAttrs), N> {
        self.entries
    }
}

impl<const N: usize> TryFrom<heapless::Vec<(ComponentId, ComponentAttrs), N>> for Chain<N> {
    type Error = ChainError;

    fn try_from(
        entries: heapless::Vec<(ComponentId, ComponentAttrs), N>,
    ) -> Result<Self, ChainError> {
        if entries.is_empty() {
            return Err(ChainError::Empty);
        }
        if entries.len() > u8::MAX as usize {
            return Err(ChainError::TooLong);
        }
        for (i, (id, _)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(prev, _)| prev == id) {
                return Err(ChainError::DuplicateId(*id));
            }
        }
        for (i, (id, attrs)) in entries.iter().enumerate() {
            if let Some(dep) = attrs.depends_on {
                match entries.iter().position(|(cid, _)| *cid == dep) {
                    None => {
                        return Err(ChainError::UnknownDependency {
                            component: *id,
                            depends_on: dep,
                        });
                    }
                    Some(j) if j >= i => {
                        return Err(ChainError::ForwardDependency {
                            component: *id,
                            depends_on: dep,
                        });
                    }
                    Some(_) => {}
                }
            }
        }
        Ok(Self { entries })
    }
}
