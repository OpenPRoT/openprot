# AST10x0 SMC Test Package

This package is now a compatibility layer and shared helper location.

## Current organization

Active SMC test flows are split into dedicated subpackages:

- `//target/ast10x0/tests/smc/read`
- `//target/ast10x0/tests/smc/dma_irq`

The parent package (`//target/ast10x0/tests/smc`) keeps only:

- Compatibility aliases:
  - `smc_read_test`
  - `smc_read_evb_test`
  - `smc_irq_test`
  - `smc_irq_evb_test`
- Shared debug helper export: `target_debug.rs`

## Canonical targets

Use these subpackage labels as canonical:

- Read flow image: `//target/ast10x0/tests/smc/read:smc_read_test`
- Read flow runnable EVB test: `//target/ast10x0/tests/smc/read:smc_read_evb_test`
- DMA IRQ flow image: `//target/ast10x0/tests/smc/dma_irq:smc_irq_test`
- DMA IRQ flow runnable EVB test: `//target/ast10x0/tests/smc/dma_irq:smc_irq_evb_test`

Legacy compatibility aliases in this package resolve to the same targets.

## Run on AST1060 EVB

Read flow:

```bash
AST1060_EVB_PI_HOST=<pi-hostname-or-ip> \
  bazel test --config=k_ast1060_evb \
  //target/ast10x0/tests/smc/read:smc_read_evb_test
```

DMA IRQ flow:

```bash
AST1060_EVB_PI_HOST=<pi-hostname-or-ip> \
  bazel test --config=k_ast1060_evb \
  //target/ast10x0/tests/smc/dma_irq:smc_irq_evb_test
```

Both flows are hardware-only by design.