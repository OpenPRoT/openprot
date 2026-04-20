# Contributing to OpenPRoT

Thank you for your interest in contributing to the OpenPRoT project!

## Contribution Process

Before you begin, please review our [Development Process](development-process.md) for details on our **RFC process** for large changes and our standard requirements for pull requests.

## Contributor License Agreement

- Use of OpenPRoT requires no CLA.
- Contributions to OpenPRoT must have signed the [CHIPS CLA](https://github.com/chipsalliance/Caliptra/blob/main/CONTRIBUTING.md#contributor-license-agreement).

## Code of Conduct

The code of conduct can be found [here](https://github.com/OpenPRoT/.github/blob/main/CODE_OF_CONDUCT.md).

## Development Setup

1. Clone the repository.
2. Use the Pigweed workflow launcher `pw` or `bazel` for common tasks:
   - `./pw presubmit` - Run presubmit checks: formatting, license checks, C/C++ header checks, and `clippy`.
   - `./pw format` - Run the code formatters.
   - `./pw ci` - Build and run the host test suite.
   - `./pw default` - Build everything buildable on a dev host without running tests.
   - `bazel build //docs` - Build documentation.

Firmware builds are driven per target platform. See the target's `defs.bzl`
for the Bazel flags and constraints used when building firmware for it.

## Code Style

- Follow the [coding style](coding-style.md) guide.
- Run `./pw presubmit` to check for lints and ensure all tests pass.

## Documentation

- Update documentation in the `docs/` directory.
- Build docs with `bazel build //docs`.
- Documentation is built with mdbook.

## Issues

Please report issues on the GitHub issue tracker.
