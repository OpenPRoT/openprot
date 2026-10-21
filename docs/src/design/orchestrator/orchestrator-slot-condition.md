# Slot condition

A component keeps its firmware in slots the board declares. This page works
through two slots and one golden image, the shape every board in the tree
wires today, and says at the end what changes when a board declares more.
An update writes the slot the component does not boot, activation makes that
slot the boot candidate, and the roles swap at the next reset.

Several failures leave a slot holding firmware a later boot should not use,
and the orchestrator has no way to write that down. The failure reaches a
`Report`, the report reaches the board sink, and the next boot starts with no
memory of it.

This page settles what a slot condition is, who records it, where it lives,
and how it survives a reset. It designs no code; the capability that follows
it is a separate change.

## What goes unrecorded today

A re-sync that fails partway. After a commit, `queue_slot_resync` re-stages
the confirmed image into the spare slot so both slots hold the same firmware.
If that transfer fails, `end_resync` abandons the pass, clears the held SVN
floor advance and emits `Report::SlotResyncFailed`. The running image is
fine, the spare slot holds whatever the abandoned pass left in it, and the
SVN floor stays at the old SVN. A fallback to that slot boots the old
version, or a fragment of the new one.

A reset during a staging pass. `Updatable::abandon` is infallible and says
so: cleanup a device needs, "marking a half-written slot dirty", belongs to
the implementor, deferred to the next staging if it must touch hardware. The
orchestrator keeps two RAM claims about a pass, `pending_update` while one is
in flight and `last_activated` once a commit owes a re-sync, and a reset
wipes both.

An SVN floor that moved past a slot. Anti-rollback compares an image SVN
against the SVN floor before boot, so once the floor advances past the SVN a
slot holds, the component refuses to boot that slot.
`UpdateError::ReadbackMismatch` names the other way a slot goes bad: it is
kept apart from `UpdateError::Device` so a caller can retire a slot that
keeps mismatching instead of retrying forever. No type records the
retirement.

A revert. Reverting moves slot metadata back and leaves the rejected image in
the slot the update wrote. The SVN floor never advanced, so the slots agree
about SVN, and nothing says the slot holds an image no verdict accepted.

## The vocabulary

Each condition is a failure the tree already describes in prose.

**Stale.** The slot does not hold the image the component runs and has
confirmed. A failed re-sync leaves it holding the firmware from before the
update; a revert leaves it holding the rejected image. Both boot something
the platform has already decided against. The tree uses the word elsewhere
for an event the state machine has moved past and for the older of two
durable copies; here it describes a slot.

**Half-written.** A copy into the slot stopped partway, so the slot holds a
prefix of an image. Today the driver stages first and verifies the staged
candidate afterwards, so the prefix carries no verdict at all. Once the
staging region freezes before verification and the apply step copies
verified bytes, the prefix becomes a piece of an image the eRoT accepted.
Either way the slot holds less than an image, and the condition says so
without naming which bytes arrived.

**Unbootable.** The slot holds an image the component refuses: below the SVN
floor, or failing readback often enough that the eRoT retires it. The content
may be intact and still be unusable.

Active and confirmed stay out, because the tree already answers them. Which
slot is active is device state behind the `Updatable` seam for a device that
keeps its own slot metadata, and `activate` names only "the image the last
completed staging delivered". Whether the running image is confirmed is the
trial gate: `DeviceTrialBoot` for a managed device, the `SelfUpdate` session
for the eRoT. Adding either to a slot condition would put a second owner on a
question that already has one.

## One record per slot, and one owner per component

The record holds a condition per slot, keyed by the `SlotId` the board
declares in its layout. Two slots mean two entries, and the entry for the
slot the component boots is as useful as the other one: a readback mismatch
retires the slot it happened in, whichever that is.

The owner is per component, not global. `Updatable` says a device that keeps
its own slot metadata admits no second slot-selection owner, and `activate`
carries no slot argument for that reason; `DeviceTrialBoot` draws the same
line, where `confirm` and `revert` take no arguments because a device holds
at most one open trial and which slot is which stays behind the seam. That
rule is about devices that own their slots. For a component whose flash the
eRoT writes, the eRoT is the single owner, it already picks the write target
from the layout, and a slot it cannot name is a slot it cannot repair.

So slot identity crosses the seam exactly where the eRoT owns the slots, and
stays behind it where the device does. The record follows the same line: the
eRoT keeps `SlotId`-keyed conditions for the components it writes, and keeps
nothing for a component that manages itself.

## Who records it

The binding decides. `SvnFloorBinding::Erot` means the eRoT holds the SVN
floor; `SelfManaged` means the component tracks its own SVN.
`commit_svn_floor` already reads slot ownership off that binding, and its doc
names the wart: the binding only says who keeps the SVN floor, so a board
where the SVN floor keeper and the slot keeper differ cannot be wired today.

Rather than add a second attribute no board would set, widen the binding to
what the driver already treats it as: the one place that says who keeps the
durable facts about a component, the SVN floor, the slot metadata and the
slot conditions together. `SelfManaged` keeps all three for itself and the
eRoT holds nothing for it.
The eRoT variant keeps all three, writes the slots, and records the
conditions. The rename belongs to the capability change that follows this
page; naming it `SvnFloorBinding` while it answers three questions is what
made the wart need a doc comment.

The eRoT side also needs the operations the record implies: mark a slot
unbootable, and set which slot the component boots next. Both are new seam
operations on components the eRoT owns, so both land in the capability
catalog in [Platform Architecture](./orchestrator-platform.md) once the
capability change writes them.

## A record, not a report

`ReportSink` is infallible and lossy on purpose: a report names something
that already happened, so an undeliverable one costs information rather than
containment. The three integration boards log reports to the console, and no
sink in the tree matches `SlotResyncFailed`, `SvnFloorCommitFailed` or
`SvnFloorHeld` at all.

A slot condition is state a later boot reads, so it has to survive the reset
that wipes RAM. The report says what just happened and the record says what
is true now; a board wants both.

### How it survives

Durability has a rule in the tree already, on the `SelfUpdate` trait, and the
slot condition takes the same one: two copies with a sequence number and a
checksum, writing the stale copy and reading the newest valid one, so a write
interrupted by power loss leaves the previous copy readable. The record is
fixed size, so a torn write fails its checksum instead of parsing as a
condition nobody wrote. The checksum detects a torn write and nothing more:
it is not a signature, because the record steers which slot a fallback tries
while signature verification and the SVN floor stay the gates that decide
whether that image boots.

The write happens before the operation that can fail, not after. The failure
can be the reset itself, which gives the driver no chance to write anything
when it happens, so the driver records the pessimistic condition first and
clears it on success:

- Staging into a slot marks it half-written before the first byte lands, and
  clears the mark when the transfer and the verification finish. This is
  what `pending_update` claims in RAM today.
- A commit that queues a re-sync marks the spare slot stale before the
  orchestrator answers the Update Agent, and clears the mark when the
  re-sync completes. This is what `last_activated` claims in RAM today, and
  a reset while `held_floor` is pending finds the stale mark already
  written.
- A revert marks the slot it rejected stale, and a slot the eRoT retires
  after repeated readback mismatches is marked unbootable. The driver
  observes both while running, so both are written where they happen.

A crash between the write and the operation leaves a record claiming a slot
is bad when the slot is fine. The next staging into that slot clears it. The
ordering never leaves the opposite case, a bad slot nothing flagged.

The eRoT keeps the record in its own storage, not on the device, because the
eRoT cannot read a condition out of a slot the component refuses to boot.
The same record covers the slots the eRoT boots from: no binding names the
eRoT, since `SelfUpdate` holds its session, but a half-written or stale slot
of its own is the same fact and the storage is already there.

### What it needs from storage

`util_io::Storage` is a flat byte region with sessions and split-phase
writes, no filesystem, and its doc names RAM test buffers alongside SPI NOR.
A scenario can therefore try the whole design out before the real flash
driver exists, backing the record with a RAM `Storage` it wires itself. The
mock in `util/io/storage.rs` has the shape, though `cfg(test)` keeps it
inside that crate, so a scenario either promotes it or keeps its own: a
static buffer carries the record across a simulated reset, and a
`start_write` that never completes models the torn write the checksum has
to catch. What is
still missing is a `Storage` implementor for real flash, which the eRoT
self-update session needs just as much. That blocker is shared, not new.

## The driver records it; the state machine does not

The driver writes the condition where it already runs the operation: the
staging pass, the re-sync path, the revert, the SVN floor advance. No new
`Effect` and no new `Event`.

Nothing the state machine decides reads a slot condition today, and no
transition in [State Machine](./orchestrator-machine.md#state-machine)
branches on one. No board
wires a recovery scheme that falls back slot to slot: `()` reports source
exhaustion on every attempt, and the one scenario that wires a real source
restores over the image in place and exhausts after the first try. A
scheme that walks the slots is the consumer this design serves, and how it
should treat a slot a revert left is still open: either the reader refuses
the contents, or the revert overwrites them. When that decision lands and the
state machine needs to branch on a condition, an event carries it then.

## The golden image

`ImageLayout` declares slots in recovery order and falls back to the golden
image last, because restoring from golden drops every update the platform
took since the board was made. Last in the recovery walk says nothing about
where the image sits in flash. The board picks that. SPI NOR block
protection covers a contiguous range anchored at the top or the bottom of
the part, so either end can be locked, and the start of the flash is the
natural home for an image written once at manufacture and never again.

Today the golden image is not a slot: the eRoT never writes it, the SVN floor
skips it (a golden SVN is fixed at manufacture, so an SVN floor above it would
retire the last image that still boots), and recovery restores it into a slot
before anything boots it. A board could instead boot the golden image where
it lies and skip the restore, which costs a reset less on a board whose slots
are all bad. That needs a layout that marks an image bootable but never
writable, which the current `Slot` and `Golden` types do not express, and it
needs the write protection in hardware that skipping the SVN floor already
assumes. Neither belongs in this change.

The record covers slots. A golden image the board write-protects cannot
acquire a condition the eRoT would have to record.

## Standards

DSP0267 addresses components, not slots: the Update Agent names a component
and a version, and where the Firmware Device puts the bytes is the business
of the device. Nothing in this design changes what the Update Agent sees, and
no PLDM message carries a slot condition.

NIST SP 800-193 is the yardstick that applies. Its detection and recovery
expectations are what a durable condition record serves: the platform detects
that an image is corrupt or unusable, and the recovery path reaches a
known-good image instead of retrying the bad one. The golden image the layout
already carries is the recovery image in those terms, and the record is what
lets the eRoT skip a slot it has already found unusable rather than rediscover
the failure on every boot.

## What this does not fix

The orchestrator tells the Update Agent a security revision commit succeeded
before the re-sync runs, because the answer goes out when the driver
dispatches `Effect::CommitSvnFloor`. That ordering is the driver design plus
what the integration scenario does with it; no shipped adapter answers the
Update Agent yet. Recording the stale slot does not change the answer. Both
appear in the integration gaps under "A failed spare-slot re-sync leaves the
floor behind", and the answer timing is a separate change to when the
orchestrator replies.

Nothing here repairs a slot. A stale or half-written slot stays wrong until
something re-stages it, and the scheme that would is the slot-to-slot
recovery named above.

## More than two slots

A board that declares three or more slots changes the size of the record and
nothing else: the conditions are keyed by `SlotId` from the start, so a
larger layout means more entries. What a larger layout does need is a rule
for picking the next write target, which two slots answer by elimination.
Round robin over the declared slots is the obvious first algorithm, and the
choice is per component and const, like the rest of the layout. Both the
policy and the config that selects it are future work.
