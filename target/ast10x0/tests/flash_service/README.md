# flash_service

Tests the flash service over kernel IPC on the AST10x0. A flash server process
owns the FMC and serves its CS1 flash on one channel through
`services/flash/server-runtime`. A flash client process reaches it through
`services/flash/client-ipc`. This is the AST10x0 counterpart of
`target/veer/tests/flash_service`. The `smc/*` tests drive the controller
directly in one task and do not cover the IPC path.

The client takes the sector size and flash size the server reports, which the
driver discovers over SFDP, and checks that they are consistent instead of
comparing them with fixed values. The service does not report the program
page, so the program that crosses a page boundary assumes 256 bytes.

Then it erases, programs (across a program page boundary) and reads back one
sector at 1 MiB. It also checks that a wrong-size erase, an unaligned erase and a read past the end are rejected.
It backs the sector up first and restores it at the end, even if a check
failed, so it leaves the flash as it found it on the EVB.

The kernel target applies the FMC pinmux before it starts any process, so no
task maps the SCU.

## Running

QEMU seeds erased 64 MiB images on both chip selects and models a
`w25q512jv`:

```
bazel test --config=virt_ast10x0 //target/ast10x0/tests/flash_service:flash_qemu_test
```

EVB hardware, with the W25Q512 on CS1. This runs on the AST1060 EVB through
the Pi fixture, so `AST1060_EVB_PI_HOST` must name the Pi host. Without it the
test fails:

```
AST1060_EVB_PI_HOST=<pi_host> bazel test --config=k_ast1060_evb //target/ast10x0/tests/flash_service:flash_evb_test
```

See [`../README.md`](../README.md) for the fixture setup.
