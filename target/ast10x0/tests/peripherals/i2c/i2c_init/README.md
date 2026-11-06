# i2c_init — AST10x0 I2C controller init smoke test

Hardware smoke test that instantiates the AST1060 I2C1 controller in each
supported timing mode and verifies the resulting register state matches the
expected timing parameters.

## What it covers

For each speed mode below, the test calls the matching `Ast1060I2c`
constructor, then reads back `i2cc00`, `i2cc04`, and `i2cm10` to verify
master enable, bus auto-release, multi-master configuration, base clock
divider, SCL low/high pulse widths, timeout settings, and SMBus alert
enables:

- Standard mode (`I2cSpeed::Standard`, PIO transfer)
- Fast mode (`I2cSpeed::Fast`, PIO transfer)
- Fast-plus mode (`I2cSpeed::FastPlus`, PIO transfer)
- DMA-fast mode (`I2cSpeed::Fast` via `new_with_dma`)

Board-level I2C global init (`Ast10x0Board`) is run once up front so pinmux
and clocks are valid before any controller is instantiated.

Success is signalled over UART with `TEST_RESULT:PASS`.

## Prerequisites

- AST1060 EVB connected to the lab Raspberry Pi fixture.
- SSH access to the Pi host.
- `AST1060_EVB_PI_HOST` environment variable set to the Pi's hostname or IP
  so the bazel harness can reach the fixture.

## Run on hardware

```bash
AST1060_EVB_PI_HOST=<pi-hostname-or-ip> \
  bazel test --config=k_ast1060_evb \
    --nocache_test_results \
    --test_output=streamed \
    --test_timeout=300 \
    --curses=no --noshow_progress \
    //tests/peripherals/i2c/i2c_init:i2c_init_test
```

Notes:

- `--nocache_test_results` forces a re-run against hardware; without it
  bazel will report `PASSED (cached)` and never touch the board.
- `--curses=no --noshow_progress` keeps streamed UART output from being
  overwritten by bazel's progress UI in some terminals.
- If streamed output is silent, the full log is still captured at:
  `bazel-testlogs/tests/peripherals/i2c/i2c_init/i2c_init_test/test.log`

## Expected output (excerpt)

```
[INF] === AST10x0 I2C init smoke test ===
[INF] Board-level I2C global init complete
[INF] Instantiating controller 1 in standard mode
[INF] standard mode init+verify passed
[INF] Instantiating controller 1 in fast mode
[INF] fast mode init+verify passed
[INF] Instantiating controller 1 in fast-plus mode
[INF] fast-plus mode init+verify passed
[INF] Instantiating controller 1 in dma-fast mode (new_with_dma)
[INF] dma-fast mode init+verify passed
[INF] === AST10x0 I2C init smoke test complete ===
TEST_RESULT:PASS
```

## Files

- `target.rs` — test entry point (`TargetInterface` implementation).
- `BUILD.bazel` — `system_image` + `system_image_test` rules.
- `system.json5` — kernel/arch memory layout for the test image.
