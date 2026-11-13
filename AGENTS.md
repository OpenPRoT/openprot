# AGENTS.md

Guidance for AI coding agents working in the OpenPRoT repository.

If `docs/src/`, `workflows.json` or a README already states something, this file gives the path to it instead of repeating it. The exception is the CI gate commands (`./pw presubmit` and the wildcard tag filters), which this file writes out. A PR that puts the same statement in a doc and in this file removes the copy in this file.

## Build, test, and lint

```bash
bazel build --build_tag_filters=-hardware,-disabled -- //... -//third_party/caliptra/...            # practical wildcard build
bazel build --build_tag_filters=-hardware,-disabled,-verilator -- //... -//third_party/caliptra/...
```

Wildcard builds need the tag filters: `//...` includes hardware-only and verilator targets that fail on dev machines.

## Before pushing

- Run `./pw presubmit`, the CI gate, before every commit.
- Run `bazel test` on the targets the change touches. For wildcard test runs, see `docs/src/usage.md`.
- Rebase on main. The main ruleset requires linear history.

## Pointers

- Repository layout, placement rules, targets, images, emulator configs, and Pigweed and crate pinning: `docs/src/architecture.md` and the per-directory `README.md` files.
- Coding style (Rust, Python, Starlark, C/C++): `docs/src/coding-style.md`.
- Review rules and forbidden patterns: `.github/copilot-instructions.md`.
- Contribution process, RFCs, and CLA: `docs/src/contributing.md` and `docs/src/development-process.md`.
- Pigweed integration: `docs/src/design/pigweed-overview.md`.
- `pw_kernel` IPC: `docs/src/design/pw-kernel-ipc.md`; example in `target/veer/ipc/`.
- Orchestrator state machine and platform contracts: `docs/src/design/orchestrator/`.
- Specification: `docs/src/specification/`.
- Other `./pw` groups, direct Bazel usage and clippy: `docs/src/usage.md`. Docs build and serve: `docs/src/getting-started.md`.
