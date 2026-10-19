# What the QEMU integration tests do not cover

Written 2026-10-05. The scenarios and how to run them are in README.md;
this is the list of what a green run still does not say.

## The two halves have met, in one scenario

`full_update` closes this: five apps in one image, an update over PLDM
followed by the device being reset into it and reporting ready a second
time. Checked against its negative, a device that stops reporting after
the post-activation reset.

It carries two negatives of its own: a device that never comes up, and a
device that takes the update and never comes back. `mock_downstream_device` and
`pldm_update` keep theirs.

## The lines are IPC channels, not GPIOs

Reset and boot-worked are board traces between two chips. QEMU models one
chip and the mock downstream device is a process inside it, so there is no second
device for a pin to reach. The adapters in `board/src/bmc.rs` are swapped
out at the trait seam rather than exercised. Pin wiring is what the
hardware tests are for.

## It is not a black box

The orchestrator runs as an app inside the image and the scenarios assert
on its state from in there. The only observations from outside the guest
are the UART sentinel and the CS1 backing file, and only the happy path
checks the latter.

## Parts of the DSP0267 Type 5 flow with no scenario

`full_update` covers the happy path: Type 0 discovery, inventory, request,
pass-component, update-component, the data loop, transfer, verify, apply,
activate, reset, and the device booting what it was given. The following
parts of the specification are not exercised:

- Automatic activation without ActivateFirmware. The device advertises
  self-contained activation and the agent sends ActivateFirmware. A flow
  where the RoT writing the image and resetting the device is the
  activation, with `activate()` never called and `PerformActivate` never
  produced, has no scenario.
- The pending-reset handshake: a pending reset signal to the device, the
  device preparing for shutdown, and an acknowledgement. This exists
  nowhere, not in DSP0267, not in pldm-lib, not in the orchestrator.
- Authenticating the running image before the first release. The verifier
  reads nothing and the device table carries no layout, so the first boot
  is unverified. Doing it for real means two processes mapping the FMC,
  with the firmware device writing CS1 while the RoT reads it.
- Updating the backup partition after the commit. #512 merged, but the
  component here binds `SvnFloorBinding::SelfManaged`, and since 993219b8
  a self-managed component's slots are left to the device. The re-sync
  cannot trigger until a scenario variant binds an eRoT floor.
- Boot progress over MCTP. The managed device in the spec is one device
  that is the update agent, the reset target and the progress reporter at
  once. Here those are two apps, and progress arrives on an IPC channel.

## What the transfer-error scenario does not say

`pldm_update/transfer_error` has the agent answer one RequestFirmwareData
with an error completion code. The device aborts the transfer, which is the
claim. Two things it does not cover: a dropped response rather than an
errored one, which would be a T2 retry and then a T1 timeout, and
`RetryRequestFwData`, which the device is supposed to answer by asking
again. The run also ends with `run_terminus` returning an error rather than
the device going back to idle, so what a second update attempt after an
abort would do is untested.

## Arcs of the update state machine with no scenario

- `UpdateSecurityRevision` on the eRoT's own floor. The downstream half
  runs end to end in `pldm_update/svn_commit`: the agent sends 0x22 after
  the activation, the device parks at `SvnCommitPending`, and the loop
  dispatches `Event::SvnCommitRequest`, then sends PerformSvnCommit if
  the state machine emitted `Effect::CommitSvnFloor` and RejectSvnCommit
  if it did not. `svn_commit_refused` runs the reject side, a second
  request with nothing outstanding. The eRoT's own floor has no
  equivalent yet. It takes the same path: the eRoT is a firmware device
  like any other, so the agent offers it an update over PLDM and sends it
  0x22 afterwards. Nothing serves that side on the eRoT yet, so
  `commit_self_svn_floor` still has no caller outside tests. The other
  update scenarios keep dispatching the event themselves, the way they
  already dispatch `BootConfirmed`: without the 0x22 exchange nothing
  parks their device at the status.
- A self-update that is actually performed. `self_trial` starts the
  session where an activation would have left it and checks what the
  next boot does about it. Nothing stages an image into the eRoT's own
  slot, marks the trial pending or resets the eRoT, because no board
  wires a flash-backed `Updatable` for that slot and QEMU models one
  chip. The half the scenario covers is the boot after the reset, not
  the update that caused it.
- Recovery preempting an update: `Updating` to `Recovering`, with the
  staged image discarded.
- The spare slot re-sync after a commit, and the floor advance held until
  the spare has the image.

## What the measurement scenario does not say

`mock_downstream_device/measured` runs the path #546 merged: a verdict
carrying `Some(measurement)` reaches the sink as `Report::Measured`, once,
for the component the walk measured. The digest is a constant, so what it
proves is the wiring, not any hashing. Nothing computes a measurement
anywhere in the tree: the `Measurer` capability has no implementor and is
not a `BoardCapabilities` type, and the real one belongs to the crypto
lane. The staged candidate is not measured either, because the update path
verifies through `IncrementalVerifier`, which has no measurement seam.

The eRoT's own image is not measured at all. `Report::Measured` comes from
a verdict, and the walk passes over the eRoT, so it is never verified: its
image settles through `settle_self_image_at_boot`, which dispatches the
settlement event without calling a verifier. Measuring it needs a seam on
that path, not another scenario.

## Events no scenario can reach

Checked by grepping the scenario apps for each `Event`, then asking what
the wired board can produce. An app naming no event is not enough: the
driver makes `BootFailed` and `RecoveryUnavailable` on its own, and
`device_hangs` reaches both. `Timeout` and `CommitTimeout` come from
`server/src/runtime.rs`, which no scenario runs.

- `VerificationFailed`. All three scenarios wire the chain walk to
  `StubVerifier` in `services/orchestrator/test/stubs.rs`, which returns
  `Authenticated` without opening the image, so no walk can fail. The
  update candidate has a different verifier, `PatternVerifier`, which can
  fault, but that ends as `UpdateRejected`.
- `CorruptionDetected`. Nothing reports runtime corruption, so `Ready` to
  `Recovering` is never taken and an `Isolable` component is never gated.
- `Restored`. Every board wires `Recovery = ()`, which answers
  `SourceExhausted` on the first attempt, so recovery always exhausts and
  never succeeds. The retry cap and `max_retry` have no scenario.
- `AttestationChallenge`. No scenario issues one.
- `CommitTimeout`. The scenarios run their own event loop rather than
  `server/src/runtime.rs`, and nothing else arms the watchdog, so
  commit-or-lock cannot fire here.
- `EffectFailed`. The effects a scenario runs either cannot fail or are
  never made to fail: `StubVerifier` never opens an image, and
  `Recovery = ()` returns a verdict rather than an error. No scenario
  makes an actuation fault, so the lockdown that follows one is
  untested.

## Edge cases with no scenario

- Two components in flight. Every scenario supervises one managed
  device. A sibling's recovery during another component's commit window,
  and the id scoping that decides what it voids, are covered only by
  unit tests.
- A device that reports ready twice with no reset in between, or reports
  for a component that was never released.
- An update aimed at the eRoT itself. No scenario sends one.

## Resets in the middle of a flow

Each scenario boots once, so anything that needs a reset mid-flow has no
scenario at all. Three of them matter:

- Between `BootConfirmed` and the agent's request. The outstanding
  advance lives in RAM, so a reset drops it and the later
  `SvnCommitRequest` finds nothing to move.
- During a spare-slot re-sync. The claim that a pass is in flight is
  RAM-only too, so a reset leaves a half-written spare with no durable
  record that it is dirty. `driver/src/board.rs` records that open
  question under `Report::SlotResyncFailed`.
- Between the device taking its SVN and the eRoT writing its mirror. The
  two floors disagree afterwards and nothing reconciles them.

## Assertions that are looser than they look

`pldm_update` has no managed device, so its `BootConfirmed` is a
stand-in: it is dispatched once the activation is acknowledged, not once
anything booted. `full_update` is where that claim is actually tested.

The same scenario checks the machine is back in `Ready` after the update.
Since `UpdateVerified` enters `PreSupervision`, the path back to `Ready`
runs through verification, and the stub verifier passes synchronously, so
that check would hold even with no boot walk ever polled.

## Known weaknesses in the test apps

- In `full_update` the RoT owns the sentinel, so a firmware-device failure
  after the activation is acknowledged shows in the log but does not fail
  the run. The orchestrator's own path covers the cases that matter today,
  because every step needs the device's acknowledgement and a missing one
  times out, and the host checks CS1 afterwards. It does not cover the
  device's consent flags.

- The RoT's own verifier reads the staging region, which in this scenario
  is a synthetic pattern rather than the bytes the device actually staged.
  The device checks the real bytes; the RoT checks a stand-in.
- A refusal is recorded rather than enforced. The device stops its own
  flow, but the orchestrator has no way to make the agent's request fail
  with a PLDM completion code.
- `query_status` reports `PhaseFailed` with a fixed phase and result code
  rather than the values the device sent the agent.
- The RoT does not wait on the device's `USER` signal. It works because a
  `QueryStatus` sits in the channel until the device serves it and this
  RoT has nothing else to do, but a real event loop would wait on the
  signal.

## Things that bit us, kept here so they are not rediscovered

- A scenario asserting `State::Ready` passed with the device wedged. The
  pass condition has to be the thing being tested, not a state that is
  reached anyway.
- Three apps were calling `debug_shutdown`, so two sentinels reached the
  console and the runner graded whichever came first.
- The agent calls `handle_component` twice, once to pass the component
  table and once to start the component.
- An app whose thread stack is too small dies before its first log line,
  with no panic and no warning.
- An app that is misaligned or oversized still builds; the only sign is a
  PMSAv7 subregion overlap warning on the console.
- Whichever app owns the sentinel decides the run. `full_update` passed
  while the device never rebooted, because the firmware device was still
  declaring the verdict and its own flow had finished.
- `svn_commit` failed about one run in ten, and once on CI. Nothing is
  registered between the device's two `run_terminus` calls, so the
  Update Agent's UpdateSecurityRevision was dropped. It retries now. A
  future scenario whose agent app sends a command just after update mode
  ends needs the same retry.
