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

## Notes

- `bazel build //target/ast10x0/...` builds all targets under the AST10x0 tree.
- `bazel test //target/ast10x0/...` builds the AST10x0 test targets and any
  required dependencies, but skips bare-metal test execution.
- `bazel test --config=virt_ast10x0 //target/ast10x0/...` executes the AST10x0
  system-image tests under QEMU.