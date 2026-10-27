<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchestrator platform driver (`openprot_orchestrator_driver`)

The effect-executing layer around the orchestrator state machine. `PlatformDriver`
implements the SM's `Platform` seam: one executor per `Effect`, each
documenting its obligation from the platform-boundary contract
([orchestrator-model.md §6](../../../docs/src/design/orchestrator/orchestrator-model.md)).
An effect whose capability is not composed yet returns `EffectError` from
`execute`; the SM fail-closes on it.

Everything device-specific arrives through the seams in `board.rs`
(`ImageSource`, `Verifier`, `BootControl`, `BootWatch`, bundled in `Board`).
Synchronous results (the verification verdict) return through `execute` and
settle within the same dispatch run — there is no driver-side event queue.

Boot-walk verdicts are the one asynchronous read. `ReleaseReset` arms the
component's walk; the run loop polls and dispatches until quiet, then sleeps
until the earliest walk deadline:

```rust
orch.dispatch(&mut driver, event);
loop {
    let poll = driver.poll_boot_walks(now_millis);
    match poll.event {
        Some(ev) => orch.dispatch(&mut driver, ev),
        None => break, // poll.next_deadline_millis = next wake-up
    }
}
```

Implemented executors: `ReadFirmware`, `VerifyFirmware`, `ReleaseReset`
(arms the boot walk), `AssertReset` (stops it). Everything else fails closed
until its pillar lands (recovery, update path, attestation, reporting,
lockdown latch).
