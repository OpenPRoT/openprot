<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchestrator state machine (`openprot_orchestrator_sm`)

Pure-reducer eRoT boot-sequence state machine. Walks the platform trust chain
— verifying each component's firmware and releasing it from reset in order —
then governs the operational lifecycle (attestation, firmware update, corruption
recovery).

**No I/O, no hardware.** Every action is an [`Effect`] the surrounding shell
carries out. Every piece of outside information arrives as an [`Event`].

## Key types

| Type | Role |
|---|---|
| `ComponentId` | Opaque `u8` — the shell maps it to hardware; the core never inspects it. |
| `ComponentKind` | `Active` (eRoT + iRoT gates) or `Passive` (eRoT gate only). |
| `ComponentAttrs` | `kind` + `required`: if `false`, a failed component is skipped (held in reset) rather than triggering recovery. |
| `Orchestrator<N>` | Public handle for the caller's event loop. Call `dispatch` or `dispatch_with` once per event. |
| `Platform` | Implement this to carry out effects (drives reset GPIOs, reads flash, etc.). |

## Usage

```rust
use openprot_orchestrator_sm::{
    ComponentAttrs, ComponentId, Orchestrator, Event, PowerOnResult, State,
};

const CAPACITY: usize = 3;
const BMC:  ComponentId = ComponentId::new(0);
const HOST: ComponentId = ComponentId::new(1);
const NIC:  ComponentId = ComponentId::new(2);

let mut chain = heapless::Vec::<_, CAPACITY>::new();
let _ = chain.push((BMC,  ComponentAttrs::active_required()));
let _ = chain.push((HOST, ComponentAttrs::active_required()));
let _ = chain.push((NIC,  ComponentAttrs::passive_optional()));

let mut orch = Orchestrator::new(chain, /*max_retry=*/ 3);
let mut board = MyBoard;

orch.dispatch(&mut board, Event::PowerGood(PowerOnResult::Provisioned));
// ...deliver VerificationPassed / ComponentReady events as they arrive...
assert_eq!(orch.state(), State::Ready);
```

## Design docs

Full domain model, verification boundary, and state transition tables are in the
OpenPRoT book:

- `docs/src/design/orchestrator/verification-model.md`
- `docs/src/design/orchestrator/state-machine.md`
