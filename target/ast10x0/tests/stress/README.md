# AST10x0 Stress Tests

Stress tests run forever and only exit on `TEST_RESULT:FAIL`. They are excluded
from CI and must be run manually using one of the configs below.

## Tests

| Target | What it stresses |
|---|---|
| `stress/mutex/kernel:mutex_stress_test` | Mutex contention across three kernel threads |
| `stress/ipc/user:ipc_stress_test` | IPC channel between initiator and handler processes |
| `stress/process_termination/user:process_termination_stress_test` | Repeated process termination and restart |

## Running on QEMU

```
# Open-ended soak — runs until Ctrl-C or FAIL, no timeout.
# Target the system_image directly, not the _stress_test rule.
bazel run --config=stress_virt_ast10x0 //target/ast10x0/tests/stress/mutex/kernel:mutex

# Timed run — Bazel kills after 3600s if no FAIL detected.
bazel test --config=stress_virt_ast10x0 //target/ast10x0/tests/stress/<test>:*_stress_test
```

Use `bazel run` on the `system_image` target (`:mutex`, `:ipc`, `:process_termination`)
for open-ended soak testing — this runs QEMU directly with no wrappers.
`bazel test` targets the `_stress_test` rule and is bounded by Bazel's 3600s ceiling.

## Running on hardware (EVB via Pi fixture)

```
AST1060_EVB_PI_HOST=<pi-hostname> bazel test --config=stress_k_ast1060_evb //target/ast10x0/tests/stress/<test>
```

`bazel run` is not supported for EVB — the Pi fixture is only wired into
`bazel test`. Tests run one at a time (`--local_test_jobs=1` is inherited from
`k_ast1060_evb`) and have no runner timeout.
