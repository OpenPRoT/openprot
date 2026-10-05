# What the QEMU integration tests do not cover

Written 2026-10-05. The scenarios and how to run them are in README.md;
this is the list of what a green run still does not say.

## The two halves have not met

The boot scenario has the reset and boot-worked lines but no PLDM. The
update scenario has the full PLDM path but no managed device, so its
`BootConfirmed` is a stand-in for a device coming back rather than one
that was watched coming back. Nothing yet runs an update and then sees
the device reboot into it.

Merging the two images is the next piece of work and the one the demo
script needs: the agent offers an image over PLDM, the RoT stages it,
activates, resets the device, the device reports ready, the floor
commits.

## The lines are IPC channels, not GPIOs

Reset and boot-worked are board traces between two chips. QEMU models one
chip and the mock BMC is a process inside it, so there is no second
device for a pin to reach. The adapters in `board/src/bmc.rs` are swapped
out at the trait seam rather than exercised. Pin wiring is what the
hardware tests are for.

## It is not a black box

The orchestrator runs as an app inside the image and the scenarios assert
on its state from in there. The only observations from outside the guest
are the UART sentinel and the CS1 backing file, and only the happy path
checks the latter.

## Arcs of the update state machine with no scenario

- Cancel mid-transfer. The agent's `CancelUpdate` and the `AckCancel` that
  answers it. The device's IPC handler refuses `ack_cancel` today.
- `UpdateSecurityRevision` and the `SvnCommitPending` status, so
  `commit_self_svn_floor` has no path that reaches it.
- Commit timeout, and the commit-or-lock latch that bounds the
  activated-but-not-committed window.
- Recovery preempting an update: `Updating` to `Recovering`, with the
  staged image discarded.
- The re-walk after an activation. It happens, since `UpdateVerified`
  enters `PreSupervision`, but nothing asserts the device was reset into
  the image it just activated.
- The spare slot re-sync after a commit, and the floor advance held until
  the spare has the image.

## Assertions that are looser than they look

`handle_activated` checks the machine is back in `Ready`. Since
`UpdateVerified` enters `PreSupervision`, the path back to `Ready` runs
through verification, and the stub verifier passes synchronously, so the
check would hold even if no boot walk were ever polled. The boot scenario
had the same shape before its negative case caught it: a green that does
not depend on the thing being tested.

## Known weaknesses in the test apps

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
