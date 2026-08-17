<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchestrator platform driver (`openprot_orchestrator_driver`)

The effect-executing layer around the orchestrator state machine. `PlatformDriver`
implements the SM's `Platform` seam: one method per `Effect`, each
documenting its obligation from the platform-boundary contract
([orchestrator-model.md §6](../../../docs/src/design/orchestrator/orchestrator-model.md)). Unimplemented executors return
`DriverError::NotImplemented`; the SM fail-closes on them.

Everything device-specific arrives through the seams in `board.rs`
(`ImageSource`, `Verifier`, bundled in `Board`); executor-produced events
return to the SM via `PlatformDriver::take_event`. The event loop dispatches an
outside event, then keeps dispatching what the executors produced until
`take_event` returns `None`:

```rust
orch.dispatch(&mut driver, event);
while let Some(ev) = driver.take_event() {
    orch.dispatch(&mut driver, ev);
}
```

Implemented executors: `read_firmware`, `verify_firmware`. Everything else
returns `NotImplemented` until its pillar lands (boot walk, recovery,
update path, attestation, reporting).
