# orchestrator-update-api

The update intake seam between the orchestrator and an update source (the
PLDM firmware-device service in `services/pldm` today, a self-update path
later). Contract only: the `UpdateIntake` trait a source calls, the
`IntakeStatus` phases the orchestrator answers with, and the `wire` encoding
that carries both over a kernel channel. No transport, no policy, no state,
so both processes depend on it and it builds and tests on the host.

The source is the channel's initiator and the orchestrator its handler, and
there is no channel the other way. The orchestrator never waits on the
source, and every request it answers is one bounded step, so a wedged update
source cannot delay a boot window or a recovery. Everything the orchestrator
has to say comes back as the answer to a request the source made: each
response carries the current phase, and the source reads it with `poll` as
often as its own protocol needs.

One update, from the source's side:

1. `offer(target, total)` reserves the staging region.
2. `write(offset, bytes)` fills it, at most 512 bytes per call, in any order.
3. `complete()` declares the candidate complete, which is what starts the
   update.
4. `poll()` until `Activated` or `Failed`.

What backs the staging region is the board's choice. Where the eRoT sits in
the target's flash path there is no room to hold a copy, so a write goes
straight into that device's inactive slot and the eRoT authenticates by
reading the slot back. The phase order follows from that and is not fixed:
see the `IntakeStatus` docs before assuming one.

`abort()` drops the job from any phase. Activation needs no call: the state
machine activates on its own verdict once the candidate authenticates.

Run the tests with:

    bazel test //services/orchestrator/update-api:orchestrator_update_api_test
