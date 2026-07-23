# Verification Model

This document describes how platform firmware verification is modelled in the
orchestrator state machine (`services/orchestrator/sm`): the problem it solves,
the types that carry the domain, the states that sequence the work, and the
boundary between the pure core and the platform shell that executes it.

---

## 1. The Problem

The eRoT (external Root of Trust — the discrete RoT device, e.g. on a DC-SCM)
must verify every platform component's firmware before releasing it from reset.
Two independent mechanisms do this:

1. **eRoT-side**: the eRoT reads the component's firmware image from the SPI
   flash it controls, verifies the signature and SVN against a Reference
   Integrity Manifest (RIM/PFM), and only then releases the component from reset.

2. **iRoT-side**: components with an integrated Root of Trust (e.g. a BMC SoC
   or CPU with Caliptra) perform their own independent local self-verification
   after reset. The eRoT must wait for this local check to complete before
   treating the component as trusted and advancing to the next one in the chain.

Components that have no integrated iRoT (e.g. a NIC) rely solely on the
eRoT-side check. The eRoT can advance immediately after releasing them.

This two-tier model — eRoT gate + optional iRoT gate — is the core problem the
verification states solve. It is grounded directly in the CSA architecture boot
sequence: "The eRoT and the iRoT provide complementary guarantees: the eRoT
controls whether a component is released from reset; the iRoT controls whether
the component's own firmware executes."

The **verification boundary** is the interface between the platform shell and the
pure state-machine core. Only verdicts cross it: the shell performs all
cryptographic work (reading flash, checking signatures and SVN) and then signals
the outcome via an event. The core never sees raw firmware data or hash values —
it only acts on the resulting `VerificationPassed` or `VerificationFailed`. This
keeps the core free of I/O and testable without hardware.

---

## 2. Domain Types

### `ComponentKind`

Classifies the iRoT gate for a component. Supplied by the shell at chain-build
time; the core never derives it.

```
Active  — has an integrated iRoT (e.g. Caliptra); both eRoT and iRoT checks apply
Passive — no integrated iRoT; only the eRoT check applies
```

### `ComponentAttrs`

Per-component attributes that combine two orthogonal axes:

```rust
pub struct ComponentAttrs {
    pub kind: ComponentKind,  // iRoT gate: Active | Passive
    pub required: bool,       // failure policy: true = recover, false = skip
}
```

| `kind` | `required` | `VerificationFailed` behaviour |
|---|---|---|
| Active / Passive | `true` | → `Recovering`; component held in reset; chain walk halts |
| Active / Passive | `false` | component held in reset; cursor advances; chain walk continues |

A `required: false` component that fails verification is **never** released from
reset — releasing a component whose firmware failed verification would mean
running untrusted code, which breaks the trust invariant regardless of the
recovery policy.

Convenience constructors: `ComponentAttrs::active_required()`,
`passive_required()`, `active_optional()`, `passive_optional()`.

### `ComponentId`

An opaque `u8` the core carries and equality-compares but never inspects. The
shell decides which id maps to which physical device.

### Events that cross the verification boundary

| Event | Direction | Meaning |
|---|---|---|
| `VerificationPassed(ComponentId)` | shell → core | The eRoT-side check passed: signature and SVN valid. |
| `VerificationFailed(ComponentId)` | shell → core | The eRoT-side check failed: image rejected. |
| `ComponentReady(ComponentId)` | shell → core | An `Active` component's integrated iRoT has finished its local verification and the component is operational (e.g. MCTP channel established). |

### Effects the core emits for verification work

| Effect | Meaning |
|---|---|
| `ReadFirmware(ComponentId)` | Ask the shell to read the component's firmware image from eRoT-controlled flash. |
| `VerifyFirmware(ComponentId)` | Ask the shell to verify the image against the RIM/PFM. The shell responds with `VerificationPassed` or `VerificationFailed`. |
| `ReleaseReset(ComponentId)` | Release the named component from reset. Emitted only after `VerificationPassed`. |

These are descriptions, not actions. The shell's `Platform::execute` carries
them out; the core never touches hardware.

---

## 3. Sequencing by `ComponentAttrs`

### Active → Passive (happy path)

```
chain: [(C0, {Active, required}), (C1, {Passive, required})]

VerifyingPlatform (entry):
  emit ReadFirmware(C0)
  emit VerifyFirmware(C0)

VerificationPassed(C0):           ← eRoT check done
  emit ReleaseReset(C0)
  emit ReadFirmware(C1)           ← speculative eRoT check of next
  emit VerifyFirmware(C1)
  cursor = 1, awaiting = Some(C0)
  → AwaitingReady

ComponentReady(C0):               ← C0's iRoT done
  awaiting = None
  Handled (stay in AwaitingReady, wait for VerificationPassed(C1))

VerificationPassed(C1):           ← speculative eRoT check resolved
  emit ReleaseReset(C1)
  chain done → Ready
```

### Optional component failure (skip, continue)

```
chain: [(BMC, {Active, required}), (NIC, {Passive, optional})]

VerificationPassed(BMC):
  emit ReleaseReset(BMC)
  emit ReadFirmware(NIC)
  emit VerifyFirmware(NIC)
  awaiting = Some(BMC) → AwaitingReady

VerificationFailed(NIC):          ← NIC firmware rejected; optional → skip
  NIC stays held in reset
  cursor advances past end
  awaiting is still Some(BMC) → stay in AwaitingReady

ComponentReady(BMC):              ← BMC iRoT done; cursor past end → Ready
  awaiting = None → Ready
```

### Concrete example: BMC (Active, required) → HOST (Active, required) → NIC (Passive, optional)

This matches the CSA single-node boot sequence.

```
chain: [(BMC, {Active, required}), (HOST, {Active, required}), (NIC, {Passive, optional})]

VerifyingPlatform (entry):
  emit ReadFirmware(BMC)
  emit VerifyFirmware(BMC)          ← eRoT reads and checks BMC firmware from SPI flash

VerificationPassed(BMC):            ← eRoT: BMC firmware signature + SVN valid
  emit ReleaseReset(BMC)            ← eRoT releases BMC from reset; Caliptra iRoT runs
  emit ReadFirmware(HOST)           ← speculative: eRoT starts HOST firmware check
  emit VerifyFirmware(HOST)           while BMC's Caliptra iRoT is still booting
  cursor = 1, awaiting = Some(BMC)
  → AwaitingReady

ComponentReady(BMC):                ← BMC Caliptra iRoT done; MCTP channel up
  awaiting = None
  Handled (still in AwaitingReady, waiting for VerificationPassed(HOST))

VerificationPassed(HOST):           ← eRoT: HOST firmware signature + SVN valid
  emit ReleaseReset(HOST)           ← eRoT releases HOST from reset; Caliptra iRoT runs
  emit ReadFirmware(NIC)            ← speculative: eRoT starts NIC firmware check
  emit VerifyFirmware(NIC)            while HOST's Caliptra iRoT is still booting
  cursor = 2
  Handled (stay in AwaitingReady — still waiting on ComponentReady(HOST) and/or NIC result)

ComponentReady(HOST):               ← HOST Caliptra iRoT done; BIOS/UEFI executing
  awaiting = None
  Handled

VerificationPassed(NIC):            ← eRoT: NIC firmware valid (Passive — no iRoT gate)
  emit ReleaseReset(NIC)
  chain done → Ready
```

If NIC fails verification instead:
```
VerificationFailed(NIC):            ← NIC optional → skip; NIC stays held in reset
  cursor = 3 (past end)
  awaiting = None (already cleared by ComponentReady(HOST))
  → Ready
```

---

## 4. The Speculative Read Pattern

When an `Active` component passes eRoT verification the core does three things
in the same handler, before transitioning to `AwaitingReady`:

```
emit ReleaseReset(current)
emit ReadFirmware(next)        ← speculative: next eRoT check starts immediately
emit VerifyFirmware(next)      ← while current's iRoT is still booting
cursor += 1
awaiting = Some(current)
→ Transition(AwaitingReady)
```

This overlaps the integrated iRoT boot time of the current component with the
eRoT firmware read of the next. The two checks are independent (different
hardware paths), so the overlap is safe.

---

## 5. The Platform Boundary

The core never reads flash, never checks signatures, never observes reset lines.
It only emits descriptions. The complete split:

| Responsibility | Core (`sm/src/lib.rs`) | Shell (`Platform` impl) |
|---|---|---|
| Chain order and `ComponentAttrs` | reads from `Rot.chain`, set by shell at startup | decides and provides |
| Read firmware image | emits `ReadFirmware(id)` | executes: eRoT reads via SPI interposition, I3C, or other transport |
| Verify signature / SVN | emits `VerifyFirmware(id)` | executes: eRoT checks against RIM/PFM; responds with `VerificationPassed` or `VerificationFailed` |
| Release from reset | emits `ReleaseReset(id)` | executes: eRoT drives reset GPIO or equivalent |
| Detect iRoT readiness | waits for `ComponentReady(id)` event | observes: integrated iRoT signals readiness (MCTP channel-up, GPIO, etc.); calls `dispatch` |
| Required vs optional failure policy | checks `attrs.required` in handler | none — policy is encoded in the chain at startup |

---

## 6. What This Model Does Not Cover

- **Self-verification of the eRoT firmware itself**: happens one boot layer down
  (eRoT ROM + measuring bootloader) before this machine runs. The result is
  delivered as `PowerOnResult` in `Event::PowerGood`.
- **Attestation** (`AttestationChallenge` / `SignAttestation`): handled in the
  `Operational` superstate, not part of the boot-time verification chain.
- **Firmware update verification** (`AuthenticateUpdate`): handled in the
  `Updating` state, distinct from boot-time chain verification.
