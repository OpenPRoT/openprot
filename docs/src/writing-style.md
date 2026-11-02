# Writing style

This page covers the prose in the tree: code comments, doc comments, commit
messages, pull request bodies, issues, and the documents under `docs/`.
Formatting rules for code live in [Coding Style](./coding-style.md).

Be precise and concise, both. Say exactly what holds, in the fewest words
that still say it. Concision never excuses dropping part of the contract,
precision never excuses padding, and concision that costs the reader a
second pass is not concision.

Write plain matter-of-fact sentences. Say what a thing is and what it does,
with commands, paths and file names in place. The target voice is a
colleague explaining the code, not a specification.

## Name the actor

Write in the active voice and put the actor in the subject. "The commit
advances the SVN floor" rather than "the SVN floor is advanced". Passive
voice drops the actor, which is usually the fact worth stating. Keep the
passive only where the actor is the hardware or the signer and naming it
would mislead, as in "the image is signed".

Absolutes hide the actor the same way. "Nothing confirms it", "never runs"
and "always holds" leave the reader asking which component was supposed to
act. Name the actor and say why it does not act: "the boot that would
confirm the session will not come". Keep an absolute only where the absence
is the fact being stated and no actor exists to name, which is why GAPS.md
is full of them.

Write what happens rather than what does not. "The device boots the old
image at the next reset" says more than "nothing brings the new image
back". A gaps document is the exception, because absence is its subject.

Name the actor in full the first time and whenever two could be meant: a
downstream device rather than "a device", the state machine rather than
"the machine", the eRoT rather than "the RoT". Shorten only where one actor
is in play and the full name is in the same paragraph.

Replace a pronoun with the name unless the noun sits in the same sentence.
That applies to "there", "this", "that flag", "one" and "it". Before
sending, search what you wrote for those words and either substitute the
name or confirm the noun is in the same sentence.

Name what a bare noun acts on. Not "the advance" but "the SVN floor
advance", not "the request" alone but "the request the Update Agent sends".
A noun that needs the previous paragraph to be read is a noun missing a
word.

## Words this project reserves

Write "SVN floor", never a bare "floor", and "SVN floor advance", never
"floor advance". A bare "floor" reads as a building floor or a rate floor.
Identifiers and test names stay as they are, and inside a sentence that
already wrote "SVN floor" once, "the floor" may refer back to it.

Write "state machine", never a bare "machine", wherever the orchestrator
state machine is meant. A bare "machine" reads as the physical box.

Write "Update Agent" and "Firmware Device" with capital letters when naming
the DSP0267 roles. Lowercase reads as any agent or any device.

Write "result sentinel" for the `TEST_RESULT` line the QEMU runner greps
from the UART, never a bare "sentinel".

Use the word the specification uses. DSP0267 says request, so a protocol
message is a request, and an agent sends or makes one. Never write "ask"
for a message, as a noun or a verb: "the request for the SVN floor advance",
not "the agent's ask". The word "ask" stays only where a person asks a
question.

Use the MCUboot words for a trial boot. A test image runs unconfirmed, and
a successful boot confirms it. Never say an image "proves itself" or call a
boot "proven", because "prove" reads as a signature or attestation check,
which happens earlier.

"Arm" and "disarm" are watchdog words. For anything else, say what the code
writes: a revert clears the pending marker, it does not disarm a slot.

Never write "arc" for a run of transitions. Name the states and events the
run passes through, or write "sequence" or "path". "Arc" reads as
narrative, and no two readers guess the same endpoints.

Do not coin a term that reads as something else. "Recovery build" read as a
production build flavour when it meant a binary compiled with the `recovery`
cfg.

## What not to write

No em dashes. Use commas, colons, parentheses or a plain hyphen.

No genitive, meaning any apostrophe-s: not "the device's own", "the
machine's decision", "the component's update". Write the noun as the actor
("the state machine decides") or restructure ("the trial the device holds",
"an update for another component").

No bold-led bullets and no "Contract" headings. Fold the contract into one
or two prose paragraphs.

No aphorisms and no rhetorical parallelism. If a sentence sounds quotable,
rewrite it. The same goes for "deliberately", "precisely" and "nothing more
than".

Nothing editorial, meaning a verdict with no mechanism behind it. "The eRoT
is the one that cannot lose anti-rollback" adds nothing to "the ROM compares
the SVN before booting, so an image without one cannot be installed". Name
the mechanism or cut the sentence.

No "leaves it alone" or "left where it is". Write the operation that did not
happen: the clear does not run, the field stays set, the SVN floor keeps the
SVN it has.

No numbers spelled out in words. Write the literal, or cite the constant
(`CANDIDATE.len()`) where the number would otherwise go stale.

No counts in prose that a list or table below already holds. "Sixteen
scenarios" and "the eight negatives" go stale on the next commit and have to
be re-derived on every rebase. Write "each scenario", "the negatives", or
name the one case that matters.

No rationale by reference. State the reason in place, in one sentence,
rather than pointing at a design document for it.

## Prefer the concrete case

Name the scenario or the effect that reaches a branch. "The channel broke"
beats "no answer was received". Write from the side the file implements: the
app for the RoT says what the RoT did, not what happened to the device.

Two examples from review, to calibrate:

    No:  A refused security revision commit is not one.
    Yes: The verdicts read `refused` as "the RoT rejected the update", and
         the security revision commit is a separate decision, taken after
         the image is installed.

    No:  `None` if no answer was received.
    Yes: `None` means the channel broke: nothing arrived before the
         deadline, which `ipc_failed` records.

## Commit messages

The subject says what changed. The body says what changed and the one
non-obvious reason for it, and nothing else.

## Line breaks

Settle every word change first, then fix the line breaks. Re-wrap only the
comment blocks the patch already touches, to the width the file uses, and
never re-wrap a block whose words did not change. A diff full of reflowed
comments buries the change inside it.

## What these rules do not cover

Long descriptive identifiers and test names are good. The brevity rules
apply to prose, not to names.
