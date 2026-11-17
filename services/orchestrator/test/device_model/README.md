<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# device model (`openprot_device_model`)

A managed device as seen from its reset line, for tests. Hold reset and the
device is down; release it and one boot attempt starts. A script says what each
attempt does. It is a pure state machine: `#![no_std]`, no dependencies, no
kernel code, no clock, no heap.

Tests of the orchestrator's boot walk and recovery need a stand-in device. For
the test to mean anything the **device decides its own behaviour**. If the code
under test also tells the device how to act, it can shape the world it is judged
against, and a bug in it (a double release, say) can quietly change the script
instead of failing.

## What a device is

A device is the hardware on the far side of a reset line. While reset is held it
is down; when reset is released it starts one boot attempt, and it reports one
fact: whether it is ready. The model captures only that.

A device is **not a service**. Real downstream devices (BMC flash, NIC, retimer)
are never talked to directly. The orchestrator reaches them through platform
services such as the reset controller, GPIO and MCTP, each its own task over IPC.
The model stands in for the device itself, and a **carrier** exposes it: pins on
a board, or a `ServiceLoop` over IPC in a simulation. The model never touches
IPC, so it does not change with the carrier.

The device decides its own behaviour. Its script is fixed when the image is
built, and the orchestrator can only drive reset and listen.

The planned consumer is the `devices` app of the `async_sim` simulation (#716).
That app stands in for the reset controller service: a `reset` channel
(release, assert, restore) and an `events` long-poll that reports boots, with
one `DeviceModel` per component behind it. Until that lands, nothing else in
the repo uses this crate.

## Public seams

- `DeviceModel::new(script)` builds a device from a `&[Behaviour]`. It starts
  with reset asserted.
- `Behaviour` is what one boot attempt does: `Hang` (never ready),
  `Boots { after_millis }` (ready after a delay, stays up), or
  `BootsThenFails { after_millis, up_millis }` (ready, then drops and stays down
  until the next reset). A hung device gives no signal at all, so the
  orchestrator learns of a hang only when its own deadline expires.
- `set_reset(asserted, now_millis)` drives the reset line.
- `poll(now_millis)` advances time and returns the `Phase`
  (`Held`, `Booting`, `Ready`, `Hung`, `Down`). `ready()` is the ready level.
- `next_event_at()` is when the next change is due, so an event loop can sleep
  until then.
- `take_transition()` drains ready changes as `Transition { at_millis, ready }`,
  so a pulse that rose and fell between two polls is not lost.

The model does not know what carries these signals. A board adapter can map them
to pins; a simulation can map them to IPC messages.

## Invariants

- **The delay counts from the release.** A reset mid-boot throws the attempt
  away, and the next release starts a fresh one.
- **Asserting reset always wins.** It drops ready and returns the device to
  `Held` from any phase. Repeating a reset level changes nothing.
- **The script is indexed by release.** The `n`th release follows the `n`th
  entry, the last entry repeats, and an empty script hangs.
- **Changes are stamped with when they were due,** not when a late caller
  noticed them, and a late poll applies every change that fell due, in order.
- **Nothing panics.** Time and counters saturate; a clock that goes backwards
  simply leaves nothing due. The transition queue holds 4 entries; overflow is
  counted in `dropped()`, not hidden.

## Known gaps: running on `PlatformDriver`

`async_sim` (#716) drives the real `orchestrator-sm` core through its own
asynchronous executor against the `Platform` contract in `orchestrator-sm`. It
deliberately does not use `PlatformDriver` (`services/orchestrator/driver`),
because the simulation's point is that the orchestrator never blocks on a
service. Some seams of the driver would fit:

- `BootControl`: queue the reset command and return.
- `BootWatch`: `arm` and `poll`, fed by the boot long-poll and the boot window.
- `ImageSource`, `SvnFloor` and `ReportSink`: stubs are enough.

Three gaps stop the simulation's scenarios from running on the driver as it is:

1. **Verify is synchronous.** `Verifier::verify` returns the verdict and
   `verify_firmware` returns it from `execute`, so a slow verify blocks. The
   `Platform` contract wants such verdicts delivered later as their own events.
   The seam would need an `arm`/`poll` shape and `execute` would return
   `Ok(None)`.
2. **There is no verify deadline or cancel.** `verify_hangs` cannot be
   expressed.
3. **`RecoverComponent` is not implemented.** The driver returns `EffectError`,
   which locks the machine, so `flaky_boot` and `never_boots` could not re-walk.
   A recovery seam has to be composed first.

## Host tests

```sh
bazel test //services/orchestrator/test/device_model:device_model_test
```

The tests run on a host with no kernel. They check boot timing, late polls, a
reset mid-boot, pulses between polls, script indexing, boot-then-fail,
`next_event_at` in every phase, queue overflow and reuse, a clock that goes
backwards, and a delay that would overflow.
