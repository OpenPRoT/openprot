# Getting Started

## Prerequisites

- [Bazel](https://bazel.build/). We recommend installing [bazelisk](https://github.com/bazelbuild/bazelisk) to automatically manage bazel versions.

No additional tools are required - all dependencies are managed by bazel.

## Installation

Clone the repository:

```bash
git clone https://github.com/OpenPRoT/openprot
cd openprot
```

## Available Tasks

You can run common development tasks using the Pigweed workflow launcher `pw` or
`bazel` directly. `./pw` is a thin shell wrapper around
`bazelisk run //:pw -- "$@"`, dispatching to workflows defined in
`workflows.json`.

- `./pw presubmit` - Run presubmit checks: formatting, license checks, C/C++ header checks, and `clippy`.
- `./pw format` - Run the code formatters.
- `./pw ci` - Build and run the host test suite, excluding targets that require real hardware or a verilator sim.
- `./pw default` - Build everything buildable on a dev host without running tests.
- `./pw upstream_pigweed` - Full CI plus the Earlgrey verilator tests; used to validate upstream Pigweed bumps.
- `bazel build //docs` - Build the mdbook documentation.
- `bazel run //docs:serve` - Serve the mdbook on `http://localhost:8000` for local review.

## Next Steps

- Read the [Usage](./usage.md) guide.
- Check out the [Architecture](./architecture.md) documentation.
- Learn about [Contributing](./contributing.md).
