# Orchestrator State Machine

The orchestrator is the eRoT's boot-sequence controller. It walks the platform
trust chain — verifying each component's firmware and releasing it from reset in
order — and then governs the operational lifecycle (attestation, firmware update,
corruption recovery).

It lives in `services/orchestrator/sm` as a pure state machine: it never touches
hardware directly. Every action is described as an [`Effect`] value that the
surrounding shell carries out; every piece of outside information arrives as an
[`Event`]. This keeps the core testable without hardware and free of I/O.

## Documents

- [**Verification Model**](./orchestrator-model.md): The two-tier firmware
  verification model (eRoT gate + optional iRoT gate), the verification
  boundary, `ComponentAttrs`, and concrete sequencing examples.
- [**State Machine**](./orchestrator-machine.md): All states, shared storage, entry
  actions, transition table, and the `Operational` superstate.

## Design Principles

**Effects, not actions.** Handlers call `ctx.emit(Effect::…)` to describe what
should happen. The shell's `Platform::execute` carries it out. The core never
reads flash, drives a GPIO, or opens a channel.

**Reads as events.** The core never reads OTP, UFM, or any provisioning store.
Outside information (power-on result, verification verdicts, iRoT readiness
signals) arrives in event payloads.

**Feedback as data.** Internal follow-up signals (e.g. the retry-cap lockdown
`RecoveryFailed`) are emitted as `Effect::Emit(event)`. The orchestrator queues
and handles them immediately, making them visible in the effect trace rather than
hiding them as implicit state changes.

**Board-supplied policy.** The core hard-codes no deployment-specific values.
The shell supplies the trust chain (component ids, kinds, and required/optional
policy) and the recovery-retry cap at startup.

## Relationship to CSA Architecture

The state machine is a direct implementation of the boot sequence described in
the CSA architecture document:

| CSA concept | State machine encoding |
|---|---|
| eRoT holds component in reset until firmware verified | `VerifyingPlatform` emits `ReleaseReset` only on `VerificationPassed` |
| Component with Caliptra iRoT requires two independent checks | `ComponentKind::Active` → `AwaitingReady` until `ComponentReady` |
| Passive component (no iRoT): eRoT check only | `ComponentKind::Passive` → advance immediately after `ReleaseReset` |
| Optional component: failure skips, not blocks | `ComponentAttrs::required = false` → advance without `Recovering` |
