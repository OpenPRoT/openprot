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

You can run common development tasks using the Pigweed workflow launcher `pw` or `bazel`:

- `./pw presubmit` - Run presubmit checks: formatting, license checks, C/C++ header checks, and `clippy`.
- `./pw format` - Run the code formatters.
- `bazel test //...` - Run all tests.
- `bazel build //docs` - Build documentation.

## Next Steps

- Read the [Usage](./usage.md) guide.
- Check out the [Architecture](./architecture.md) documentation.
- Learn about [Contributing](./contributing.md).
