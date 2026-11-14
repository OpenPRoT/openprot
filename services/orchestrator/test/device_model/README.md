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

## Public seams

- `DeviceModel::new(script)` builds a device from a `&[Behaviour]`. It starts
  with reset asserted.
- `Behaviour` is what one boot attempt does: `Hang` (never ready),
  `Boots { after_millis }` (ready after a delay, stays up), or
  `BootsThenFails { after_millis, up_millis }` (ready, then drops and stays down
  until the next reset).
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

## Host tests

```sh
bazel test //services/orchestrator/test/device_model:device_model_test
```

The tests run on a host with no kernel. They check boot timing, late polls, a
reset mid-boot, pulses between polls, script indexing, boot-then-fail,
`next_event_at` in every phase, queue overflow and reuse, a clock that goes
backwards, and a delay that would overflow.
