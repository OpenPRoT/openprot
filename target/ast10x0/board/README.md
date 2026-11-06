# ast10x0_board

Board-level integration crate for AST10x0 platforms.

This crate owns board-level hardware initialization and board-selected SGPIOM
configuration for AST10x0 targets.

## Responsibilities

- Apply board pinctrl groups via SCU
- Gate/reset board-level peripherals needed at boot (currently I2C flow)
- Export board descriptor metadata used by runtime init
- Consume generated SGPIOM configuration selected at board build time

SGPIOM policy is board-owned here, while SGPIOM execution remains in
`//peripherals`.

## SGPIOM JSON Pipeline (Bazel)

The board package wires SGPIOM JSON tooling into Bazel with these targets:

- `//board:sgpiom_validate`
- `//board:sgpiom_merged`
- `//board:sgpiom_generate`
- `//board:sgpiom_check`
- `//board:sgpiom_report`

Generated artifact compiled into this crate:

- `sgpiom_config_generated.rs`

See `//tools/sgpiom/README.md` for CLI details and manifest format.

## Build

```
bazelisk build --config=virt_ast10x0 //board:ast10x0_board
```

For AST1060 hardware config and SGPIOM pipeline artifacts:

```bash
bazelisk build --config=k_ast1060_evb \
	//board:ast10x0_board \
	//board:sgpiom_validate \
	//board:sgpiom_merged \
	//board:sgpiom_generate \
	//board:sgpiom_check \
	//board:sgpiom_report
```
