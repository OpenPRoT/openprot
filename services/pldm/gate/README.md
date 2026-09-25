# pldm_gate

The orchestrator's side of the PLDM update gate. `#![no_std]`,
host-buildable, depends only on `pldm_ipc_api`.

The firmware device asks before it acts. It parks at each phase, raises a
nudge, and the orchestrator reads `FdStatus` and answers with one operation.
This crate turns a status into that answer.

## `AlwaysGrant`

The policy that permits everything.

```rust
let gate = AlwaysGrant::new(0x2000_0000);   // staging base, board wiring

match gate.decide(&status) {
    Decision::Idle => {}                     // nothing to answer, wait
    decision => send(decision),
}
```

| status | answer |
|---|---|
| `OfferPending` | `AcceptOffer { staging_base }` |
| `VerifyPending` | `GrantVerify` |
| `ApplyPending` | `GrantApply` |
| `ActivationPending` | `GrantActivate` |
| `SvnCommitPending` | `GrantSvnCommit` |
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
