# I2C Slave RX Burst Test

This test reproduces, in isolation, the I2C receive-drop defect first found in
the PLDM firmware-update flow: `services/i2c/server-runtime` keeps only one
received frame per bus, so a frame that arrives before the previous one has
been drained is silently overwritten.

Two AST1060 EVBs are wired bus-to-bus on I2C Bus 2. Device A (master) writes
`BURST_LEN` sequence-tagged frames back-to-back with no delay between them.
Device B (slave) runs the same `i2c_server_runtime` code path used by the MCTP
transport and asserts the frames arrive with no gaps in sequence. Either board
prints `TEST_RESULT:PASS` or `TEST_RESULT:FAIL` on its own UART when done.

`bazel test` automates building both images and driving the hardware over SSH
to a Raspberry Pi test fixture. This document explains what that automation is
doing, so the same test can be run by hand — flashing both boards and watching
their UARTs yourself, with no Bazel invocation on the Pi at all.

## Hardware

- Two AST1060 EVBs wired I2C bus-to-bus (Bus 2 on each board)
- A Raspberry Pi connected to both boards via:
  - Two USB-to-UART adapters (`/dev/ttyUSB0` for device A, `/dev/ttyUSB1` for device B)
  - GPIO lines for SRST and FWSPICK on each board, per
    [`evb_config.toml`](../../../../harness/evb_config.toml)

## 1. Build the images

```
bazelisk build --config=k_ast1060_evb \
    //target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst:master \
    //target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst:i2c_slave_rx_burst
```

This produces, under `bazel-bin/target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/`:

- `master.bin` / `master.elf` — device A (the burst master)
- `i2c_slave_rx_burst.bin` / `i2c_slave_rx_burst.elf` — device B (the i2c_server + slave client)

The `.bin` is what gets uploaded to a board. The `.elf` is only read for its
`pw_tokenizer` tokens, so logs print as words instead of `$<base64>`.

## 2. Copy the images and the uploader onto the Pi

The Pi needs Python 3 and `pyserial` (`pip install pyserial` if not already
present), plus the files that actually talk to the hardware:

```
scp \
    bazel-bin/target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/master.bin \
    bazel-bin/target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/master.elf \
    bazel-bin/target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/i2c_slave_rx_burst.bin \
    bazel-bin/target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/i2c_slave_rx_burst.elf \
    target/ast10x0/harness/pi_test_runner.py \
    <pi-user>@<pi-hostname>:~/i2c_slave_rx_burst/
scp -r target/ast10x0/harness/pw_tokenizer <pi-user>@<pi-hostname>:~/i2c_slave_rx_burst/
```

`pw_tokenizer` is vendored alongside `pi_test_runner.py` so the Pi needs
nothing installed beyond `pyserial`.

## 3. Run it from the Pi

```
ssh <pi-user>@<pi-hostname>
cd ~/i2c_slave_rx_burst
python3 pi_test_runner.py /dev/ttyUSB0 master.bin \
    --srst-pin 23 --fwspick-pin 18 \
    --slave-firmware i2c_slave_rx_burst.bin \
    --slave-uart-device /dev/ttyUSB1 \
    --slave-srst-pin 25 --slave-fwspick-pin 24 \
    --baudrate 115200 \
    --elf master.elf --elf i2c_slave_rx_burst.elf
```

The pin numbers and serial ports above are the defaults in
[`evb_config.toml`](../../../../harness/evb_config.toml); use your Pi's actual
values if its wiring differs.

Passing `--slave-firmware` puts `pi_test_runner.py` into paired two-device
mode (the same mode `system_image_test`'s `slave_image` attribute drives). It:

1. Flashes device A first (SRST low, drain stale UART bytes, FWSPICK high,
   SRST high; wait for the bootloader's ready byte; upload `master.bin`) and
   starts streaming its UART. Device A is flashed first so it is already
   running — and already past its `ARM_DELAY_SPINS` wait for device B to
   boot — rather than joining late and missing the start of the burst.
2. Flashes device B the same way with `i2c_slave_rx_burst.bin`, and starts
   streaming its UART.
3. Streams both boards' UART output, interleaved, detokenized against the
   `.elf`s passed to `--elf`, until each board emits `TEST_RESULT:PASS` or
   `TEST_RESULT:FAIL` (or either panics).

The process exits 0 only if both boards reported PASS.

## What `bazel test` does instead

```
bazelisk test --config=k_ast1060_evb \
    --test_env=AST1060_EVB_PI_HOST=<pi-hostname> \
    //target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst:i2c_slave_rx_burst_test
```

runs the same steps above automatically:

1. Bazel builds `:master` and `:i2c_slave_rx_burst` and symlinks their
   `.bin`/`.elf` outputs next to the test binary.
2. `test_runner.py`, on your workstation, acquires an exclusive lock on the Pi
   (`/tmp/ast1060_evb.lock`) so concurrent runs don't collide, then SCPs the
   firmware and `pi_test_runner.py` over.
3. It invokes `pi_test_runner.py` on the Pi over SSH with the flags shown
   above (derived from `evb_config.toml`) and streams its output back,
   detokenizing on the host side.

Everything in section 3 above is what step 3 here is doing on your behalf.
