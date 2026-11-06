# SMC DMA IRQ Test (EVB-only)

This package contains the SMC DMA IRQ test flow split out from the parent SMC package.

## Targets

- Image: `//tests/smc/dma_irq:smc_irq_test`
- Runnable EVB test: `//tests/smc/dma_irq:smc_irq_evb_test`
- Panic detector: `//tests/smc/dma_irq:no_panics_test`

## Run on AST1060 EVB

```bash
AST1060_EVB_PI_HOST=<pi-hostname-or-ip> \
  bazel test --config=k_ast1060_evb \
  //tests/smc/dma_irq:smc_irq_evb_test
```
