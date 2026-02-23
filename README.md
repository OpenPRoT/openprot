# OpenPRoT

## Technical Charter

The OpenPRoT Technical Charter can be found at
[<u>https://github.com/OpenPRoT/.github/blob/main/GOVERNANCE.md</u>](https://github.com/OpenPRoT/.github/blob/main/GOVERNANCE.md)

## Getting Started

NOTE: We are converting our build system to [bazel](https://bazel.build/).  We recommend installing [bazelisk](https://github.com/bazelbuild/bazelisk) to automatically manage bazel versions.

### Available Tasks


You can run tasks using the Pigweed workflow launcher `pw` or `bazel`.

- `./pw presubmit` - Run presubmit checks: formatting, license checks, C/C++ header checks and `clippy`.
- `./pw format` - Run the code formatters.
- `bazel test //...` - Run all tests.
- `bazel build //docs` - Build documentation.

### Development

The project is structured as a bazel module.

## Requirements

- [Bazel](https://bazel.build/).  We recommend installing [bazelisk](https://github.com/bazelbuild/bazelisk) to automatically manage bazel versions.

No additional tools are required - all dependencies are managed by bazel.
