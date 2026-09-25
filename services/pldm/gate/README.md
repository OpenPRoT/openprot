# pldm_gate

The orchestrator's side of the PLDM update gate. `#![no_std]`,
host-buildable, depends on `pldm_api` and the orchestrator's config schema.

The firmware device parks at each phase and raises a nudge. The orchestrator
reads `FdStatus` and answers with one operation. This crate turns a status
into that answer.

## `AlwaysPerform`

The policy that answers every phase with proceed.

```rust
let gate = AlwaysPerform::new(STAGING);   // a Region, board wiring

match gate.decide(status) {
    Decision::Idle => {}                     // nothing to answer, wait
    decision => send(decision),
}
```

| status | answer |
|---|---|
| `OfferPending` | `AcceptOffer { staging }` |
| `VerifyPending` | `PerformVerify` |
| `ApplyPending` | `PerformApply` |
| `ActivationPending` | `PerformActivate` |
| `SvnCommitPending` | `PerformSvnCommit` |
| `Cancelled` | `AckCancel` |
| `Idle`, `ReadyXfer`, `PhaseFailed` | `Decision::Idle` |

It exists so the update path runs end to end before any real policy is
written, and so a test can drive every phase without one.

**It is not a policy and must not ship on a device.** It makes no checks: not
component isolation, not the SVN floor, not whether the component is one this
orchestrator manages. A test pins the property that it never refuses
anything, which is the whole of what it does.

A real gate replaces it and brings the refusing decisions with it. When a
second policy exists, the two share a trait; one implementation does not
need one.
