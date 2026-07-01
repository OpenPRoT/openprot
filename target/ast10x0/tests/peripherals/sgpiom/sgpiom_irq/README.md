<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# AST10x0 SGPIOM Interrupt Configuration Test (`sgpiom_irq`)

Real-hardware test image that exercises the SGPIOM interrupt configuration path
end-to-end through the OpenPRoT microkernel **wait-on-object** model — the same
model the EarlGrey GPIO driver documents, where interrupts are delivered to
userspace via syscalls rather than in-driver ISR callbacks.

This is **not** a QEMU test: `sgpiom_irq_test` is tagged `hardware` and is marked
incompatible when `qemu_enabled`. It must run on a real AST1060 board.

## What it does

A single userspace app (`sgpiom_irq_server`):

1. Owns the SGPIOM register block (device mapping `sgpiom_regs` @ `0x7e780500`).
2. Drives a static output pattern (`0x55`) on SGPIO_A pins 0–7.
3. Programs both-edge sensitivity on the same pins via the HAL
   `GpioInterrupt::irq_configure`.
4. Registers the SGPIOM interrupt object (**IRQ 51**, `INTR_SGPIOM`) with a wait
   group, then enables the pins via `GpioInterrupt::irq_control(Enable)` and
   unmasks the line at the kernel with `interrupt_ack`.
5. Reads back the `int_en` register and verifies it matches the watch mask.
6. Calls `debug_shutdown(Ok(()))` → **PASS** if the register check passes;
   `debug_shutdown(Err(_))` → **FAIL** on any init or config error.

No external signal source is required — the test validates IRQ configuration
correctness via register readback, not edge delivery.

Console sentinels: `TEST_RESULT:PASS` / `TEST_RESULT:FAIL` (emitted by the
kernel `Target::shutdown`).

## Pin mux

The kernel applies `pinctrl::PINCTRL_SGPIOM` at boot (SCU41C[8:11]):

| pin       | SCU41C bit | function          |
|-----------|------------|-------------------|
| `sgpmclk` | 8          | serial clock out  |
| `sgpmld`  | 9          | load/latch out    |
| `sgpmo`   | 10         | serial data out   |
| `sgpmi`   | 11         | serial data in    |

SGPIOM is clocked by PCLK (always running), so no clock ungate or controller
reset is performed.

## Build / run

```
bazel build //target/ast10x0/tests/peripherals/sgpiom/sgpiom_irq:sgpiom_irq
AST1060_EVB_PI_HOST=<pi-host> bazelisk test --config=k_ast1060_evb \
  //target/ast10x0/tests/peripherals/sgpiom/sgpiom_irq:sgpiom_irq_test
bazel test  //target/ast10x0/tests/peripherals/sgpiom/sgpiom_irq:no_panics_test
```

## Tunables (`sgpiom_irq_server_main.rs`)

| const        | meaning                                |
|--------------|----------------------------------------|
| `NGPIOS`     | total SGPIO count in the global config |
| `CLOCK_DIV`  | serial clock divider                   |
| `OUT_MASK`   | output pin mask                        |
| `OUT_PATTERN`| static output pattern driven on boot   |
| `WATCH_MASK` | which bank-A pins to arm for IRQ       |
