# Orchestrator

The orchestrator is the eRoT's boot-sequence controller. It walks the platform
trust chain — verifying each component's firmware and releasing it from reset in
order — and then governs the operational lifecycle (attestation, firmware update,
corruption recovery).

It lives in `services/orchestrator/sm` as a pure state machine: it never touches
hardware directly. Every action is described as an [`Effect`] value that the
surrounding platform carries out; every piece of outside information arrives as an
[`Event`]. This keeps the core testable without hardware and free of I/O.

## State Topology

The reachable states, the events (with guards) that drive transitions between
them, and the effects each transition emits are all shown in the full state
diagram in [State Machine](./orchestrator-machine.md#state-machine). That diagram
is the single source of truth for the topology; it is not duplicated here to
avoid the two drifting apart.

## Documents

- [**Verification Model**](./orchestrator-model.md): The two-tier firmware
  verification model (eRoT gate + optional iRoT gate), the verification
  boundary, `ComponentAttrs`, and concrete sequencing examples.
- [**State Machine**](./orchestrator-machine.md): All states, shared storage, entry
  actions, transition table, and the `SupervisingPlatform` superstate.
- [**Platform Architecture**](./orchestrator-platform.md): The platform half around
  the core — surrounding services, capability contracts, the board device table,
  and the fail-safe rules at the responsibility boundary.
- [**Runtime**](./orchestrator-runtime.md): How the runtime loop
  gathers hardware interrupts, IPC channel messages, and watchdog deadlines into
  the core's event stream, and carries the resulting effects and decisions back
  out.

## Design Principles

**Effects, not actions; reads as events.** Handlers only call
`ctx.emit(Effect::…)` to describe what should happen, and receive every piece of
outside information in event payloads — the core never reads flash, drives a
GPIO, opens a channel, or touches a provisioning store. The full core/platform
split is the [Platform Boundary](./orchestrator-model.md#5-the-platform-boundary)
in the Verification Model.

**Feedback as data.** Internal follow-up signals (e.g. the retry-cap lockdown
`RecoveryFailed`) are emitted as `Effect::Emit(event)`. The orchestrator queues
and handles them immediately, making them visible in the effect trace rather than
hiding them as implicit state changes. See the
[`Recovering` state](./orchestrator-machine.md#recovering) for a worked example.

**Board-supplied policy.** The core hard-codes no deployment-specific values.
The platform supplies the trust chain (component ids, kinds, and required/optional
policy) and the recovery-retry cap at startup.

## Board composition

*Board composition* (or *system composition*) is the per-target choice — described
declaratively in a [`system.json5`](../../architecture.md) file, assembled by
Pigweed at build time — of how the platform's functions are split across
processes and which resources each process owns: hardware register blocks and the
kernel [interrupt objects and IPC channels](../pw-kernel-ipc.md) built on top of
them. It is separate from the
board-supplied *policy* above: policy is *what* to verify (the trust chain);
composition is *how* the surrounding services are wired. The orchestrator core
and its platform-agnostic crates are the same across every composition — a driver
may own a GPIO bank and forward boot-progress over a channel in one image, while
the orchestrator holds the pins directly in another. That choice changes which
inbound sources the [runtime](./orchestrator-runtime.md) sees and who owns each
device, but never the core's states or the loop that serves them.

## Relationship to CSA Architecture

The state machine is a direct implementation of the boot sequence described in
the CSA architecture document:

| CSA concept | State machine encoding |
|---|---|
| eRoT holds component in reset until firmware verified | `PreSupervision` emits `ReleaseReset` only on `VerificationPassed` |
| Component with Caliptra iRoT requires two independent checks | `ComponentKind::Active` → `AwaitingReady` until `ComponentReady` |
| Passive component (no iRoT): eRoT check only | `ComponentKind::Passive` → advance immediately after `ReleaseReset` |
| Isolable component: failure skips, not blocks | `FailurePolicy::Isolable` → skip (held in reset); advance without `Recovering`; no cascade |
| Cascading skip: failure also holds dependents | `FailurePolicy::Cascading` + `ComponentAttrs::depends_on` → cascade-skip via `statuses` (`Isolated`) |
| Boot-progress watchdog: component must signal readiness in time | `Timeout(ComponentId)` event → `AwaitingReady` → `Recovering` |
| Recovery scope groups components that restore together | `ComponentAttrs::recovery_region` (`RegionId`) → platform restores full region on `RestoreGoldenImage` |

## Applicability Across Admissible Architectures

The orchestrator is **eRoT-scoped**: one instance runs per discrete eRoT chip
running OpenPRoT firmware. The same crate and binary are used at every such
tier — the only difference between deployments is the chain configuration
supplied at startup. eRoT chips not running OpenPRoT (e.g. a third-party AMC
or a legacy DC-SCM implementation) appear as opaque `ComponentId` entries in
the chain of the nearest OpenPRoT eRoT above them.

**Model 1 — Module** (single PCIe add-in card): a discrete eRoT is optional.
When present, it runs one orchestrator instance with a short chain containing
the card's SoC. When absent, the module relies on the parent node's eRoT
instance to verify and release it as a `ComponentId` in the node's chain.

**Model 2 — Single Node Compute**: the DC-SCM discrete eRoT runs one
orchestrator instance whose chain covers all critical node devices (CPU, BMC,
NIC, storage). This is the canonical single-instance deployment.

**Model 3 — Complex Heterogeneous Compute**: the three-tier hierarchy maps to
independent orchestrator instances at each tier:

- Each **AMC / EAM** (subsystem eRoT) runs one instance whose chain covers the
  GPU or AI accelerator devices it manages.
- The **DC-SCM node eRoT** runs one instance whose chain covers CPUs, BMC,
  SmartNICs, storage, and the AMC/EAM chips themselves.

The node eRoT treats each AMC/EAM as just another `ComponentId` — `Active` if
the AMC has its own integrated iRoT, `Passive` if not. It has no visibility
into the AMC's internal chain walk; it only observes the AMC's
`VerificationPassed` / `ComponentReady` signals, like any other component.

| Admissible Architecture | Orchestrator instances | Chain scope per instance |
|---|---|---|
| Module (eRoT present) | 1 | Card SoC |
| Module (no eRoT) | 0 | — (verified by parent node eRoT) |
| Single Node Compute | 1 | All node critical devices |
| Complex Heterogeneous Compute | 1 per subsystem eRoT + 1 node eRoT | Subsystem devices / all node devices including subsystem eRoTs |
