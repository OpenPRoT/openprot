# Contributing to OpenPRoT

## Contributor License Agreement

- Use of OpenPRoT requires no CLA.
- Contributions to OpenPRoT must have signed the [CHIPS CLA](https://github.com/chipsalliance/Caliptra/blob/main/CONTRIBUTING.md#contributor-license-agreement).

## Code of Conduct

The code of conduct can be found [here](https://github.com/OpenPRoT/.github/blob/main/CODE_OF_CONDUCT.md).

## Development Setup

1. Clone the repository
2. Install dependencies: `cargo xtask check`
3. Run tests: `cargo xtask test`
4. Format code: `cargo xtask fmt`

## Code Style

### Formatters

The following formatters are used in the project:

- `rustfmt`: For Rust code formatting
- `PEP8` and `black` for Python code formatting
- `clang-format` for C code formatting
- `buildifier` for Bazel code formatting

## Documentation

- Update documentation in the `docs/` directory
- Build docs with `cargo xtask docs`
- Documentation is built with mdbook

## Pull Requests

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run the full test suite
5. Submit a pull request

## Issues

Please report issues on the GitHub issue tracker.
