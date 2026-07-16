<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# AST10x0 GPIO Interrupt Configuration Test

This hardware-only test follows the SGPIOM IRQ test's wait-on-object model.

The kernel applies the GPIOA0 pin mux and starts one userspace app. The app:

1. Owns the GPIO register block at `0x7e780000`.
2. Configures GPIOA0 as a pulled-down input.
3. Registers GPIO IRQ 11 with a wait group.
4. Clears pending status and enables both-edge interrupts.
5. Acknowledges the kernel interrupt object.
6. Verifies GPIOA0 in the interrupt-enable and sensitivity-type-2 registers.

The test validates interrupt configuration without requiring an external edge.
The kernel emits `TEST_RESULT:PASS` or `TEST_RESULT:FAIL` at shutdown.

## Build

```sh
bazelisk build --config=k_ast1060_evb \
  //target/ast10x0/tests/peripherals/gpio/gpio_irq:gpio_irq
```
