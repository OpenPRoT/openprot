# Timer QEMU test

This test runs on an emulated AST10x0 (under QEMU) and checks that the
orchestrator's timer bookkeeping, [`TimerManager`](../../../../../services/orchestrator/timer/src/lib.rs),
works with the real system clock and the kernel's wait call.

The timer *logic* is already proven by host-based unit tests: ordering,
tie-breaks, one-shot firing, resetting a timer, and cancelling one are all
asserted there against a fake clock. This test does not re-prove that logic. Its
job is the part the unit tests can't reach — the integration with real hardware
timing:

- real time actually elapsing, and the `Instant + Duration` math that goes with
  it, instead of a hand-advanced counter;
- the kernel's wait call really returning "deadline passed" when a deadline
  lapses and "signal arrived" when an interrupt fires — the unit tests never call
  it.

The scenarios below mirror the unit tests on purpose, so the same behavior is
confirmed once the real clock and the real wait call are in the loop.

## The idea being tested

The orchestrator watches each component with a timer. It sets a deadline, then
waits for either the component to signal progress or the deadline to pass:

```text
1. Ask TimerManager for the next deadline.
2. Wait for either a signal or that deadline.
3. If the deadline passed first  -> a timer expired (the component was too slow).
   If a signal arrived first      -> the component reported in; cancel its timer.
```

This test recreates that wait loop and confirms each part behaves.

## How the test is set up

- Firmware image running on the chip.
- A fake "component signalled progress" event, produced by triggering interrupt
  number 44 from within the test itself.
- Deadlines come from `TimerManager`; the time comes from the system clock.
- When done, the program shuts the machine down and prints `TEST_RESULT:PASS`
  (or `FAIL`).

## What each scenario checks

1. **Timers expire in the right order.** Three timers are set (two component
   timers and one commit timer, two of them sharing the same deadline). The test
   confirms they expire in the expected order, none expire early, and each fires
   only once.
2. **Resetting a timer replaces the old one.** A timer is set for 30 ms, then
   immediately reset to 80 ms. The test confirms only the 80 ms deadline is left
   — the old 30 ms one is gone, not stacked on top.
3. **A signal cancels the timer.** A timer is set for 500 ms, but the progress
   signal is sent first. The test confirms the wait returns because of the
   signal (not the deadline), the timer is cancelled, and nothing is left
   pending.

## Files

- `main.rs` — the three scenarios and the program's entry point.
- `system.json5` — how the program, its memory, and the interrupt are laid out.
- `target.rs` — prints the final `TEST_RESULT:PASS/FAIL` line.
- `BUILD.bazel` — how the test image is built and run.

## Running it

```sh
bazelisk test --config=virt_ast10x0 \
  //target/ast10x0/tests/orchestrator/timer:timer_test --test_output=all
```

Other build targets in this folder:

- `:timer` — the bootable image.
- `:timer_test` — runs that image in QEMU (the command above).
- `:no_panics_test` — checks the image contains no panic code paths.
