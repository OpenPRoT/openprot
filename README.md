# OpenProt



## Getting Started

This project uses [cargo-xtask](https://github.com/matklad/cargo-xtask) for build automation and project management.

### Available Tasks

You can run tasks using `cargo xtask <task-name>`:

- `cargo xtask build` - Build the project
- `cargo xtask test` - Run all tests
- `cargo xtask check` - Run cargo check
- `cargo xtask clippy` - Run clippy lints
- `cargo xtask fmt` - Format code with rustfmt
- `cargo xtask clean` - Clean build artifacts
- `cargo xtask dist` - Build a distribution (release build)
- `cargo xtask docs` - Build documentation with mdbook
- `cargo xtask cargo-lock` - Manage Cargo.lock file
- `cargo xtask precheckin` - Run all pre-checkin validation checks
- `cargo xtask header-check` - Check license headers in source files
- `cargo xtask header-fix` - Fix missing license headers in source files

### Examples

```bash
# Build the project
cargo xtask build

# Run tests
cargo xtask test

# Create a distribution
cargo xtask dist

# Format code
cargo xtask fmt

# Run clippy
cargo xtask clippy

# Build documentation
cargo xtask docs

# Run all pre-checkin validation checks
cargo xtask precheckin

# Check license headers
cargo xtask header-check

# Fix missing license headers
cargo xtask header-fix
```

### Development

The project is structured as a Cargo workspace with two main components:

- `openprot/` - The main application
- `xtask/` - Build automation scripts

The xtask workflow allows you to add custom build steps, automation, and project management tasks written in Rust, making them cross-platform and easy to maintain.

## Requirements

- Rust 1.70+ (2021 edition)
- Cargo

No additional tools are required - everything is handled through Cargo and the xtask scripts.
