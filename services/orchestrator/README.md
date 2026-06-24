# OpenPRoT Resiliency Orchestrator — Architecture

## Purpose and Context

Modern platforms integrate multiple independent Roots of Trust (RoTs). Each
enforces local trust within its own boundary, but system-level security
requires coordinated behavior across all components. Without coordination,
validation, update, and recovery operate independently, producing inconsistent
system states and weak attestation guarantees.

The **Orchestrator** is a system-level control-plane function that solves this
problem. It enforces consistent global trust state across all RoTs and
boot-managed devices, covering the full lifecycle: validate, authorize,
recover if needed, release reset, and attest.

**Orchestrator as concept vs. OpenPRoT as implementation:**

> The Orchestrator is a concept — a system-level coordination function.
> OpenPRoT is one implementation of a Platform Root of Trust (PRoT) that
> includes orchestration as a capability. OpenPRoT provides update (RTU),
> recovery (RTRec), validation, attestation, and orchestration at the platform
> level. OpenPRoT includes orchestration but is not equivalent to it.

**Security guarantees the orchestrator enforces:**
- No unauthorized firmware executes before it is verified.
- The system maintains a consistent and auditable trust state at all times.
- Recovery behavior is deterministic and bounded.
- Attestation reflects post-verification, post-recovery state.

**Design principles:**
- Separation of concerns between state logic, platform effects, and event ingestion.
- Deterministic, event-driven execution — one event processed at a time.
- Composability across multiple RoTs through domain abstraction.
- Standardized interfaces between the SM, platform, and runner layers.

**The SM is the policy authority:**
- The SM defines what happens, in what order, and under what conditions.
- Platform impls (`ResiliencyPlatform`) and the runner adapt to the SM's `Effect`
  and `Event` vocabulary — they do not redefine it.
- A new target cannot change SM transitions; it can only change what happens
  inside `execute()`.
- Platform-specific code  maps native hardware events and actions to SM vocabulary. The SM stays clean.

---

## Overview

The resiliency orchestrator service is split into three layers:

1. **State machine** — pure transition logic only, no platform I/O.
2. **Platform contract** — a trait that abstracts target-specific side effects.
3. **Runner** — the integration loop that connects events, the SM, and the platform.

The ordered execution model the SM enforces:
```
PowerOn → verify → [recover if needed] → release boot holds → runtime → attest
```

This separation keeps the state machine testable without hardware, and lets
multiple target platforms implement their own effects without touching the SM.

---

## Domain Model

A **domain** is a controlled firmware or platform-security target that the
resiliency orchestrator manages as one security boundary.

The state machine does not model every internal detail of that target. It
models only the behavior that changes boot progress, verification, recovery,
update, or lockdown decisions.

**What a domain includes:**
- Its active and recovery images.
- Its verify, recover, and update flow.
- Hold and release control for boot progression.
- Checkpoints and timeout behavior.
- Failure handling and lockdown behavior.

**What a domain does not include:**
- Internal CPU or chipset micro-architecture details.
- Transport plumbing such as SMBus, I3C, or GPIO wiring.
- Flash offsets, staging mechanics, or storage layout details unless they
  affect observable behavior.
- Config-specific implementation branches unless they change reachable SM
  behavior.

**Two domains are present in the current model:**

| Domain | Role |
|--------|------|
| `RoT` | Root-of-trust firmware. Verified at INIT; recovered via ROT_RECOVERY. |
| `HostTarget` | Host-side boot-managed firmware (e.g. PCH on Intel, or equivalent on other platforms). Verified in BOOT_GATE; held/released at OPERATIONAL_PHASE. |

Using domain-level terms keeps the SM vendor-neutral. A platform impl maps
`BootTarget::HostTarget` to its concrete chipset or firmware region. The SM
never names a specific vendor.

**Modeling rule:** use domain-level terms when the behavior is about
controlling a security boundary; use implementation terms only in the platform
layer where the exact mechanism matters.

### Non-exhaustive domain set

The architecture does not require a closed list of domains. The domain set is
intentionally extensible.

**Required core domains:**
- `RoT` — root-of-trust firmware; the SM's trust anchor. Cannot be absent.
- `HostTarget` — the primary host-side boot-managed firmware region. Cannot be
  absent; without it OPERATIONAL_PHASE boot-hold/release has no target.

**Optional domains:**
- Any additional boot-managed or attestable firmware/security target. Examples:
  device-attestation firmware, management-controller side targets, or
  platform-specific programmable logic regions.

---

### Extension points

There are four explicit extension points in the architecture. Each has a
defined rule for what may change and what must stay fixed.

#### EP-1 — `BootTarget` variants (domain identity)

**What it is:** The `BootTarget` enum in `effect.rs` identifies which domain
a `HoldBoot` / `ReleaseBoot` effect targets.

**How to extend:** Add a new variant to `BootTarget`.

**Rules:**
- The enum is `#[non_exhaustive]`. All platform `match` arms must have a
  wildcard. New variants will not break existing platform impls silently.
- Adding a variant does not change any SM state handler or superstate logic.
- The platform impl decides what hardware signal the new variant maps to.

**What must not change:** existing variant names and their semantics.

---

#### EP-2 — `Effect` variants (side-effect vocabulary)

**What it is:** The `Effect` enum in `effect.rs` is the complete vocabulary of
things the SM can ask the platform to do.

**How to extend:** Add a new variant for a new class of side effect.

**Rules:**
- New variants must be emitted only from entry/exit actions, never from state
  transition handlers.
- Each new variant must have a single, clearly named responsibility.
- Platform impls that do not support the new effect must handle it explicitly
  (log and ignore, or assert — never silently drop through an unguarded arm).
- Effects must be stateless requests: the SM does not track whether an effect
  was executed or what it returned.

**What must not change:** the queue-and-drain dispatch model; effects must
remain fire-and-forget from the SM's perspective.

---

#### EP-3 — `Event` variants (input vocabulary)

**What it is:** The `Event` enum in `lib.rs` is the normalized set of inputs
the SM accepts.

**Events do not change per platform.** The same `Event` enum is used on every
target. What varies is how hardware signals are translated into events — that
translation is the runner's job:

```
AST10x0:  watchdog register fires  →  runner reads register  →  Event::WatchdogTimeout
other:    watchdog IPC notification →  runner reads channel   →  Event::WatchdogTimeout
```

The SM always receives `Event::WatchdogTimeout` regardless of how the
underlying hardware delivered it.

The one nuance is config-gated events. `SeamlessUpdateRequested` only makes
sense on platforms that support seamless updates. The runner suppresses it when
the capability is absent — it never reaches the SM. The `Event` enum still
contains the variant; the runner simply never generates it on that platform.

**How to extend:** Add a new variant for a new input signal or async result.

**Rules:**
- New events must be generated by the runner or platform adapter, never
  synthesized inside the SM itself.
- New events must be handled in at least one state or superstate handler;
  unhandled events in all states are silently discarded by `statig` — document
  which states consume the new event.
- If a new event is config-gated, the runner must suppress it when the
  capability is absent rather than sending it to the SM.

**What must not change:** existing event semantics and variant names.

---

#### EP-4 — `ResiliencyPlatform` implementations (target adapters)

**What it is:** Each target provides a concrete `impl ResiliencyPlatform` that
maps `Effect` values to hardware calls, register writes, or service calls.

**How to extend:** Implement the trait for a new target in
`target/<target>/orchestrator/`.
- Implementations must handle every `Effect` variant, including future ones
  (wildcard arm required when `Effect` is marked `#[non_exhaustive]`).
- Config-gated behavior (watchdog policy, checkpoint recovery, attestation
  backend) lives entirely here. The SM does not branch on config flags.
- The platform impl must not feed new events back into the SM synchronously
  during an `execute()` call — only the runner's event loop posts events.

**What must not change:** the trait signature. Extending the trait (adding
methods) is a breaking change and requires all existing impls to update.

---

## Layer 1 — Pure State Machine (`openprot-resiliency-sm`)

**Location:** `services/orchestrator/`

**Responsibilities:**

- Define normalized input events (`Event`).
- Define normalized output effects (`Effect`).
- Implement state, superstate, and action handlers on `Orchestrator`.
- Queue `Effect` values during entry/exit actions for the runner to drain.

**Constraint:** state handlers and actions never call platform code directly.
All side-effect intent is expressed by pushing an `Effect` onto
`Orchestrator::pending`.

### Key types

```
pub enum Event { ... }          // Normalized inputs (hardware + async results + commands)
pub enum VerifyResult { ... }   // Payload carried by VerifyComplete
pub enum Effect { ... }         // Platform-side requests emitted by state actions
pub struct Orchestrator { ... } // State machine context + pending effect queue
```

### Effect queue

`Orchestrator` carries a bounded, inline effect queue:

```rust
pub struct Orchestrator {
    pub recovery_attempts: u8,
    pub boot_count:        u32,
    pub provisioned:       bool,
    pub(crate) pending:    heapless::Vec<Effect, 16>,
}
```

Entry and exit actions push effects:

```rust
fn on_enter_system_reboot(&mut self) {
    self.boot_count = self.boot_count.saturating_add(1);
    self.pending.push(Effect::LogPanic).ok();
    self.pending.push(Effect::Reboot).ok();
}
```

The runner drains effects after each `sm.handle()` call:

```rust
sm.handle(&event);
for effect in sm.drain_effects() {
    platform.execute(effect);
}
```

---

## What is an Effect?

An `Effect` is a request the state machine emits to tell the platform "do this
side effect for me," without doing it itself.

State handlers run inside the `statig` dispatch loop and only have access to
`&mut self`. They cannot call hardware functions directly without coupling the
SM to a specific platform. Instead of calling `hold_bmc_boot()` directly, a
state action pushes `Effect::HoldBoot(BootTarget::HostTarget)` onto a queue.
The runner picks it up after `sm.handle()` returns and dispatches it to the
platform.

**Without effects (coupled — avoid this):**

```rust
fn on_enter_system_reboot(&mut self) {
    self.boot_count += 1;
    pfr_cpld_update_reboot();   // hardware call baked into the SM
}
```

**With effects (decoupled):**

```rust
fn on_enter_system_reboot(&mut self) {
    self.boot_count += 1;
    self.pending.push(Effect::LogPanic).ok();
    self.pending.push(Effect::Reboot).ok();  // intent only, no hardware call
}
```

The runner dispatches effects after the SM returns:

```rust
sm.handle(&event);
for effect in sm.drain_effects() {
    platform.execute(effect);   // hardware call happens here, outside the SM
}
```

In one sentence: an `Effect` is the SM's way of saying "something needs to
happen in the real world" without knowing or caring how that world works.

### One event, many effects

A single event can produce multiple effects, because a state transition
traverses the hierarchy and fires entry/exit actions on every state boundary
crossed. Each action can push one or more `Effect` values.

**Example — `BootComplete` while in `boot_gate`:**

1. `on_enter_operational_phase` → queues `ArmMonitors`, `ReleaseBoot(HostTarget)`

Two effects from one event.

**Example — `RebootRequested` from `runtime`:**

1. `on_exit_runtime` → queues `DisarmWatchdog`
2. `on_exit_operational_phase` → queues `DisarmMonitors`
3. `on_enter_system_reboot` → queues `LogPanic`, `Reboot`

Four effects from one event, across three state boundaries. The runner drains
them all in order after `sm.handle()` returns.

---

## Layer 2 — Platform Contract

**Location:** `services/orchestrator/src/platform.rs`

A single trait consumed by the runner:

```rust
pub trait ResiliencyPlatform {
    fn execute(&mut self, effect: Effect);
}
```

Each target provides its own implementation.
Config-gated behavior (seamless update, SPDM attestation, checkpoint recovery)
lives in the platform impl, not in the SM.

**Test stub** (in-crate, `#[cfg(test)]`):

```rust
pub struct NoopPlatform;
impl ResiliencyPlatform for NoopPlatform {
    fn execute(&mut self, _: Effect) {}
}
```

---

## Layer 3 — Runner / Integration

**Location:** `target/<target>/orchestrator/`

The **runner** is the per-target `main`-side loop that stitches all three layers
together: it waits for incoming events on an IPC channel, feeds them into the
SM one at a time, then drains and dispatches the
resulting `Effect` values through the platform impl. It contains no policy logic
— only sequencing and wiring.

The runner owns:

- Event ingestion (IPC channel reads via `syscall::channel_read`).
- Serialization (one event processed at a time, in order).
- Platform wiring (concrete `ResiliencyPlatform` impl for the target).

**Canonical event loop shape:**

```rust
// NOTE: wire encoding for Event is not yet defined.
// A future services/orchestrator/api/ crate (mirroring services/mctp/api/)
// will provide the binary wire format and decode_event().
loop {
    // Block until the event channel is readable
    syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX);
    let len = syscall::channel_read(handle::EVENTS, 0, &mut buf)?;
    let event = decode_event(&buf[..len]);   // wire → Event; to be defined
    sm.handle(&event);
    for effect in sm.drain_effects() {
        platform.execute(effect);
    }
}
```

---

## Effect catalog

`BootTarget` identifies which domain's boot line is being controlled:

```rust
pub enum BootTarget {
    RoT,
    HostTarget,
}
```

The platform impl maps each `BootTarget` variant to the concrete hardware
signal or register for that target (e.g. BMC hold line, PCH hold line).

| Effect | When emitted | Platform action |
|--------|-------------|------------------|
| `HoldBoot(BootTarget)` | BOOT_GATE entry | Assert boot-hold for the given domain |
| `ReleaseBoot(BootTarget)` | OPERATIONAL_PHASE entry (provisioned path) | Deassert boot-hold for the given domain |
| `ArmWatchdog` | RUNTIME entry | Start domain watchdog timers |
| `DisarmWatchdog` | RUNTIME exit | Stop domain watchdog timers |
| `ArmMonitors` | OPERATIONAL_PHASE entry | Enable reset and platform monitors |
| `DisarmMonitors` | OPERATIONAL_PHASE exit | Disable reset and platform monitors |
| `SetPlatformState(s)` | Various entry actions | Publish state over IPC / LEDs |
| `LogPanic` | SYSTEM_REBOOT entry | Record last-panic cause |
| `Reboot` | SYSTEM_REBOOT entry | Issue hardware reboot (does not return) |
| `HaltBoot` | SYSTEM_LOCKDOWN entry | Hold all domains; set lockdown platform state |

---

## State hierarchy (current implementation)

```
Boot
Init
RotRecovery
BootGate              (superstate)
├── FirmwareVerify
├── FirmwareRecovery
├── FirmwareUpdate
└── SystemLockdown
OperationalPhase      (superstate)
├── Unprovisioned
├── Runtime
├── SeamlessUpdate
└── SeamlessVerify
SystemReboot
```

`SystemReboot` and `SystemLockdown` are terminal for the current boot session.
A hardware reboot restarts the machine externally; no `RebootComplete` event
is consumed because the process does not resume.

`BootGate` is the phase where the platform is still holding downstream domains
(e.g., `HostTarget`) in reset while verification and recovery are performed.
Boot holds are released only on transition into `OperationalPhase`.

`OperationalPhase` is the phase after that, where boot holds have been
released and the platform enters normal boot and runtime behavior.

---

## File layout (target state)

```
services/orchestrator/               ← orchestrator service root
  BUILD.bazel                        ← future: runner + platform-integration targets
  ARCHITECTURE.md                    ← this file
  sm/                                ← resiliency state machine component
    src/
      lib.rs                         ← Orchestrator struct, states, superstates, actions
      effect.rs                      ← Effect enum + BootTarget
      platform.rs                    ← ResiliencyPlatform trait + NoopPlatform test stub
    BUILD.bazel                      ← rust_library: openprot_resiliency_sm

target/<target_name>/orchestrator/          ← per-target runner
  src/
    runner.rs                        ← event loop, event ingestion
    platform.rs                      ← impl ResiliencyPlatform for the target
  BUILD.bazel
```

---

## What stays in the SM vs. what goes to the platform

| Concern | Layer |
|---------|-------|
| State transitions and guards | SM |
| Recovery attempt counter | SM (shared context field) |
| Boot count | SM (shared context field) |
| Provisioning flag | SM (shared context field) |
| Domain boot hold/release | Platform (via `Effect::HoldBoot` / `ReleaseBoot`) |
| Watchdog arm/disarm | Platform (via Effect) |
| Hardware reboot call | Platform (via Effect::Reboot) |
| IPC channel reads | Runner (event ingestion) |
| Config-gated behavior | Platform impl or Runner |
| SPDM attestation, PIT, CPLD | Platform impl |

---

## How to extend

1. Add `effect.rs` with the `Effect` enum.
2. Add `platform.rs` with the `ResiliencyPlatform` trait and `NoopPlatform` stub.
3. Wire `pending: heapless::Vec<Effect, 16>` into `Orchestrator` and add `drain_effects()`.
4. Update BOOT_GATE/OPERATIONAL_PHASE entry/exit actions to push the relevant `Effect` values.
5. Update `on_enter_system_reboot` and `on_enter_system_lockdown` to push their effects.
6. Update BUILD.bazel to add `heapless` dependency.
7. Implement the runner and a concrete `ResiliencyPlatform` per target.
