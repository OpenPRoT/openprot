<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# async_sim: an orchestrator that never blocks on a service

A QEMU simulation of the real `orchestrator-sm` core talking to several services
at once, without ever blocking on one. It checks that the orchestrator keeps
reacting to boot watchdogs and other services while a slow call is in flight,
that a call that outlives its deadline is abandoned and recovered, and that a
device that never boots makes the machine give up.

## Apps

Three apps in one image:

- **orchestrator**: the system under test. The real state machine, plus an
  executor that turns each effect into a command on a `util_requester` lane and
  each response back into an event. One thread, one WaitGroup, no blocking call.
- **verifier**: a `util_service::ServiceLoop` server with one channel. It defers
  each reply for a latency of its own and completes it from the loop's deadline.
- **devices**: a `ServiceLoop` server with two channels, `reset` and `events`. Each
  device is an `openprot_device_model::DeviceModel` with a boot script. `reset`
  releases or holds a device; `events` is a long-poll that is answered when a
  device boots.

The services decide how they behave. A device's boot script and each
component's verify latency are fixed when the image is built, and the
orchestrator never says how a service should act. It can only drive reset, name
the component to verify, and listen. So a bug in the orchestrator (a double
release, say) cannot quietly change the script it is judged against.

## Scenarios

One image runs one scenario, chosen by a build flag, so each script is fixed at
build time and a failure names its scenario.

| Scenario | The world | Passes when |
|---|---|---|
| `flaky_boot` | C0 does not boot on its first release; C1's first verify is slow | C0's boot watchdog fires while C1 is still verifying, the core recovers C0, C1's late verdict is dropped as stale, and the re-walk reaches `Ready` |
| `verify_hangs` | C2's first verify outlives the orchestrator's deadline | the executor cancels it and reports a failed verification, the verifier's late reply is dropped, and the re-walk reaches `Ready` |
| `never_boots` | C0 never boots, on any release | the machine gives up after its retry limit (three recoveries), reports recovery failed and locks |

## Running

```sh
bazel test --config=virt_ast10x0 //target/ast10x0/tests/orchestrator/async_sim/...
bazel test --config=virt_ast10x0 --test_output=all \
    //target/ast10x0/tests/orchestrator/async_sim/flaky_boot:flaky_boot_qemu_test
```

Repeat a run with `--runs_per_test`, not a loop inside the image: the scripts
count from the start of the image. Each image also has a `no_panics_test`.

## Adding a scenario

1. Add a `--cfg` name to `SCENARIO_CFGS` in `scenario.bzl`.
2. Add a package (`my_scenario/BUILD.bazel`) that calls `qemu_scenario` with that
   flag. The system config names each app by its target name, so a scenario is a
   package of its own.
3. Give the services their script for it: `C0_SCRIPT` in `devices_main.rs`, and
   `LATENCY_MS` in `verifier_main.rs`.
4. Add the scenario function and its checks to `orchestrator_main.rs`.

## Timing

The scenarios race tens of milliseconds against QEMU's wall clock, so the margins
matter. Each gap that has to hold is written next to the numbers:

- `BOOT_WINDOW` and `VERIFY_DEADLINE` in `orchestrator_main.rs`, and the boot
  and verify times in `devices_main.rs` and `verifier_main.rs`, form a chain
  (boot < watchdog < slow verify < deadline). Move one and the others move with it.
- `verify_hangs` needs the hang to sit between the deadline and twice the
  deadline. Widening only one side breaks the other.
- The service threads run at a higher priority than the orchestrator, so a
  service reacts as soon as a request or its timer is ready.
- The tests are `exclusive`: beside other QEMU runs the guest slows down and the
  margins go.
