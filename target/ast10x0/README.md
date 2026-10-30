# AST10x0 Pigweed Target

Pigweed kernel target for the AST10x0 platform.

## Building

Build all targets under the AST10x0 tree with:

```console
bazel build //target/ast10x0/...
```

Run the AST10x0 test targets with:

```console
bazel test //target/ast10x0/...
```

This builds the AST10x0 test targets and any required dependencies. Firmware-
backed tests are skipped unless a runner is configured.

## Running Tests Under QEMU

Run the full AST10x0 test suite under QEMU with:

```console
bazel test --config=virt_ast10x0 //target/ast10x0/...
```

The `virt_ast10x0` config launches images under QEMU (`ast1030-evb` machine)
using a local sentinel-based runner. Pass/fail is signalled by the firmware
writing `TEST_RESULT:PASS` or `TEST_RESULT:FAIL` to UART. See
`target/ast10x0/tests/README.md` for details.

For more detailed failures:

```console
bazel test --config=virt_ast10x0 --verbose_failures //target/ast10x0/...
```

## Running Tests On Hardware

Run the AST10x0 tests on a physical AST1060 EVB attached to a Raspberry Pi
test fixture with:

```console
AST1060_EVB_PI_HOST=<user>@<pi-host> bazel test --config=k_ast1060_evb //target/ast10x0/tests/...
```

Key-based SSH auth to the Pi is required. See `target/ast10x0/tests/README.md`
for the fixture details.

### Flaky Test Assessment

To check the hardware tests for flakiness, run each test multiple times with
result caching disabled:

```console
AST1060_EVB_PI_HOST=<user>@<pi-host> bazel test \
  --runs_per_test=2 \
  --runs_per_test_detects_flakes \
  --cache_test_results=no \
  --keep_going \
  --test_output=errors \
  --config=k_ast1060_evb \
  //target/ast10x0/tests/...
```

- `--runs_per_test=2` runs every test twice (increase for a stricter check).
- `--runs_per_test_detects_flakes` reports a test that passes in some runs and
  fails in others as `FLAKY` instead of `FAILED`.
- `--cache_test_results=no` forces real runs on the board.
- `--keep_going` continues past failures so the whole suite is assessed.
- `--test_output=errors` prints the log of failing runs.

## Notes

- `bazel build //target/ast10x0/...` builds all targets under the AST10x0 tree.
- `bazel test //target/ast10x0/...` builds the AST10x0 test targets and any
  required dependencies, but skips bare-metal test execution.
- `bazel test --config=virt_ast10x0 //target/ast10x0/...` executes the AST10x0
  system-image tests under QEMU.