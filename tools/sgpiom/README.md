# SGPIOM JSON Tooling

This directory contains AST10x0 SGPIOM configuration tooling that replaces DTS-style wiring with a declarative JSON pipeline.

## What This Builds

The tool consumes board manifests and produces deterministic build artifacts:

- `sgpiom_merged.json`: canonical merged manifest
- `sgpiom_config_generated.rs`: generated Rust config module
- `sgpiom_report.txt`: summary report (optional)
- check/validate stamp outputs for CI gates

Generated Rust symbols include:

- `SGPIOM_CONTROLLER`
- `SGPIOM_BANKS`
- `SGPIOM_SIGNALS`
- `SGPIOM_MANIFEST_HASH`

## CLI

Tool entrypoint:

- `tools/sgpiom/sgpio_json_tool.py`

Subcommands:

- `validate`: schema and semantic checks
- `merge`: deterministic merge of input manifests
- `generate`: emit Rust static config module
- `check`: verify generated output is up to date
- `report`: print human-readable usage summary

## Example Inputs

- `tools/sgpiom/examples/common.json`
- `tools/sgpiom/examples/ast1060_dcscm.json`
- `tools/sgpiom/examples/ast1060_prot_dice.json`

## Local Usage

Validate:

```bash
python3 tools/sgpiom/sgpio_json_tool.py validate \
  --input tools/sgpiom/examples/common.json \
  --input tools/sgpiom/examples/ast1060_dcscm.json
```

Merge:

```bash
python3 tools/sgpiom/sgpio_json_tool.py merge \
  --input tools/sgpiom/examples/common.json \
  --input tools/sgpiom/examples/ast1060_dcscm.json \
  --output /tmp/sgpiom_merged.json
```

Generate:

```bash
python3 tools/sgpiom/sgpio_json_tool.py generate \
  --input tools/sgpiom/examples/common.json \
  --input tools/sgpiom/examples/ast1060_dcscm.json \
  --output /tmp/sgpiom_config_generated.rs
```

Check:

```bash
python3 tools/sgpiom/sgpio_json_tool.py check \
  --input tools/sgpiom/examples/common.json \
  --input tools/sgpiom/examples/ast1060_dcscm.json \
  --output /tmp/sgpiom_config_generated.rs
```

Report:

```bash
python3 tools/sgpiom/sgpio_json_tool.py report \
  --input tools/sgpiom/examples/common.json \
  --input tools/sgpiom/examples/ast1060_dcscm.json
```

## Bazel Integration

Bazel tool package:

- `//tools/sgpiom`

Board-level SGPIOM pipeline targets:

- `//target/ast10x0/board:sgpiom_validate`
- `//target/ast10x0/board:sgpiom_merged`
- `//target/ast10x0/board:sgpiom_generate`
- `//target/ast10x0/board:sgpiom_check`
- `//target/ast10x0/board:sgpiom_report`

Board crate consumption target:

- `//target/ast10x0/board:ast10x0_board`

Build all SGPIOM pipeline targets:

```bash
bazelisk build --config=k_ast1060_evb \
  //target/ast10x0/board:ast10x0_board \
  //target/ast10x0/board:sgpiom_validate \
  //target/ast10x0/board:sgpiom_merged \
  //target/ast10x0/board:sgpiom_generate \
  //target/ast10x0/board:sgpiom_check \
  //target/ast10x0/board:sgpiom_report
```

## Notes

- Board crate selects configuration; peripherals crate consumes typed config.
- Keep manifests declarative and deterministic to avoid runtime parser complexity.
