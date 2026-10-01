// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

/// Outcome of [`Rot::gate_by_policy`]: whether a component was gated out of
/// service. Collapses the three [`FailurePolicy`] values into the two
/// control-flow outcomes the caller actually branches on — so the
/// runtime-corruption path and the recovery-exhaustion path decide on the same
/// result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Gating {
    /// The component was gated (`Isolable` → itself; `Cascading` → itself and
    /// its transitive dependents). Its reset is asserted and the walk skips it.
    Gated,
    /// Not a gating policy (`Required`, or an unknown/missing id). Nothing was
    /// gated; the caller handles this in its own context — recover (at runtime)
    /// or lock down (once recovery is exhausted).
    NotGated,
}

/// Per-component service lifecycle. Each chain component carries one of these
/// inside its [`ComponentStatus`], recording whether it is in normal service or
/// gated out. The walk-phase payloads (`AwaitingReady`/`Recovering`) stay on the
/// global [`State`] rather than here — those phases are properties of the whole
/// machine, not of one component. Named for the *component service* axis to keep
/// it distinct from orchestrator-capabilities's trial-boot/commit (update-slot)
/// lifecycle, which
/// is a separate concern.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ComponentLifecycle {
    /// In the normal flow: held, under verification, or released. The global
    /// `State` carries which of those the walk is in.
    Nominal,
    /// Durably gated out of service: has a live `AssertReset` and is skipped on
    /// every chain walk.
    Isolated,
}

/// One per-component record (see [`Rot::statuses`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ComponentStatus {
    pub(crate) lifecycle: ComponentLifecycle,
    /// Consecutive failed-restore count (INV7: consecutive only).
    pub(crate) retry: u8,
    /// Set while this component has been released from reset but has not yet
    /// reported its boot-progress signal ([`Event::ComponentReady`] for an
    /// `Active` component, [`Event::Booted`] for a `Passive` one). The platform driver
    /// arms a per-component watchdog on release; this bit is what a later
    /// [`Event::Timeout`] consults to tell a real boot failure from a stale or
    /// spurious timeout. Orthogonal to `lifecycle`: a gated component owes no
    /// boot-progress signal, so gating clears it.
    pub(crate) awaiting_boot: bool,
    /// Set while this component has been released from reset and not since held
    /// again — i.e. it is *live*, executing code. Set at every `ReleaseReset`,
    /// cleared at every `AssertReset` (gating, recovery, or the pre-walk
    /// quiesce). This is what makes recovery a genuine platform re-boot: on
    /// re-entering [`State::PreSupervision`] the machine asserts reset on every
    /// live component before re-verifying, so `VerifyFirmware` never runs
    /// against code that is still executing (a live check says nothing about
    /// what is running and is open to a post-check flash rewrite). Distinct from
    /// `awaiting_boot`, which is cleared once the component reports in but stays
    /// live: a booted component is `released` yet no longer `awaiting_boot`.
    pub(crate) released: bool,
}

impl Default for ComponentStatus {
    fn default() -> Self {
        Self {
            lifecycle: ComponentLifecycle::Nominal,
            retry: 0,
            awaiting_boot: false,
            released: false,
        }
    }
}
