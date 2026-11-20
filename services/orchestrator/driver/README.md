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
(`ImageSource`, `Verifier`, `BootControl`, `BootWatch`, bundled in `Board`). Nothing the
driver executes waits on a slow service. Short local calls (reset lines, the
SVN floor, reports) complete inside `execute`; the two long reads, the
verification verdict and the boot-walk verdicts, are polled. There is no
driver-side event queue.

`VerifyFirmware` starts a check and returns. The verdict arrives from
`poll_verify` as `VerificationPassed` or `VerificationFailed`; a verifier that
ran past its deadline reports `VerifyStatus::TimedOut`, which the driver
reports as `VerificationFailed` so the machine recovers the component. Starting
a check, or re-staging the image, cancels any check still outstanding, and its
answer is discarded. `ReleaseReset` arms the component's boot walk. The run
loop calls `poll`, dispatches each `Event` it returns, then sleeps on `Idle` until
`next_wake_millis` (the earlier of the earliest walk deadline and the verifier's
deadline) or a signal. It never sleeps without polling after the last dispatch,
because a new check's deadline is learnt only from its first poll. A
verifier that stops answering signals nothing, so its deadline is the only wake-up
that turns its silence into a failed verification:

```rust
orch.dispatch(&mut driver, event);
loop {
    // Read the clock each round.
    match driver.poll(now_millis) {
        // Something is ready: dispatch it, then poll again before sleeping.
        Ok(DriverPoll::Event(ev)) => orch.dispatch(&mut driver, ev),
        // Quiet: the only place the loop blocks. It wakes when a service
        // signals or next_wake_millis passes, then polls again.
        Ok(DriverPoll::Idle { next_wake_millis }) => {
            object_wait(wait_group, Signals::READABLE, next_wake_millis)?;
        }
        // The check could not be carried through: fail closed, as an
        // executor error does.
        Err(_) => orch.dispatch(&mut driver, Event::EffectFailed),
    }
}
```

Implemented executors: `ReadFirmware`, `VerifyFirmware` (starts the check), `ReleaseReset`
(arms the boot walk), `AssertReset` (stops it). Everything else fails closed
until its pillar lands (recovery, update path, attestation, reporting,
lockdown latch).
