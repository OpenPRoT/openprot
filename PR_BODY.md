Stacked on #574. #572 merged while this was being written, so the only commit
carried along is #574's glue; GitHub cannot take a fork branch as a base, so
this PR shows it as well. Review #574 first and this diff shrinks to the last
two commits.

## A refused security revision commit

`pldm_update/svn_commit` only ever runs the grant. `svn_commit_refused` runs
the other answer: the agent asks twice, which is what it does when the answer
to the first ask was lost. The floor took the SVN on the first ask and the
window closed with it, so the machine drops the second
`Event::SvnCommitRequest`, `svn_commit` sees no `Effect::CommitSvnFloor`, the
RoT sends RejectSvnCommit, and the device answers the agent
UPDATE_SECURITY_REVISION_NOT_PERMITTED. The agent checks for that code rather
than for any failure.

`svn_commit` returns a `Commit` instead of a `bool`, so the loop can tell a
refusal from an exchange that did not complete and carry on after the first
grant.

One thing this found: a refused commit is not a refused update. The device's
`await_decision` set the `refused` flag on any rejection, and `refused` means
"an update the RoT refused", so the scenario failed with "the revision was
committed and the update was not". `await_answer` is the same call without the
flag, for steps outside the update flow.

## Recovery, start to finish

Recovery had no QEMU run at all: every board wired `type Recovery = ()`, so
`State::Recovering`, `Effect::RecoverComponent` and the events that end an
episode were unit-tested and nothing more, and nothing dispatched
`Event::VerificationFailed`. This PR gives both a run.

`mock_downstream_device/recovery` starts with an image the verifier rejects.
The machine enters `Recovering`, the board writes its one recovery source, the
re-walk authenticates what was written, and the device is released and boots.
It cannot pass vacuously: the verifier refuses until the restore, so a device
that booted went through recovery to get there, and the restore count is an
exact match.

`recovery_exhausted` writes an image that is refused too. The second episode
asks for a source that is not there, the device is required, so the platform
reports it and locks. That lockdown lands inside the power-on dispatch, because
the whole arc is follow-ups of one event.

Because cfg flags do not reach the `board_stubs` crate, the recovery builds
wire a scenario-local verifier through a `ScenarioVerifier` type alias and keep
the shared stub everywhere else, the same shape as `DeviceRecovery`.

## A finding for the state machine, not for this PR

The recovery source is wired in the two recovery builds alone. With a source
behind every board, `device_hangs` stops meaning what it says: the device
verifies and simply never reports, and each re-walk's
`Event::VerificationPassed` calls `clear_retry`
(`services/orchestrator/sm/src/rot.rs:485`, INV7 "consecutive failed
restores"). The attempt handed to `restore` stays 0, so `MAX_RETRY` never
bites. The log showed four restores at attempt 0 and then the run budget
expiring: no lockdown and no boot, which for a required component is arguably
worse than locking.

Whether the retry count should survive a passing verification is a
machine-semantics call. It wants an issue of its own.

## Testing

CI is the first full run of this branch. The two commits were green on an
earlier base (`bazel test --config=virt_ast10x0
//target/ast10x0/tests/integration/...` at 34/34, `bazel test //services/...`
at 38/38, `./pw presubmit`), but they were rebased onto #574 with real conflict
resolution, and a disk quota on the machine they were rebased on stopped the
suite after 10 of 34. That is why this is a draft.

Also known: `svn_commit_refused` failed once in about twenty-five runs on the
earlier base. The agent's 0x22 landed in the gap between `run_terminus`
returning after the activation and the device going back in to serve idle
commands, and timed out. The agent does not retry, so nothing covers it over.
It is in GAPS.md rather than papered over.
