# SMC Read Test (EVB-only)

This package contains the SMC read path test flow split out from the parent SMC package.

## Targets

- Image: `//target/ast10x0/tests/smc/read:smc_read_test`
- Runnable EVB test: `//target/ast10x0/tests/smc/read:smc_read_evb_test`
- Panic detector: `//target/ast10x0/tests/smc/read:no_panics_test`

## Run on AST1060 EVB

```bash
AST1060_EVB_PI_HOST=<pi-hostname-or-ip> \
  bazel test --config=k_ast1060_evb \
  //target/ast10x0/tests/smc/read:smc_read_evb_test
```
