// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use super::*;
use std::vec::Vec;

const C0: ComponentId = ComponentId::new(0);
const C1: ComponentId = ComponentId::new(1);
const C2: ComponentId = ComponentId::new(2);
const C3: ComponentId = ComponentId::new(3);

const BOOT: Event = Event::PowerGood(PowerOnResult::Provisioned);

const CAPACITY: usize = 8;
const ECAP: usize = 2 * CAPACITY + 2;
const MAX_RETRY: u8 = 3;

fn chain(
    ids: &[(ComponentId, ComponentAttrs)],
) -> heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY> {
    let mut c = heapless::Vec::new();
    for &entry in ids {
        c.push(entry).expect("chain within CAPACITY");
    }
    c
}

fn passive_required(ids: &[ComponentId]) -> heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY> {
    chain(
        &ids.iter()
            .map(|&id| (id, ComponentAttrs::passive_required()))
            .collect::<std::vec::Vec<_>>(),
    )
}

struct Recorder {
    recorded: Vec<Effect>,
}

impl Recorder {
    fn new() -> Self {
        Self {
            recorded: Vec::new(),
        }
    }
}

impl Platform for Recorder {
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
        self.recorded.push(effect);
        Ok(None)
    }
}

fn drive(
    chain: heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY>,
    script: &[Event],
) -> (Vec<Effect>, State) {
    let mut orch =
        Orchestrator::<CAPACITY, ECAP>::new(chain.try_into().expect("valid chain"), MAX_RETRY);
    let mut platform = Recorder::new();
    for &event in script {
        orch.dispatch(&mut platform, event);
    }
    (platform.recorded, orch.state())
}

/// INV1/INV2/INV3: provisioned power-on walks the chain in order; no
/// component is released before its eRoT-side verification passes.
#[test]
fn cold_boot_walks_chain_in_order() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
        ],
    );
    assert_eq!(
        effects,
        std::vec![
            Effect::ReadFirmware(C0),
            Effect::VerifyFirmware(C0),
            Effect::ReleaseReset(C0),
            Effect::ReadFirmware(C1),
            Effect::VerifyFirmware(C1),
            Effect::ReleaseReset(C1),
        ],
    );
    assert_eq!(state, State::Ready);
}

/// Unprovisioned power-on latches immediately.
#[test]
fn unprovisioned_boot_locks_down() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[Event::PowerGood(PowerOnResult::Unprovisioned)],
    );
    assert_eq!(effects, std::vec![Effect::LatchLockdown]);
    assert_eq!(state, State::Locked);
}

/// INV11: SelfVerificationFailed latches immediately without entering
/// PreSupervision.
#[test]
fn self_verification_failure_latches_immediately() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[Event::PowerGood(PowerOnResult::SelfVerificationFailed)],
    );
    assert_eq!(effects, std::vec![Effect::LatchLockdown]);
    assert_eq!(state, State::Locked);
}

/// INV6: AttestationChallenge is answerable from every SupervisingPlatform state.
#[test]
fn attestation_shared_across_supervising_platform_states() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::AttestationChallenge,
        ],
    );
    assert_eq!(effects.last(), Some(&Effect::SignAttestation));
    assert_eq!(state, State::Ready);

    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::AttestationChallenge,
        ],
    );
    assert_eq!(effects.last(), Some(&Effect::SignAttestation));
    assert_eq!(state, State::Updating);
}

/// INV4: a rejected update rolls back via DiscardStaged and never enters
/// Recovering.
#[test]
fn update_rollback_is_not_recovery() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateRejected,
        ],
    );
    let tail = &effects[effects.len() - 3..];
    assert_eq!(
        tail,
        &[
            Effect::AuthenticateUpdate,
            Effect::StageUpdate,
            Effect::DiscardStaged
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// INV5: runtime corruption targets the named component and re-walks from
/// the top after restore.
#[test]
fn runtime_corruption_targets_component_and_rewalks() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C1),
            Event::Restored(C1),
        ],
    );
    let tail = &effects[effects.len() - 2..];
    assert_eq!(
        tail,
        &[Effect::ReadFirmware(C0), Effect::VerifyFirmware(C0)]
    );
    assert_eq!(state, State::PreSupervision);
}

/// Recovery is a full platform re-boot: a live sibling is held in reset before
/// the re-walk re-verifies, so `VerifyFirmware` never runs against executing
/// code. Here C0 and C1 both boot and go live; C0 is then corrupted and
/// restored. The re-walk must `AssertReset(C1)` (quiesce the live sibling)
/// before it re-reads and re-verifies C0 from a fully-held state.
#[test]
fn recovery_rewalk_quiesces_live_siblings_first() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1), // both live → Ready
            Event::CorruptionDetected(C0), // required → Recovering(C0)
            Event::Restored(C0),           // re-walk: quiesce C1, then re-verify C0
        ],
    );
    // The re-walk holds the live sibling before any re-verification.
    let tail = &effects[effects.len() - 3..];
    assert_eq!(
        tail,
        &[
            Effect::AssertReset(C1),
            Effect::ReadFirmware(C0),
            Effect::VerifyFirmware(C0),
        ],
    );
    assert_eq!(state, State::PreSupervision);
}

/// `quiesce_all` holds *every* live component, not just the immediate
/// neighbor: with three live parts, recovering one asserts reset on both
/// siblings before the re-walk re-verifies anything.
#[test]
fn recovery_rewalk_quiesces_all_live_siblings() {
    let (effects, state) = drive(
        passive_required(&[C0, C1, C2]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::VerificationPassed(C2), // all live → Ready
            Event::CorruptionDetected(C0), // required → Recovering(C0)
            Event::Restored(C0),           // re-walk: quiesce C1 and C2 first
        ],
    );
    // Both live siblings are held before the re-walk re-reads the chain.
    let rewalk_read = effects
        .iter()
        .rposition(|e| *e == Effect::ReadFirmware(C0))
        .unwrap();
    let hold_c1 = effects
        .iter()
        .position(|e| *e == Effect::AssertReset(C1))
        .unwrap();
    let hold_c2 = effects
        .iter()
        .position(|e| *e == Effect::AssertReset(C2))
        .unwrap();
    assert!(hold_c1 < rewalk_read);
    assert!(hold_c2 < rewalk_read);
    assert_eq!(state, State::PreSupervision);
}

/// The TOCTOU closure: a live sibling is not trusted across a recovery on
/// its old pass. The re-walk holds it (`AssertReset`), re-verifies it from
/// that held state (`ReadFirmware`/`VerifyFirmware`), and only then releases
/// it again — so the sibling is verified twice and released twice, with the
/// hold in between.
#[test]
fn recovery_rewalk_reverifies_live_sibling_at_rest() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1), // both live → Ready
            Event::CorruptionDetected(C0), // required → Recovering(C0)
            Event::Restored(C0),           // re-walk: quiesce C1
            Event::VerificationPassed(C0), // re-release C0, re-read C1
            Event::VerificationPassed(C1), // re-verify C1 at rest → Ready
        ],
    );
    assert_eq!(state, State::Ready);
    let count = |target: Effect| effects.iter().filter(|e| **e == target).count();
    // C1 is held once (quiesce), and verified + released a second time.
    assert_eq!(count(Effect::AssertReset(C1)), 1);
    assert_eq!(count(Effect::VerifyFirmware(C1)), 2);
    assert_eq!(count(Effect::ReleaseReset(C1)), 2);
    // The re-verification and re-release both come after the hold.
    let hold = effects
        .iter()
        .position(|e| *e == Effect::AssertReset(C1))
        .unwrap();
    let reverify = effects
        .iter()
        .rposition(|e| *e == Effect::VerifyFirmware(C1))
        .unwrap();
    let rerelease = effects
        .iter()
        .rposition(|e| *e == Effect::ReleaseReset(C1))
        .unwrap();
    assert!(hold < reverify);
    assert!(reverify < rerelease);
}

/// `PreSupervision` reacts to `CorruptionDetected` directly (via
/// [`Rot::handle_corruption`]), even though it isn't linked to
/// `SupervisingPlatform` (so `AttestationChallenge` is still discarded
/// there — a separate question). CSA defines no mechanism guaranteeing a
/// corruption report exists for an already-released component's *live,
/// executing* state (at-rest/NVM-polling is scoped to "at
/// rest"/"between boots", not an in-progress boot's chain walk) — but
/// that only means such a report isn't guaranteed to arrive, not that one
/// should be ignored if it does.
#[test]
fn corruption_during_presupervision_selfloop_triggers_recovery() {
    let (effects, state) = drive(
        passive_required(&[C0, C1, C2]),
        &[
            BOOT,
            Event::VerificationPassed(C0), // released; walk continues (still PreSupervision)
            Event::CorruptionDetected(C0), // C0 already released, but caught anyway
        ],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// INV7 (feedback-as-data): after MAX_RETRY restores the core self-emits
/// RecoveryFailed and latches to Locked without any external RecoveryFailed
/// in the script.
#[test]
fn retry_cap_self_latches_via_emit() {
    let mut script = std::vec![BOOT, Event::VerificationPassed(C0)];
    script.push(Event::CorruptionDetected(C0));
    for _ in 0..(MAX_RETRY - 1) {
        script.push(Event::Restored(C0));
        script.push(Event::VerificationFailed(C0));
    }
    script.push(Event::Restored(C0));

    let (effects, state) = drive(passive_required(&[C0]), &script);

    assert!(!script.contains(&Event::RecoveryFailed));
    assert_eq!(state, State::Locked);
    assert_eq!(effects.last(), Some(&Effect::LatchLockdown));
}

/// INV7: retry count resets after a successful recovery so a later episode
/// starts from zero.
#[test]
fn retry_count_resets_after_successful_recovery() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required()))
        .expect("fits");
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), 2);
    let mut effects = Vec::new();

    for ev in [
        BOOT,
        Event::VerificationPassed(C0),
        Event::CorruptionDetected(C0),
        Event::Restored(C0),
        Event::VerificationPassed(C0),
    ] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }
    assert_eq!(orch.state(), State::Ready);

    let start = effects.len();
    for ev in [
        Event::CorruptionDetected(C0),
        Event::Restored(C0),
        Event::VerificationPassed(C0),
    ] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }
    assert_eq!(orch.state(), State::Ready);
    assert!(!effects[start..].contains(&Effect::LatchLockdown));
}

/// Retry budgets are **per component**, not a single global counter. Two
/// required components each fail exactly once (well under `max_retry = 2`)
/// in an interleaved recovery sequence before the chain finally settles.
/// Under a shared global counter the second failure would push the count to
/// the cap and latch the platform to `Locked`; per-component counting lets
/// each device use its own budget, so the walk reaches `Ready`.
#[test]
fn retry_budget_is_per_component() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required()))
        .expect("fits");
    c.push((C1, ComponentAttrs::passive_required()))
        .expect("fits");
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), 2);
    let mut effects = Vec::new();

    for ev in [
        BOOT,
        Event::VerificationFailed(C0), // C0 fails once → Recovering
        Event::Restored(C0),           // C0 count = 1 (< 2) → re-walk
        Event::VerificationPassed(C0), // C0 recovered → its streak clears
        Event::VerificationFailed(C1), // C1 fails once → Recovering
        Event::Restored(C1),           // C1 count = 1 (< 2); global would be 2 → latch
        Event::VerificationPassed(C0), // re-walk restarts at the top
        Event::VerificationPassed(C1), // chain done → Ready
    ] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }

    assert_eq!(orch.state(), State::Ready);
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Board-supplied retry cap: max_retry = 1 latches on the first failed
/// restore.
#[test]
fn custom_retry_cap_latches_sooner() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required()))
        .expect("fits");
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), 1);
    let mut effects = Vec::new();
    for ev in [
        BOOT,
        Event::VerificationPassed(C0),
        Event::CorruptionDetected(C0),
        Event::Restored(C0),
    ] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }
    assert_eq!(orch.state(), State::Locked);
    assert_eq!(effects.last(), Some(&Effect::LatchLockdown));
}

/// Three-component chain uses N=3; walks all three to Ready.
#[test]
fn custom_capacity_walks_full_chain() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), 3>::new();
    for &id in &[C0, C1, C2] {
        c.push((id, ComponentAttrs::passive_required()))
            .expect("3 fits");
    }
    let mut orch = Orchestrator::<3, 8>::new(c.try_into().expect("valid chain"), MAX_RETRY);
    let mut effects = Vec::new();
    for ev in [
        BOOT,
        Event::VerificationPassed(C0),
        Event::VerificationPassed(C1),
        Event::VerificationPassed(C2),
    ] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }
    assert_eq!(orch.state(), State::Ready);
    assert_eq!(effects.last(), Some(&Effect::ReleaseReset(C2)));
}

/// INV10: Active component gates the chain walk — cursor does not advance
/// until ComponentReady arrives.
#[test]
fn active_component_gates_on_component_ready() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[BOOT, Event::VerificationPassed(C0)],
    );
    assert_eq!(state, State::AwaitingReady(Some(C0)));
    assert!(effects.contains(&Effect::ReleaseReset(C0)));
    assert!(effects.contains(&Effect::ReadFirmware(C1)));

    let (effects2, state2) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::ComponentReady(C0),
            Event::VerificationPassed(C1),
        ],
    );
    assert_eq!(state2, State::Ready);
    assert!(effects2.contains(&Effect::ReleaseReset(C1)));
}

/// INV9: a ComponentReady for the wrong id is silently ignored.
#[test]
fn spurious_component_ready_is_ignored() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::ComponentReady(C1), // wrong id
        ],
    );
    assert_eq!(state, State::AwaitingReady(Some(C0)));
    assert!(!effects.contains(&Effect::ReleaseReset(C1)));
}

/// INV12: AttestationChallenge is handled in AwaitingReady.
#[test]
fn attestation_in_awaiting_ready() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::AttestationChallenge,
        ],
    );
    assert_eq!(state, State::AwaitingReady(Some(C0)));
    assert_eq!(effects.last(), Some(&Effect::SignAttestation));
}

/// D2: a boot-progress timeout for the awaited component is treated as a
/// verification failure and enters recovery.
#[test]
fn timeout_awaited_enters_recovering() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[BOOT, Event::VerificationPassed(C0), Event::Timeout(C0)],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// D2: a timeout for a component that is not awaiting boot-progress is
/// stale/spurious and is dropped. Here `C1` has only been *verified*
/// speculatively, not released, so no boot watchdog is armed for it; the
/// machine keeps waiting on the component it actually released (`C0`).
#[test]
fn timeout_stale_id_ignored() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::Timeout(C1), // verified but never released → not awaiting boot
        ],
    );
    assert_eq!(state, State::AwaitingReady(Some(C0)));
    assert!(!effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
}

/// An out-of-chain id in a `VerificationFailed` report is dropped: the core
/// supervises only chain components, so a verdict for an id the chain does not
/// contain neither enters `Recovering` nor emits `RecoverComponent`.
#[test]
fn verification_failed_out_of_chain_id_is_dropped() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[BOOT, Event::VerificationFailed(C3)],
    );
    assert!(!effects.contains(&Effect::RecoverComponent { id: C3, attempt: 0 }));
    // Untouched: still walking the chain from the top with C0 under verification.
    assert_eq!(state, State::PreSupervision);
}

/// An out-of-chain id in a `CorruptionDetected` report is likewise dropped: a
/// malformed report from the platform cannot drive a spurious recovery or move
/// the machine out of `Ready`.
#[test]
fn corruption_out_of_chain_id_is_dropped() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C3),
        ],
    );
    assert!(!effects.contains(&Effect::RecoverComponent { id: C3, attempt: 0 }));
    assert_eq!(state, State::Ready);
}

/// Device-agnostic boot-progress: a *passive* component that is released but
/// never reports [`Event::Booted`] before its watchdog fires is recovered like
/// any other boot failure — even while the walk is still in `PreSupervision`.
/// This closes the release-and-forget gap (CSA boot-progress checkpointing is
/// device-agnostic; the orchestrator arms a `boot_timeout` for every device).
#[test]
fn passive_boot_timeout_enters_recovering() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        // C0 released (watchdog armed), walk speculatively verifies C1, then
        // C0's boot window closes with no `Booted`.
        &[BOOT, Event::VerificationPassed(C0), Event::Timeout(C0)],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::ReleaseReset(C0)));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// A passive boot timeout is caught even after the chain walk has completed and
/// the machine has reached `Ready`: speculative release means a component can
/// still owe a boot-progress signal in `Ready`, and the supervisor runs the
/// same device-agnostic watchdog there.
#[test]
fn passive_boot_timeout_in_ready_enters_recovering() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        // Single passive: reaches `Ready` on VerificationPassed while still
        // awaiting C0's boot-progress; the timeout then fires in `Ready`.
        &[BOOT, Event::VerificationPassed(C0), Event::Timeout(C0)],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// A passive component that reports [`Event::Booted`] retires its watchdog, so a
/// later timeout for it is stale and dropped — the machine stays `Ready`.
#[test]
fn passive_booted_clears_watchdog_then_timeout_is_stale() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::Booted(C0), // C0 reports in → watchdog cleared
            Event::VerificationPassed(C1),
            Event::Booted(C1),
            Event::Timeout(C0), // stale: C0 already booted
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(!effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// D2: full path — timeout drives recovery, restore rewalks from the top, and
/// the chain then completes normally.
#[test]
fn timeout_recovers_then_rewalks_to_ready() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::Timeout(C0),  // → Recovering(C0)
            Event::Restored(C0), // → PreSupervision, rewalk from top
            Event::VerificationPassed(C0),
            Event::ComponentReady(C0),
            Event::VerificationPassed(C1),
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
    assert!(effects.contains(&Effect::ReleaseReset(C1)));
}

/// Isolable component: every `VerificationFailed` is retried through a full
/// recovery episode first; only once retries are exhausted does the
/// component get held in reset and the walk continue to `Ready`.
#[test]
fn isolable_component_exhausts_recovery_then_skips() {
    let mut script = std::vec![BOOT, Event::VerificationPassed(C0)];
    for _ in 0..MAX_RETRY {
        script.push(Event::VerificationFailed(C1));
        script.push(Event::Restored(C1));
        script.push(Event::VerificationPassed(C0));
    }
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &script,
    );
    assert_eq!(state, State::Ready);
    // C1 must never be released.
    assert!(!effects.contains(&Effect::ReleaseReset(C1)));
    // Recovery IS attempted before C1 is classified and held.
    assert!(effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(effects.contains(&Effect::AssertReset(C1)));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Isolable Active component failure in AwaitingReady: retried through a
/// full recovery episode, then held once exhausted; the walk still
/// reaches Ready once the remaining chain (past the held component) drains.
#[test]
fn isolable_active_component_exhausted_in_awaiting_ready_skips() {
    // C0 Active required, C1 Active isolable.
    let mut script = std::vec![BOOT, Event::VerificationPassed(C0)]; // → AwaitingReady; spec ReadFirmware(C1)
    for _ in 0..MAX_RETRY {
        script.push(Event::VerificationFailed(C1));
        script.push(Event::Restored(C1));
        script.push(Event::VerificationPassed(C0)); // re-walk restarts at C0 each episode
    }
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::active_isolable()),
        ]),
        &script,
    );
    assert_eq!(state, State::Ready);
    assert!(!effects.contains(&Effect::ReleaseReset(C1)));
    assert!(effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(effects.contains(&Effect::AssertReset(C1)));
}

/// Runtime corruption of an `Isolable` component gates the component
/// (AssertReset) but does not trigger recovery — the machine stays in Ready.
#[test]
fn isolable_runtime_corruption_is_ignored() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C1), // Isolable → gate, no recovery
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::AssertReset(C1)));
    assert!(!effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Runtime corruption of an `Isolable` component holds it in reset: it is
/// added to `held`, so a later re-walk triggered by a *required*
/// component's recovery skips it instead of re-releasing a component we
/// already found corrupt.
#[test]
fn isolable_runtime_corruption_holds_across_rewalk() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C1), // isolable → gate + hold
            Event::CorruptionDetected(C0), // required → Recovering
            Event::Restored(C0),           // re-walk from top
            Event::VerificationPassed(C0), // C1 stays held → chain done
        ],
    );
    assert_eq!(state, State::Ready);
    // C1 is released exactly once (the initial walk); the post-corruption
    // re-walk must not release it again.
    assert_eq!(
        effects
            .iter()
            .filter(|e| **e == Effect::ReleaseReset(C1))
            .count(),
        1,
    );
    // C1 is held exactly once (the gate). `quiesce_all` skips it on the
    // recovery re-walk because it is already held — no duplicate reset.
    assert_eq!(
        effects
            .iter()
            .filter(|e| **e == Effect::AssertReset(C1))
            .count(),
        1,
    );
}

/// A durable gate must survive a return to `Ready`. The gate set is *not*
/// cleared on `Ready` entry, so a component isolated by policy stays gated
/// across an intervening return to `Ready` and a later chain walk never
/// re-reads or re-releases it. Extends the single-walk
/// `..._holds_across_rewalk` case with a *second* return to `Ready` — the
/// exact point where the old `held.clear()` dropped the gate.
#[test]
fn gate_survives_return_to_ready() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1), // walk 1 done → Ready
            Event::CorruptionDetected(C1), // isolable → gate + hold C1
            Event::CorruptionDetected(C0), // required → Recovering
            Event::Restored(C0),           // walk 2 from top
            Event::VerificationPassed(C0), // C1 gated skip → Ready (gate must persist)
            Event::CorruptionDetected(C0), // required → Recovering again
            Event::Restored(C0),           // walk 3 from top
            Event::VerificationPassed(C0), // C1 still gated → chain done → Ready
        ],
    );
    assert_eq!(state, State::Ready);
    // C1 was read/verified and released exactly once, on walk 1. If the
    // gate were dropped at `Ready`, walk 3 would re-read and re-release it.
    assert_eq!(
        effects
            .iter()
            .filter(|e| **e == Effect::ReadFirmware(C1))
            .count(),
        1,
    );
    assert_eq!(
        effects
            .iter()
            .filter(|e| **e == Effect::ReleaseReset(C1))
            .count(),
        1,
    );
}

/// Runtime corruption of a `Required` component still triggers
/// recovery as before.
#[test]
fn required_runtime_corruption_triggers_recovery() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C0), // required → Recovering
        ],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// Runtime corruption of a `Cascading` component must gate the whole
/// cascade, not just the reported component. The runtime-corruption path
/// and the recovery-exhaustion path share one `gate_by_policy` source of
/// truth, so `Cascading` cascades in both. C2 `depends_on` C1; corrupting
/// C1 must assert reset on C1 **and** C2.
#[test]
fn cascading_runtime_corruption_cascades() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_cascading()),
            (C2, ComponentAttrs::passive_required().with_depends_on(C1)),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::VerificationPassed(C2), // walk done → Ready
            Event::CorruptionDetected(C1), // Cascading → gate C1 and its dependents
        ],
    );
    assert_eq!(state, State::Ready);
    // The whole cascade is gated: C1 (the root) and C2 (its dependent).
    assert!(effects.contains(&Effect::AssertReset(C1)));
    assert!(effects.contains(&Effect::AssertReset(C2)));
    // No recovery is started for a non-required corruption.
    assert!(!effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Degraded mode (CSA): isolating a component is only half the requirement —
/// the failure must also be *reported* through the platform management
/// interface. An `Isolable` component that exhausts recovery emits
/// `ReportIsolated` alongside its `AssertReset`, and the platform keeps running.
#[test]
fn isolable_exhaustion_reports_isolation() {
    let mut script = std::vec![BOOT, Event::VerificationPassed(C0)];
    for _ in 0..MAX_RETRY {
        script.push(Event::VerificationFailed(C1));
        script.push(Event::Restored(C1));
        script.push(Event::VerificationPassed(C0));
    }
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &script,
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::AssertReset(C1)));
    assert!(effects.contains(&Effect::ReportIsolated(C1)));
    // Degraded, not halted: this is an isolation report, not a halt report.
    assert!(!effects.contains(&Effect::ReportRecoveryFailed(C1)));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Every component taken out of service by a cascade is reported, not just the
/// one that failed: operators need to know that C2 is down too, even though
/// nothing was wrong with C2 itself.
#[test]
fn cascading_exhaustion_reports_each_isolated() {
    let mut script = std::vec![BOOT, Event::VerificationPassed(C0)];
    for _ in 0..MAX_RETRY {
        script.push(Event::VerificationFailed(C1));
        script.push(Event::Restored(C1));
        script.push(Event::VerificationPassed(C0));
    }
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_cascading()),
            (C2, ComponentAttrs::passive_required().with_depends_on(C1)),
        ]),
        &script,
    );
    assert_eq!(state, State::Ready);
    // The root and its transitive dependent are both isolated and both reported.
    assert!(effects.contains(&Effect::AssertReset(C1)));
    assert!(effects.contains(&Effect::AssertReset(C2)));
    assert!(effects.contains(&Effect::ReportIsolated(C1)));
    assert!(effects.contains(&Effect::ReportIsolated(C2)));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Cascades propagate transitively, not just one hop: a `Cascading` failure at
/// the root pulls down its direct dependent AND that dependent's dependent.
/// This exercises the BFS re-enqueue in `cascade_hold` past a single iteration
/// — propagation follows `depends_on` regardless of the dependent's own policy
/// (C2/C3 are `Required` yet are still isolated because they hang off C1).
#[test]
fn cascading_runtime_corruption_cascades_transitively() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_cascading()),
            (C2, ComponentAttrs::passive_required().with_depends_on(C1)),
            (C3, ComponentAttrs::passive_required().with_depends_on(C2)),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::VerificationPassed(C2),
            Event::VerificationPassed(C3), // walk done → Ready
            Event::CorruptionDetected(C1), // Cascading root
        ],
    );
    assert_eq!(state, State::Ready);
    // Root, its dependent, and the transitive dependent are all isolated and
    // all reported — the two-hop chain C1 → C2 → C3 is fully gated.
    assert!(effects.contains(&Effect::AssertReset(C1)));
    assert!(effects.contains(&Effect::AssertReset(C2)));
    assert!(effects.contains(&Effect::AssertReset(C3)));
    assert!(effects.contains(&Effect::ReportIsolated(C1)));
    assert!(effects.contains(&Effect::ReportIsolated(C2)));
    assert!(effects.contains(&Effect::ReportIsolated(C3)));
    // A non-required cascade never enters recovery or lockdown.
    assert!(!effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// A cascade that fires mid-walk gates components the cursor has not reached
/// yet, and the walk then skips them: corruption of C1 arrives while the walk
/// is still verifying C0, so C1 -> C2 -> C3 are all gated before their turn.
/// No `ReadFirmware` is ever emitted for them, and the walk reaches the end of
/// the chain and goes to `Ready`. The post-walk cascade tests cannot cover this:
/// there the cursor is already past every component when the cascade fires.
#[test]
fn mid_walk_cascade_skips_unreached_dependents() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_cascading()),
            (C2, ComponentAttrs::passive_required().with_depends_on(C1)),
            (C3, ComponentAttrs::passive_required().with_depends_on(C2)),
        ]),
        &[
            BOOT,                          // walk starts at C0
            Event::CorruptionDetected(C1), // cascade C1 -> C2 -> C3, cursor still on C0
            Event::VerificationPassed(C0), // advance: nothing ungated is left
        ],
    );
    // The rest of the chain is gated, so the walk is done.
    assert_eq!(state, State::Ready);
    // The whole two-hop chain is isolated and reported, though the walk never
    // reached any of it.
    for id in [C1, C2, C3] {
        assert!(effects.contains(&Effect::AssertReset(id)));
        assert!(effects.contains(&Effect::ReportIsolated(id)));
    }
    // Verification is never requested for a gated component, at any depth.
    for id in [C1, C2, C3] {
        assert!(!effects.contains(&Effect::ReadFirmware(id)));
        assert!(!effects.contains(&Effect::VerifyFirmware(id)));
        assert!(!effects.contains(&Effect::ReleaseReset(id)));
    }
    // C0 is untouched by the cascade: it verifies and is released as usual.
    assert!(effects.contains(&Effect::ReleaseReset(C0)));
    assert!(!effects.contains(&Effect::AssertReset(C0)));
}

/// Runtime corruption under a non-`Required` policy reports too. This path
/// never enters recovery at all, so without its own report the isolation would
/// be silent.
#[test]
fn runtime_corruption_isolable_reports() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C1),
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::ReportIsolated(C1)));
    assert!(!effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
}

/// A `Required` component whose recovery is exhausted is named in a
/// `ReportRecoveryFailed` *before* the platform latches, so management software
/// learns which component forced the halt rather than just that one did.
#[test]
fn required_exhaustion_reports_before_lockdown() {
    let mut script = std::vec![BOOT, Event::VerificationPassed(C0)];
    script.push(Event::CorruptionDetected(C0));
    for _ in 0..(MAX_RETRY - 1) {
        script.push(Event::Restored(C0));
        script.push(Event::VerificationFailed(C0));
    }
    script.push(Event::Restored(C0));

    let (effects, state) = drive(passive_required(&[C0]), &script);

    assert_eq!(state, State::Locked);
    let report = effects
        .iter()
        .position(|e| *e == Effect::ReportRecoveryFailed(C0))
        .expect("the component that forced the halt is reported");
    let latch = effects
        .iter()
        .position(|e| *e == Effect::LatchLockdown)
        .expect("the platform latches");
    assert!(
        report < latch,
        "the report must be actuated before the latch",
    );
}

/// A component that recovers within its retry budget is not degraded, so
/// nothing is reported — reports mark components taken *out of service*, not
/// every transient failure.
#[test]
fn successful_recovery_emits_no_isolation_report() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationFailed(C0),
            Event::Restored(C0),
            Event::VerificationPassed(C0),
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
    assert!(
        !effects.iter().any(|e| matches!(
            e,
            Effect::ReportIsolated(_) | Effect::ReportRecoveryFailed(_)
        )),
        "a recovered component is not degraded",
    );
}

/// Reporting is guarded by `is_gated` exactly like the reset it accompanies, so
/// a repeated corruption report on an already-isolated component does not
/// re-report it. Management software sees one isolation event per component.
#[test]
fn report_isolated_emitted_once_per_component() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::CorruptionDetected(C1), // isolable → gate + report
            Event::CorruptionDetected(C1), // already gated → nothing
        ],
    );
    assert_eq!(state, State::Ready);
    assert_eq!(
        effects
            .iter()
            .filter(|e| **e == Effect::ReportIsolated(C1))
            .count(),
        1,
    );
    assert_eq!(
        effects
            .iter()
            .filter(|e| **e == Effect::AssertReset(C1))
            .count(),
        1,
    );
}

/// Boot-time VerificationFailed on a required component → Recovering.
/// (Distinct from CorruptionDetected: this is a failed eRoT-side check
/// before the component is ever released from reset.)
#[test]
fn boot_failure_required_enters_recovering() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[BOOT, Event::VerificationFailed(C0)],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
    // Component must never be released when its eRoT check failed.
    assert!(!effects.contains(&Effect::ReleaseReset(C0)));
}

/// Full boot-failure recovery cycle: VerificationFailed → Recovering →
/// Restored → re-walk from top → VerificationPassed → Ready.
#[test]
fn boot_failure_recovery_cycle_completes() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationFailed(C0),
            Event::Restored(C0),
            Event::VerificationPassed(C0),
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
    // ReleaseReset only after the recovery re-walk passes.
    assert!(effects.contains(&Effect::ReleaseReset(C0)));
}

/// VerificationFailed (required) on a speculative check while the machine
/// is in AwaitingReady → enters Recovering without releasing the component.
#[test]
fn required_failure_in_awaiting_ready_enters_recovering() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0), // → AwaitingReady; spec check of C1 starts
            Event::VerificationFailed(C1), // required → Recovering
        ],
    );
    assert_eq!(state, State::Recovering(C1));
    assert!(effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(!effects.contains(&Effect::ReleaseReset(C1)));
}

/// CorruptionDetected while in AwaitingReady (required component) →
/// Recovering via the SupervisingPlatform superstate handler.
#[test]
fn corruption_in_awaiting_ready_triggers_recovery() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0), // → AwaitingReady
            Event::CorruptionDetected(C0),
        ],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// CorruptionDetected while in Updating (required component) → Recovering
/// via the SupervisingPlatform superstate handler.
#[test]
fn corruption_in_updating_triggers_recovery() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::CorruptionDetected(C0),
        ],
    );
    assert_eq!(state, State::Recovering(C0));
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// Concurrent faults: corruption of a *different* component arriving while the
/// machine is already in `Recovering` re-targets recovery at the new component.
/// `Recovering` is a supervised state, so `CorruptionDetected` falls through to
/// the `SupervisingPlatform` handler even mid-recovery — the one supervising
/// state with no other corruption-during-flight test. The second (Required)
/// fault displaces the first: the machine ends in `Recovering(C2)`.
#[test]
fn corruption_while_recovering_retargets_to_new_component() {
    let (effects, state) = drive(
        passive_required(&[C0, C1, C2]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1),
            Event::VerificationPassed(C2), // walk done → Ready
            Event::CorruptionDetected(C1), // → Recovering(C1)
            Event::CorruptionDetected(C2), // arrives mid-recovery → Recovering(C2)
        ],
    );
    assert_eq!(state, State::Recovering(C2));
    // Both recovery episodes kicked off a recovery.
    assert!(effects.contains(&Effect::RecoverComponent { id: C1, attempt: 0 }));
    assert!(effects.contains(&Effect::RecoverComponent { id: C2, attempt: 0 }));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// An `UpdateRequest` arriving mid-recovery is declined, not silently dropped:
/// the machine emits `ReportUpdateDeferred` and stays in `Recovering`, leaving
/// the in-flight recovery untouched (single-flight, recovery-priority).
#[test]
fn update_request_while_recovering_is_deferred() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationFailed(C0), // → Recovering(C0)
            Event::UpdateRequest,          // declined while recovering
        ],
    );
    assert_eq!(state, State::Recovering(C0));
    assert_eq!(effects.last(), Some(&Effect::ReportUpdateDeferred));
}

/// A second `UpdateRequest` while an update is already staged (`Updating`) is
/// declined the same way — reported, not dropped, and the staged update is
/// left in place.
#[test]
fn update_request_while_updating_is_deferred() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest, // → Updating
            Event::UpdateRequest, // declined while updating
        ],
    );
    assert_eq!(state, State::Updating);
    assert_eq!(effects.last(), Some(&Effect::ReportUpdateDeferred));
}

/// An `UpdateRequest` while the chain walk is still finishing (`AwaitingReady`)
/// is declined too: the machine is not yet `Ready`, so it reports the refusal
/// and keeps waiting on the outstanding component.
#[test]
fn update_request_while_awaiting_ready_is_deferred() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0), // active released → AwaitingReady(Some(C0))
            Event::UpdateRequest,          // declined while awaiting readiness
        ],
    );
    assert_eq!(state, State::AwaitingReady(Some(C0)));
    assert_eq!(effects.last(), Some(&Effect::ReportUpdateDeferred));
}

/// Negative control: from `Ready` an `UpdateRequest` is *accepted* — it starts
/// an update (→ `Updating`) and never emits `ReportUpdateDeferred`. `Ready`
/// intercepts the request upstream, so it can never reach the deferral arm.
#[test]
fn update_request_in_ready_starts_update_not_deferred() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[BOOT, Event::VerificationPassed(C0), Event::UpdateRequest],
    );
    assert_eq!(state, State::Updating);
    assert!(!effects.contains(&Effect::ReportUpdateDeferred));
}

/// UpdateVerified activates the staged image and returns to Ready.
/// (Complements update_rollback_is_not_recovery which tests UpdateRejected.)
#[test]
fn update_verified_activates_update() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateVerified,
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::ActivateUpdate));
    assert!(!effects.contains(&Effect::DiscardStaged));
    assert!(!effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
}

/// The anti-rollback floor is committed only on a proven-healthy boot, never
/// at activation. `UpdateVerified` activates the image (authentication) but
/// must NOT emit `CommitSvnFloor`; a later `BootConfirmed` (the runtime health
/// proof) is what advances the floor. This pins the decoupling so activation
/// can never silently commit the floor early.
#[test]
fn svn_floor_commits_on_boot_confirmed_not_on_activation() {
    // Activation alone: no floor commit yet.
    let (activated, activated_state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateVerified,
        ],
    );
    assert_eq!(activated_state, State::Ready);
    assert!(activated.contains(&Effect::ActivateUpdate));
    assert!(!activated.contains(&Effect::CommitSvnFloor(C0)));

    // Proven-healthy boot: the floor advances now, without leaving Ready.
    let (confirmed, confirmed_state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateVerified,
            Event::BootConfirmed(C0),
        ],
    );
    assert_eq!(confirmed_state, State::Ready);
    assert!(confirmed.contains(&Effect::CommitSvnFloor(C0)));
}

/// Commit-or-lock watchdog: while the activated-but-not-committed window is
/// open (update activated, `BootConfirmed` not yet seen), a `CommitTimeout`
/// fails closed — the machine latches `Locked` rather than leaving the
/// downgrade window open indefinitely, and never commits the unproven image.
#[test]
fn commit_timeout_while_pending_latches_locked() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateVerified,
            // Window open: activated, awaiting BootConfirmed. Watchdog fires.
            Event::CommitTimeout,
        ],
    );
    assert_eq!(state, State::Locked);
    assert!(effects.contains(&Effect::ActivateUpdate));
    assert!(effects.contains(&Effect::LatchLockdown));
    // The floor was never advanced for the unproven image.
    assert!(!effects.contains(&Effect::CommitSvnFloor(C0)));
}

/// Once `BootConfirmed` has committed the floor the window is closed, so a
/// later (stale) `CommitTimeout` is a no-op: the machine stays `Ready` and does
/// not lock.
#[test]
fn commit_timeout_after_confirm_is_stale_noop() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateVerified,
            Event::BootConfirmed(C0),
            // Window already closed by the commit above.
            Event::CommitTimeout,
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(effects.contains(&Effect::CommitSvnFloor(C0)));
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// A `CommitTimeout` in steady-state `Ready` with no update in flight (no
/// window open) is stale and ignored — a spurious watchdog fire must not lock a
/// healthy device.
#[test]
fn commit_timeout_without_pending_is_ignored() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[BOOT, Event::VerificationPassed(C0), Event::CommitTimeout],
    );
    assert_eq!(state, State::Ready);
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// A recovery that intervenes during the commit window voids it: after the
/// machine recovers and walks back to `Ready`, the window is closed, so a
/// `CommitTimeout` no longer locks. This pins that entering `Recovering` clears
/// `pending_commit`, so the flag cannot go stale across a recovery round-trip.
#[test]
fn recovery_clears_commit_window() {
    let (effects, state) = drive(
        passive_required(&[C0]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::UpdateRequest,
            Event::UpdateVerified,
            // Window open, then a Required corruption preempts to recovery.
            Event::CorruptionDetected(C0),
            // Restore succeeds (retry < MAX_RETRY) and re-walk re-verifies.
            Event::Restored(C0),
            Event::VerificationPassed(C0),
            // Back in Ready with the window voided; the watchdog is now stale.
            Event::CommitTimeout,
        ],
    );
    assert_eq!(state, State::Ready);
    assert!(!effects.contains(&Effect::LatchLockdown));
}

/// Locked is a terminal state: no effects are produced in response to any
/// event after the machine latches.
#[test]
fn locked_is_terminal() {
    let mut c: heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY> = heapless::Vec::new();
    c.push((C0, ComponentAttrs::passive_required())).unwrap();
    // max_retry = 1 so the first failed restore latches immediately.
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), 1);
    let mut effects: Vec<Effect> = Vec::new();

    for ev in [BOOT, Event::VerificationFailed(C0), Event::Restored(C0)] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }
    assert_eq!(orch.state(), State::Locked);

    let count_before = effects.len();
    for ev in [
        BOOT,
        Event::VerificationPassed(C0),
        Event::AttestationChallenge,
        Event::UpdateRequest,
        Event::CorruptionDetected(C0),
    ] {
        orch.dispatch_with(ev, |e| {
            effects.push(e);
            Ok(None)
        });
    }
    assert_eq!(
        effects.len(),
        count_before,
        "Locked state must produce no effects"
    );
}

/// An Isolable component at the head of the chain exhausts its recovery
/// retries, gets held, and the walk continues to the remaining required
/// components.
#[test]
fn isolable_first_component_exhausts_then_walk_continues() {
    let mut script = std::vec![BOOT];
    for _ in 0..MAX_RETRY {
        script.push(Event::VerificationFailed(C0));
        script.push(Event::Restored(C0));
    }
    script.push(Event::VerificationPassed(C1));
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_isolable()),
            (C1, ComponentAttrs::passive_required()),
        ]),
        &script,
    );
    assert_eq!(state, State::Ready);
    assert!(!effects.contains(&Effect::ReleaseReset(C0)));
    assert!(effects.contains(&Effect::ReleaseReset(C1)));
    // Recovery IS attempted before C0 is classified and held.
    assert!(effects.contains(&Effect::RecoverComponent { id: C0, attempt: 0 }));
    assert!(effects.contains(&Effect::AssertReset(C0)));
}

/// The speculative read emits ReleaseReset · ReadFirmware · VerifyFirmware
/// all in the same handler as VerificationPassed for an Active component,
/// before ComponentReady has arrived. Verifies both presence and order.
#[test]
fn speculative_read_effects_are_emitted_together() {
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(
        chain(&[
            (C0, ComponentAttrs::active_required()),
            (C1, ComponentAttrs::passive_required()),
        ])
        .try_into()
        .expect("valid chain"),
        MAX_RETRY,
    );
    let mut effects: Vec<Effect> = Vec::new();

    orch.dispatch_with(BOOT, |e| {
        effects.push(e);
        Ok(None)
    });
    assert_eq!(
        effects,
        std::vec![Effect::ReadFirmware(C0), Effect::VerifyFirmware(C0)],
    );

    effects.clear();
    orch.dispatch_with(Event::VerificationPassed(C0), |e| {
        effects.push(e);
        Ok(None)
    });
    // All three effects emitted in the same handler, before ComponentReady.
    assert_eq!(
        effects,
        std::vec![
            Effect::ReleaseReset(C0),
            Effect::ReadFirmware(C1),
            Effect::VerifyFirmware(C1),
        ],
    );
    assert_eq!(orch.state(), State::AwaitingReady(Some(C0)));
}

/// A chain with a single Active component goes directly to Ready on
/// VerificationPassed — no AwaitingReady, no ComponentReady required.
/// This exercises the `chain done` branch of PreSupervision for an
/// Active component (distinct from the multi-component Active path which
/// transitions to AwaitingReady).
#[test]
fn single_active_chain_goes_directly_to_ready() {
    let mut c: heapless::Vec<(ComponentId, ComponentAttrs), CAPACITY> = heapless::Vec::new();
    c.push((C0, ComponentAttrs::active_required())).unwrap();
    let (effects, state) = drive(c, &[BOOT, Event::VerificationPassed(C0)]);
    assert_eq!(state, State::Ready);
    assert_eq!(
        effects,
        std::vec![
            Effect::ReadFirmware(C0),
            Effect::VerifyFirmware(C0),
            Effect::ReleaseReset(C0),
        ],
    );
}

/// An empty component list is not a valid chain of trust.
#[test]
fn chain_rejects_empty() {
    let empty = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    assert_eq!(Chain::try_from(empty).unwrap_err(), ChainError::Empty);
}

/// A repeated `ComponentId` is rejected: the state machine's linear id lookups would
/// otherwise be ambiguous.
#[test]
fn chain_rejects_duplicate_id() {
    let v = chain(&[
        (C0, ComponentAttrs::passive_required()),
        (C0, ComponentAttrs::passive_required()),
    ]);
    assert_eq!(Chain::try_from(v).unwrap_err(), ChainError::DuplicateId(C0),);
}

/// A `depends_on` that names a component not in the chain is rejected.
#[test]
fn chain_rejects_unknown_dependency() {
    let v = chain(&[(C1, ComponentAttrs::passive_required().with_depends_on(C0))]);
    assert_eq!(
        Chain::try_from(v).unwrap_err(),
        ChainError::UnknownDependency {
            component: C1,
            depends_on: C0,
        },
    );
}

/// A dependency must appear strictly earlier in the walk than its dependent;
/// a forward reference is rejected.
#[test]
fn chain_rejects_forward_dependency() {
    let v = chain(&[
        (C0, ComponentAttrs::passive_required().with_depends_on(C1)),
        (C1, ComponentAttrs::passive_cascading()),
    ]);
    assert_eq!(
        Chain::try_from(v).unwrap_err(),
        ChainError::ForwardDependency {
            component: C0,
            depends_on: C1,
        },
    );
}

/// A component may not depend on itself.
#[test]
fn chain_rejects_self_dependency() {
    let v = chain(&[(C0, ComponentAttrs::passive_required().with_depends_on(C0))]);
    assert_eq!(
        Chain::try_from(v).unwrap_err(),
        ChainError::ForwardDependency {
            component: C0,
            depends_on: C0,
        },
    );
}

/// A well-formed chain with a backward dependency validates successfully.
#[test]
fn chain_accepts_valid_dependency() {
    let v = chain(&[
        (C0, ComponentAttrs::passive_cascading()),
        (C1, ComponentAttrs::passive_required().with_depends_on(C0)),
    ]);
    assert!(Chain::try_from(v).is_ok());
}

/// A [`Platform`] that records every effect and fails a chosen one, to exercise
/// the effect failure channel.
struct FailOn {
    trigger: Effect,
    recorded: Vec<Effect>,
    failed: bool,
}

impl FailOn {
    fn new(trigger: Effect) -> Self {
        Self {
            trigger,
            recorded: Vec::new(),
            failed: false,
        }
    }
}

impl Platform for FailOn {
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
        self.recorded.push(effect);
        if effect == self.trigger {
            self.failed = true;
            Err(EffectError)
        } else {
            Ok(None)
        }
    }
}

/// A failed reset actuation is fail-closed: the orchestrator injects `EffectFailed`
/// and the machine latches to `Locked`, emitting `LatchLockdown`.
#[test]
fn effect_failure_latches_lockdown() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required())).unwrap();
    let mut orch =
        Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), MAX_RETRY);
    let mut plat = FailOn::new(Effect::ReleaseReset(C0));

    orch.dispatch(&mut plat, BOOT); // ReadFirmware/VerifyFirmware C0 — both succeed
    orch.dispatch(&mut plat, Event::VerificationPassed(C0)); // ReleaseReset(C0) fails

    assert!(plat.failed, "the trigger effect should have been attempted");
    assert_eq!(orch.state(), State::Locked);
    assert!(plat.recorded.contains(&Effect::LatchLockdown));
}

/// A failed isolation actuation (`AssertReset`) is equally fail-closed: even a
/// non-required component's containment failing latches the platform.
#[test]
fn failed_isolation_actuation_latches_lockdown() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required())).unwrap();
    c.push((C1, ComponentAttrs::passive_isolable())).unwrap();
    let mut orch =
        Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), MAX_RETRY);
    let mut plat = FailOn::new(Effect::AssertReset(C1));

    orch.dispatch(&mut plat, BOOT);
    orch.dispatch(&mut plat, Event::VerificationPassed(C0));
    orch.dispatch(&mut plat, Event::VerificationPassed(C1)); // C1 released → Ready
    orch.dispatch(&mut plat, Event::CorruptionDetected(C1)); // isolable → AssertReset(C1) fails

    assert!(plat.failed);
    assert_eq!(orch.state(), State::Locked);
    assert!(plat.recorded.contains(&Effect::LatchLockdown));
}

/// A failed recovery actuation is fail-closed too: if the platform driver cannot even
/// recover a required component, the platform latches rather
/// than continuing with an unrecovered component.
#[test]
fn failed_restore_actuation_latches_lockdown() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required())).unwrap();
    let mut orch =
        Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), MAX_RETRY);
    let mut plat = FailOn::new(Effect::RecoverComponent { id: C0, attempt: 0 });

    orch.dispatch(&mut plat, BOOT);
    orch.dispatch(&mut plat, Event::VerificationFailed(C0)); // → Recovering → RecoverComponent(C0) fails

    assert!(plat.failed);
    assert_eq!(orch.state(), State::Locked);
    assert!(plat.recorded.contains(&Effect::LatchLockdown));
}

/// The lockdown latch is the last line of defense: even if *it* fails to
/// actuate, the machine must not spin. The re-injected `EffectFailed` is
/// ignored while `Locked`, so dispatch terminates and the latch is attempted
/// exactly once.
#[test]
fn failed_lockdown_actuation_does_not_loop() {
    let mut c = heapless::Vec::<(ComponentId, ComponentAttrs), CAPACITY>::new();
    c.push((C0, ComponentAttrs::passive_required())).unwrap();
    let mut orch =
        Orchestrator::<CAPACITY, ECAP>::new(c.try_into().expect("valid chain"), MAX_RETRY);
    let mut plat = FailOn::new(Effect::LatchLockdown);

    // An unprovisioned power-on latches immediately; the latch actuation fails.
    orch.dispatch(&mut plat, Event::PowerGood(PowerOnResult::Unprovisioned));

    assert!(plat.failed, "the lockdown latch should have been attempted");
    assert_eq!(orch.state(), State::Locked);
    assert_eq!(
        plat.recorded
            .iter()
            .filter(|&&e| e == Effect::LatchLockdown)
            .count(),
        1,
        "a failing latch must not re-latch forever",
    );
}

/// Actuation is fail-fast: once an effect in a batch fails, no effect ordered
/// *after* it is attempted. Here `VerificationPassed(C0)` emits the batch
/// `[ReleaseReset(C0), ReadFirmware(C1), VerifyFirmware(C1)]`; failing the first
/// effect must abandon the two speculative reads of `C1` and latch, rather than
/// actuate them for a transition that is immediately overridden by `Locked`.
#[test]
fn batch_actuation_is_fail_fast() {
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(
        passive_required(&[C0, C1]).try_into().expect("valid chain"),
        MAX_RETRY,
    );
    let mut plat = FailOn::new(Effect::ReleaseReset(C0));

    orch.dispatch(&mut plat, BOOT); // ReadFirmware/VerifyFirmware C0 — both succeed
    orch.dispatch(&mut plat, Event::VerificationPassed(C0)); // ReleaseReset(C0) fails first

    assert!(plat.failed, "the failing effect should have been attempted");
    assert!(
        plat.recorded.contains(&Effect::ReleaseReset(C0)),
        "the failing effect itself is attempted",
    );
    assert!(
        !plat.recorded.contains(&Effect::ReadFirmware(C1)),
        "an effect ordered after the failure must not be actuated",
    );
    assert!(
        !plat.recorded.contains(&Effect::VerifyFirmware(C1)),
        "an effect ordered after the failure must not be actuated",
    );
    assert_eq!(orch.state(), State::Locked);
    assert!(plat.recorded.contains(&Effect::LatchLockdown));
}

/// GAP 1 (red): a `Required` corruption of another device while an update is in
/// flight preempts the update but leaves the staged image dangling. Correct
/// behavior emits `DiscardStaged` when leaving `Updating` for recovery.
#[test]
fn corruption_during_update_discards_staged() {
    let (effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1), // → Ready
            Event::UpdateRequest,          // → Updating (AuthenticateUpdate, StageUpdate)
            Event::CorruptionDetected(C1), // Required corruption preempts the update
        ],
    );
    assert_eq!(state, State::Recovering(C1));
    assert!(
        effects.contains(&Effect::DiscardStaged),
        "leaving Updating for recovery must discard the staged image",
    );
    assert!(
        effects.contains(&Effect::ReportUpdateAborted),
        "preempting an in-flight update must report it aborted, not drop it silently",
    );
}

/// The abort report is *only* for a genuine preemption. An `Isolable`
/// corruption of another device while `Updating` is contained (`Handled`) and
/// the update continues untouched — so neither `DiscardStaged` nor
/// `ReportUpdateAborted` is emitted, and the machine stays in `Updating`.
#[test]
fn contained_corruption_during_update_does_not_abort() {
    let (effects, state) = drive(
        chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::passive_isolable()),
        ]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1), // → Ready
            Event::UpdateRequest,          // → Updating
            Event::CorruptionDetected(C1), // isolable: contained, update survives
        ],
    );
    assert_eq!(state, State::Updating);
    assert!(!effects.contains(&Effect::ReportUpdateAborted));
    assert!(!effects.contains(&Effect::DiscardStaged));
}

/// GAP 2 (red): a second `Required` corruption while already recovering clobbers
/// the single `Recovering` slot, and because `Restored` is id-blind a restore
/// for the *displaced* target is mis-credited to the new one. Correct behavior:
/// a `Restored` whose id is not the recovery target does not advance recovery.
#[test]
fn restored_for_wrong_component_does_not_advance_recovery() {
    let (_effects, state) = drive(
        passive_required(&[C0, C1]),
        &[
            BOOT,
            Event::VerificationPassed(C0),
            Event::VerificationPassed(C1), // → Ready
            Event::CorruptionDetected(C0), // → Recovering(C0)
            Event::CorruptionDetected(C1), // clobbers → Recovering(C1)
            Event::Restored(C0),           // restore of the *displaced* target
        ],
    );
    assert_eq!(
        state,
        State::Recovering(C1),
        "a Restored for a non-target component must not be credited to the current recovery",
    );
}

// ---------------------------------------------------------------------------
// Property / model test
//
// Instead of enumerating hand-picked traces, drive the machine with thousands
// of *arbitrary* event sequences — including nonsensical and adversarial
// orderings a hostile platform could inject — and assert the one safety
// invariant that no example-based test generalizes across the whole input
// space. It complements the example suite: those pin *behavior*, this pins
// *safety* over every ordering.
//
// The lockdown-absorbing and membership properties are intentionally *not*
// re-checked here — they are already covered by dedicated example tests
// (`self_verification_failure_latches_immediately`, the boundary-guard
// `*_out_of_chain_id_is_dropped` cases), so repeating them under the fuzzer
// would add cost without signal.
//
// Invariant checked:
//   INV8  Verify-before-release. A component is released (`ReleaseReset`)
//         only if it was verified (`VerifyFirmware`) since its most recent
//         hold. A component starts held; `AssertReset` and `RecoverComponent`
//         re-hold it. So a component is never released on a stale verification
//         from before it was last taken down — the whole-input-space form of
//         "recovery is a re-boot" / "no live component trusted without an
//         at-rest recheck". This is the property the quiesce change introduced
//         and the one no single example trace captures.
// ---------------------------------------------------------------------------

/// SplitMix64 — a tiny, dependency-free deterministic PRNG. `no_std`/bazel
/// friendly: no external proptest/quickcheck crate required.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish value in `0..n`.
    fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
}

/// Build one random event over the given id palette. Id-less events ignore it.
fn random_event(rng: &mut SplitMix64, ids: &[ComponentId]) -> Event {
    let id = ids[rng.below(ids.len() as u32) as usize];
    match rng.below(15) {
        0 => Event::VerificationPassed(id),
        1 => Event::VerificationFailed(id),
        2 => Event::ComponentReady(id),
        3 => Event::Booted(id),
        4 => Event::BootConfirmed(id),
        5 => Event::CorruptionDetected(id),
        6 => Event::Restored(id),
        7 => Event::Timeout(id),
        8 => Event::AttestationChallenge,
        9 => Event::UpdateRequest,
        10 => Event::UpdateVerified,
        11 => Event::UpdateRejected,
        12 => Event::RecoveryFailed,
        13 => Event::CommitTimeout,
        _ => Event::EffectFailed,
    }
}

#[test]
fn property_verify_before_release_holds_under_random_sequences() {
    const RUNS: u64 = 4000;
    const MAX_LEN: u32 = 24;

    // C0..C2 are in-chain; C3 is intentionally out-of-chain — fed as noise so
    // the fuzzer also exercises the dispatch-boundary guard, but membership is
    // asserted by the dedicated boundary-guard example tests, not here.
    let palette = [C0, C1, C2, C3];

    for seed in 0..RUNS {
        let mut rng = SplitMix64(seed.wrapping_mul(0xD1B5_4A32_D192_ED03).wrapping_add(1));

        let ch = chain(&[
            (C0, ComponentAttrs::passive_required()),
            (C1, ComponentAttrs::active_isolable()),
            (C2, ComponentAttrs::passive_required()),
        ]);
        let mut orch =
            Orchestrator::<CAPACITY, ECAP>::new(ch.try_into().expect("valid chain"), MAX_RETRY);
        let mut platform = Recorder::new();

        // Power on first — usually a clean provisioned boot, occasionally a
        // degraded power-on result so lockdown paths get exercised too.
        let boot = match rng.below(12) {
            0 => Event::PowerGood(PowerOnResult::Unprovisioned),
            1 => Event::PowerGood(PowerOnResult::SelfVerificationFailed),
            _ => BOOT,
        };
        orch.dispatch(&mut platform, boot);

        let len = 1 + rng.below(MAX_LEN);
        for _ in 0..len {
            let event = random_event(&mut rng, &palette);
            orch.dispatch(&mut platform, event);
        }

        // Post-hoc structural scan over the full effect trace.
        let trace = &platform.recorded;

        // INV8: verify-before-release. A component starts held; AssertReset
        // and RecoverComponent re-hold it; VerifyFirmware clears it for release.
        let mut verified = [false; CAPACITY];

        for effect in trace {
            match effect {
                Effect::AssertReset(id) | Effect::RecoverComponent { id, .. } => {
                    verified[id.get() as usize] = false;
                }
                Effect::VerifyFirmware(id) => {
                    verified[id.get() as usize] = true;
                }
                Effect::ReleaseReset(id) => {
                    assert!(
                        verified[id.get() as usize],
                        "seed {seed}: released {id:?} without a verify since its last hold",
                    );
                }
                _ => {}
            }
        }
    }
}

/// A returned event settles in the same run: a platform that answers every
/// `VerifyFirmware` in place walks a provisioned chain to `Ready` from one
/// dispatch.
#[test]
fn returned_verdicts_settle_in_one_dispatch() {
    struct InstantVerify {
        recorded: Vec<Effect>,
    }
    impl Platform for InstantVerify {
        fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
            self.recorded.push(effect);
            Ok(match effect {
                Effect::VerifyFirmware(id) => Some(Event::VerificationPassed(id)),
                _ => None,
            })
        }
    }

    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(
        passive_required(&[C0, C1]).try_into().expect("valid chain"),
        MAX_RETRY,
    );
    let mut platform = InstantVerify {
        recorded: Vec::new(),
    };

    orch.dispatch(&mut platform, BOOT);

    assert_eq!(orch.state(), State::Ready);
    assert_eq!(
        platform.recorded,
        std::vec![
            Effect::ReadFirmware(C0),
            Effect::VerifyFirmware(C0),
            Effect::ReleaseReset(C0),
            Effect::ReadFirmware(C1),
            Effect::VerifyFirmware(C1),
            Effect::ReleaseReset(C1),
        ],
    );
}

/// A batch whose executors return more events than the pending queue holds
/// fails closed: the run latches `Locked` instead of losing feedback.
#[test]
fn returned_event_overflow_latches_locked() {
    struct Chatty;
    impl Platform for Chatty {
        fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
            Ok(match effect {
                // The re-walk after Restored quiesces every live component
                // and re-verifies the first — on a full 8-chain that is more
                // returned events than PENDING_CAP in one batch.
                Effect::AssertReset(_) | Effect::ReadFirmware(_) => {
                    Some(Event::AttestationChallenge)
                }
                Effect::VerifyFirmware(id) => Some(Event::VerificationPassed(id)),
                _ => None,
            })
        }
    }

    let ids: Vec<ComponentId> = (0..8).map(ComponentId::new).collect();
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(
        passive_required(&ids).try_into().expect("valid chain"),
        MAX_RETRY,
    );
    let mut platform = Chatty;

    orch.dispatch(&mut platform, BOOT);
    assert_eq!(orch.state(), State::Ready);

    orch.dispatch(&mut platform, Event::CorruptionDetected(C0));
    orch.dispatch(&mut platform, Event::Restored(C0));

    assert_eq!(orch.state(), State::Locked);
}

/// A failed effect with the pending queue already full still latches: the
/// latch evicts the newest queued event rather than being dropped itself.
/// Regression test — a back-of-queue push would be lost here, and the run
/// would settle to `Ready` as if nothing failed.
#[test]
fn failed_effect_with_full_queue_still_latches() {
    struct ChattyThenFail {
        rewalking: bool,
    }
    impl Platform for ChattyThenFail {
        fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
            match effect {
                // The re-walk after Restored asserts reset on all eight live
                // components first — exactly PENDING_CAP returned events, so
                // the queue is full (but never overflowed) when the read
                // that follows fails.
                Effect::AssertReset(_) if self.rewalking => Ok(Some(Event::AttestationChallenge)),
                Effect::ReadFirmware(_) if self.rewalking => Err(EffectError),
                Effect::VerifyFirmware(id) => Ok(Some(Event::VerificationPassed(id))),
                _ => Ok(None),
            }
        }
    }

    let ids: Vec<ComponentId> = (0..8).map(ComponentId::new).collect();
    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(
        passive_required(&ids).try_into().expect("valid chain"),
        MAX_RETRY,
    );
    let mut platform = ChattyThenFail { rewalking: false };

    orch.dispatch(&mut platform, BOOT);
    assert_eq!(orch.state(), State::Ready);

    orch.dispatch(&mut platform, Event::CorruptionDetected(C0));
    platform.rewalking = true;
    orch.dispatch(&mut platform, Event::Restored(C0));

    assert_eq!(orch.state(), State::Locked);
}

/// An event returned by executing `LatchLockdown` itself is queued, settles
/// in `Locked`, and is discarded — the latch stays terminal and nothing is
/// actuated after the lockdown.
#[test]
fn event_returned_during_lockdown_is_discarded() {
    struct FailRelease {
        recorded: Vec<Effect>,
    }
    impl Platform for FailRelease {
        fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
            self.recorded.push(effect);
            match effect {
                Effect::ReleaseReset(_) => Err(EffectError),
                Effect::LatchLockdown => Ok(Some(Event::AttestationChallenge)),
                _ => Ok(None),
            }
        }
    }

    let mut orch = Orchestrator::<CAPACITY, ECAP>::new(
        passive_required(&[C0]).try_into().expect("valid chain"),
        MAX_RETRY,
    );
    let mut platform = FailRelease {
        recorded: Vec::new(),
    };

    orch.dispatch(&mut platform, BOOT);
    orch.dispatch(&mut platform, Event::VerificationPassed(C0));

    assert_eq!(orch.state(), State::Locked);
    assert_eq!(platform.recorded.last(), Some(&Effect::LatchLockdown));
}
