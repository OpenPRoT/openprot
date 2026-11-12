# flash

Tests the AST10x0 FMC CS1 flash through the flash HAL, without IPC. The test
runs in the kernel with `Ast10x0FmcFlashDriver` wrapped in `BlockingFlash`.
This is the AST10x0 counterpart of `target/veer/tests/flash`.
[`flash_service`](../flash_service/README.md) covers the same flash over IPC.

The test checks the geometry, then erases two sectors, programs data across a
program page boundary, checks that programming only clears bits, and checks
that an erase leaves the next sector unchanged. It also checks that a read
past the end and an unaligned erase are rejected. It saves both sectors, 1 MiB
into the flash, before it starts and restores them at the end, even if a check
failed, so it leaves the flash as it found it on the EVB.

## Running

QEMU seeds erased 64 MiB images on both chip selects and models a
`w25q512jv`:

```
bazel test --config=virt_ast10x0 //target/ast10x0/tests/flash:flash_qemu_test
```

EVB hardware, with the W25Q512 on CS1. This runs on the AST1060 EVB through
the Pi fixture, so `AST1060_EVB_PI_HOST` must name the Pi host. Without it the
test fails:

```
AST1060_EVB_PI_HOST=<pi_host> bazel test --config=k_ast1060_evb //target/ast10x0/tests/flash:flash_evb_test
```

See [`../README.md`](../README.md) for the fixture setup.
