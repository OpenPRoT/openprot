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
  self-contained activation and the Update Agent sends ActivateFirmware. A
  flow where the RoT writing the image and resetting the device is the
  activation, with `activate()` never called and `PerformActivate` never
  produced, has no scenario.
- The pending-reset handshake: a pending reset signal to the device, the
  device preparing for shutdown, and an acknowledgement. This exists
  nowhere, not in DSP0267, not in pldm-lib, not in the orchestrator.
- Authenticating the running image before the first release. The verifier
  reads nothing and the device table carries no layout, so the first boot
  is unverified. Doing it for real means two processes mapping the FMC,
  with the Firmware Device writing CS1 while the RoT reads it.
- Updating the backup partition after the commit. #512 merged, but the
  component here binds `SvnFloorBinding::SelfManaged`, and since 993219b8
  a self-managed component's slots are left to the device. The re-sync
  cannot trigger until a scenario variant binds an eRoT floor.
- Boot progress over MCTP. The managed device in the spec is one device
  that is the Update Agent, the reset target and the progress reporter at
  once. Here those are two apps, and progress arrives on an IPC channel.

## What the transfer-error scenario does not say

`pldm_update/transfer_error` has the Update Agent answer one
RequestFirmwareData with an error completion code. The device aborts the
transfer, which is the claim. Two things it does not cover: a dropped response
rather than an errored one, which would be a T2 retry and then a T1 timeout,
and `RetryRequestFwData`, which the device is supposed to answer by sending the
request again. The run also ends with `run_terminus` returning an error rather
than the device going back to idle, so what a second update attempt after an
abort would do is untested.

## Arcs of the update state machine with no scenario

- `UpdateSecurityRevision` on the eRoT's own floor. The downstream half
  runs end to end in `pldm_update/svn_commit`: the Update Agent sends
  0x22 after the activation, the device parks at `SvnCommitPending`, and
  the loop dispatches `Event::SvnCommitRequest`, then sends
  PerformSvnCommit if the state machine emitted `Effect::CommitSvnFloor`
  and RejectSvnCommit if it did not. `svn_commit_refused` runs the reject
  side, a second request with nothing outstanding. The eRoT's own SVN
  floor has no equivalent yet. It takes the same path: the eRoT is a Firmware
  Device like any other, so the Update Agent offers it an update over
  PLDM and sends it 0x22 afterwards. Nothing serves that side on the
  eRoT yet, so
  `commit_self_svn_floor` still has no caller outside tests. The other
  update scenarios keep dispatching the event themselves, the way they
  already dispatch `BootConfirmed`: without the 0x22 exchange nothing
  parks their device at the status.
- A self-update in one run. The two scenarios split it at the reset.
  `self_update` offers the eRoT a candidate, stages it, authenticates it
  and reports the SVN it carries, and the activation records the session
  and marks the trial pending; the run ends where the eRoT would reboot.
  `self_trial` starts from that session and judges the boot. Joining them
  needs a reset this target does not have (nothing drives SYSRESETREQ, and
  QEMU models one chip) and a session in flash rather than in RAM, so the
  session would survive the reset into the same run.
- A candidate that came from somewhere. `self_update` is its own update
  source: the bytes are a constant and the slot counts them rather than
  writing them, because nothing serves the eRoT's own update over PLDM
  and no board wires a flash-backed `Updatable` for that slot.
- `Ready` to `Recovering`. `full_update/recovery_preempts` reports
  corruption while the component is `Updating`; nothing reports it on a
  component sitting `Ready`, so `Ready` to `Recovering` is never taken
  and an `Isolable` component is never gated.
- A recovery loop that never ends. `device_hangs` wires no recovery
  source, so it locks on the first attempt. With a source behind it, a
  device that verifies but never boots recovers forever:
  `Event::VerificationPassed` clears the component's retry count, so the
  attempt handed to `restore` stays 0 and `MAX_RETRY` never bites. Found
  while writing `recovery`, which is why the source is wired in the two
  recovery cfgs alone. Whether the count should survive a passing
  verification is a question for the machine.
- The spare slot re-sync after a commit, and the floor advance held until
  the spare has the image. It cannot run on any board here: the re-sync
  sits below the binding test in `commit_svn_floor`, so a component that
  keeps its own SVN never reaches it, and all three boards bind
  `SvnFloorBinding::SelfManaged`. A scenario needs a device whose floor
  the eRoT holds, which is a board shape no itest has yet.
- The same two for the slots the eRoT boots from, which nothing
  implements, not just nothing covers. `commit_self_svn_floor` advances
  the floor as soon as the trial is confirmed, and no pass re-stages the
  slot the eRoT stopped booting from. `queue_slot_resync` cannot serve
  the eRoT: the claim it reads, `last_activated`, is RAM, and the
  self-reset wipes it. The ROM rejects an image below the counter, so after
  the advance the other slot is unbootable, and a corruption of the running
  slot leaves the eRoT with nothing to fall back to.
- A board that wires no self-update at all. `Board.self_update` is an
  `Option` and `()` satisfies `SelfUpdate` and `SelfReset`, so such a
  board builds and refuses an update aimed at the eRoT at submit, with
  `NoErotUpdate`. Self-resiliency is required of every eRoT, so the
  `Option` is a bring-up seam for QEMU and the mock boards rather than a
  configuration a platform may ship; nothing in the types says so. Two
  stages close it: a wiring type with a named cannot-self-update variant,
  so each board states the choice the way `SvnFloorBinding` makes it
  state who keeps a floor, and then a plain field once a board wires a
  real self-update.

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
image settles through `settle_self_update_at_boot`, which dispatches the
settlement event without calling a verifier. Measuring it needs a seam on
that path, not another scenario.

## What the lockdown latch does not prove

`LockdownLatch` is wired and the scenarios count it: each run that ends
in lockdown expects the count, and each run that comes up expects none.
What they do not exercise is a safe state. The mock counts; a real one on
this board holds every managed device in reset and denies writes in the
SPI monitor, and answers `Ok` only when both are in force. QEMU models
one chip and there is no second device to hold, so the claim here is
about the orchestrator, not the platform.

A failed latch is invisible. The attempt happens: `Locked` entry emits the
effect and the driver calls the board once. What is lost is the failure.
The driver answers `EffectError`, and the orchestrator injects
`Event::EffectFailed` at most once per dispatch, so a run that reached
`Locked` through an earlier failure has already spent that injection;
where it has not, the event arrives in `Locked`, which is unsupervised and
drops it. Either way there is no second attempt and no report, and the
driver crate has no logger. So a board that cannot engage its safe state
leaves the state machine locked and the managed devices running, with
nothing saying so. Surfacing the failed latch needs a `Report` variant or
a log dependency in the driver crate.

## Arcs the trial gate does not cover

`DeviceTrialBoot` is wired and `full_update` closes the trial for a
downstream device both ways: confirmed when the device boots what it was given, reverted when
the device takes the update and never comes back. What no scenario reaches
is `is_pending`, which exists for an orchestrator that rebooted mid-update
and has to read an open trial for a device from stored state. Nothing calls
`is_pending`, so a trial left open by a crashed orchestrator is not
settled at the next boot. Whether a boot should settle a trial nobody claimed is also
undecided for the eRoT itself.

A revert leaves the rejected image in the slot the update wrote it to. Only
the slot metadata moves, and the one re-sync in the tree,
`queue_slot_resync` in the driver, runs on the commit path to bring the
spare slot up to the image that was just confirmed. Nothing brings the
spare back to the confirmed image after a revert, so the slot keeps an
image no verdict accepted, and nothing records the slot as stale. The
floor never advances on a revert, so the slots agree about SVN. What is
undecided is how a later reader should treat that slot: a recovery scheme
that falls back slot to slot, or a settle path for a trial left open.
Either the reader treats the contents as untrusted, or the revert
overwrites them.

## What the preemption scenario does not say

`full_update/recovery_preempts` takes the state machine from `Updating`
to `Recovering`: the corruption lands while the device is transferring,
and the driver drops the candidate and reports the update superseded to
the requester. What
the run cannot show is recovery succeeding afterwards, because the board
wires no recovery source and the episode ends in lockdown. A scenario with
both a source and an update in flight would say more.

The scenario app raises the event itself. `orchestrator_main.rs`
dispatches `CorruptionDetected` where the device is transferring; no
platform code raises it. The driver makes its verdicts from the chain
walk, and the walk does not run while the state machine is in
`Updating`, so what the transition waits on is a runtime monitor, not
another scenario.

The corruption lands on the component under update. `Updating` preempts on
a `Required` corruption of any component, including one the update is not
touching, and that transition is untaken.

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
- `Restored`. Every board wires `Recovery = ()`, which answers
  `SourceExhausted` on the first attempt, so recovery always exhausts and
  never succeeds. The retry cap and `max_retry` have no scenario.
- `AttestationChallenge`. No scenario issues one.
- `CommitTimeout`. The scenarios run their own event loop rather than
  `server/src/runtime.rs`, and nothing else arms the watchdog, so
  commit-or-lock cannot fire here. The same holds outside the tests:
  `BootWatchdogs::arm_commit` has no caller but the ast10x0 runtime test,
  so the activated-but-not-committed window is unbounded on a real
  platform.
- `EffectFailed`. The effects a scenario runs either cannot fail or are
  never made to fail: `StubVerifier` never opens an image, and
  `Recovery = ()` returns a verdict rather than an error. No scenario
  makes an actuation fault, so the lockdown that follows one is
  untested.

## Edge cases with no scenario

- Two components in flight. Every scenario supervises one managed
  device. Recovery of one component during the commit window of another,
  and the id scoping that decides what the recovery voids, are covered
  only by unit tests.
- A device that reports ready twice with no reset in between, or reports
  for a component that was never released.
- An update aimed at the eRoT itself. No scenario sends one.

## Resets in the middle of a flow

Each scenario boots once, so anything that needs a reset mid-flow has no
scenario at all. The ones that matter:

- Between `BootConfirmed` and the request the Update Agent sends. The
  outstanding advance lives in RAM, so a reset drops it and the later
  `SvnCommitRequest` finds nothing to move.
- During a spare-slot re-sync. The claim that a pass is in flight is
  RAM-only too, so a reset leaves a half-written spare with no durable
  record that it is dirty. `driver/src/board.rs` records that open
  question under `Report::SlotResyncFailed`.
- Between the device taking its SVN and the eRoT writing its mirror. The
  two floors disagree afterwards and nothing reconciles them.

## A failed spare-slot re-sync leaves the floor behind

`commit_svn_floor` holds the advance when it queues a re-sync, so the
floor takes the new SVN only once the spare holds the image. A pass that
fails clears the held advance and reports `Report::SlotResyncFailed`
(`driver/src/driver/update.rs`). The floor keeps the old SVN, so the
image the update replaced installs again, and no pass re-stages the
spare or records the slot as stale anywhere a reset would preserve.

The Update Agent hears that the commit succeeded either way. The
scenario loop sends PerformSvnCommit when the state machine emits
`Effect::CommitSvnFloor`, which happens before the pump runs, so the
answer says granted while the floor still holds the old SVN. Two changes
close this: re-attempt the pass or mark the slot stale somewhere
durable, and answer the Update Agent once the SVN floor advances
rather than when the effect arrives.

## Assertions that are looser than they look

`pldm_update/svn_commit` asserts the effect, not the outcome. The state
machine emits `Effect::CommitSvnFloor` whatever SVN floor binding the
component carries, and the orchestrator answers the Update Agent on seeing
it, but `commit_svn_floor` in the driver returns at once for a component that
keeps its own SVN (`driver.rs`, the `SvnFloorBinding::Erot` test). Every board
here binds `SelfManaged`, so no SVN floor moves in that run or in its refusal
variant. What those scenarios prove is the 0x22 exchange and the decision the
state machine takes, which is their claim; the SVN floor itself has no run.

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
  flow, but the orchestrator has no way to make the request the Update
  Agent sends fail with a PLDM completion code.
- `query_status` reports `PhaseFailed` with a fixed phase and result code
  rather than the values the device sent the Update Agent.
- The RoT does not wait on the `USER` signal the Firmware Device raises. It
  works because a `QueryStatus` sits in the channel until the device serves
  it and this RoT has nothing else to do, but a real event loop would wait
  on the signal.

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
  while the device never rebooted, because the Firmware Device was still
  declaring the verdict and its own flow had finished.
- `svn_commit` failed about one run in ten, and once on CI. Nothing is
  registered between the device's two `run_terminus` calls, so the
  Update Agent's UpdateSecurityRevision was dropped. It retries now. A
  future scenario whose agent app sends a command just after update mode
  ends needs the same retry.
