// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use xshell::{cmd, Shell};

type DynError = Box<dyn std::error::Error>;

pub(crate) fn docs() -> Result<(), DynError> {
    check_mdbook()?;
    check_mermaid()?;

    println!("Running: mdbook");
    let sh = Shell::new()?;
    let project_root = project_root();
    let docs_dir = project_root.join("docs");
    let dest_dir = project_root.join("target/book");

    // Create docs directory if it doesn't exist
    if !docs_dir.exists() {
        create_default_docs_structure(&docs_dir)?;
    }

    sh.change_dir(&docs_dir);
    cmd!(sh, "mdbook build --dest-dir {dest_dir}").run()?;

    println!(
        "Docs built successfully: view at {}/index.html",
        dest_dir.display()
    );

    Ok(())
}

fn check_mdbook() -> Result<(), DynError> {
    let status = Command::new("mdbook")
        .args(["--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_ok() {
        return Ok(());
    }

    println!("mdbook not found; installing...");
    let sh = Shell::new()?;
    cmd!(sh, "cargo install mdbook").run()?;

    Ok(())
}

fn check_mermaid() -> Result<(), DynError> {
    let status = Command::new("mdbook-mermaid")
        .args(["--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_ok() {
        return Ok(());
    }

    println!("mdbook-mermaid not found; installing...");
    let sh = Shell::new()?;
    cmd!(sh, "cargo install mdbook-mermaid").run()?;

    Ok(())
}

fn create_default_docs_structure(docs_dir: &Path) -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.create_dir(docs_dir)?;

    // Create book.toml
    let book_toml = docs_dir.join("book.toml");
    sh.write_file(
        &book_toml,
        r#"[book]
authors = ["OpenProt Team"]
language = "en"
multilingual = false
src = "src"
title = "OpenProt Documentation"

[preprocessor.mermaid]
command = "mdbook-mermaid"

[output.html]
"#,
    )?;

    // Create src directory and SUMMARY.md
    let src_dir = docs_dir.join("src");
    sh.create_dir(&src_dir)?;

    let summary_md = src_dir.join("SUMMARY.md");
    sh.write_file(
        &summary_md,
        r#"# Summary

[Introduction](./introduction.md)

# User Guide

- [Getting Started](./getting-started.md)
- [Usage](./usage.md)

# Developer Guide

- [Architecture](./architecture.md)
- [Contributing](./contributing.md)
"#,
    )?;

    // Create introduction.md
    let intro_md = src_dir.join("introduction.md");
    sh.write_file(&intro_md, r#"# Introduction

Welcome to the OpenProt documentation!

This documentation provides comprehensive information about the OpenProt project, including user guides, developer documentation, and API references.

## What is OpenProt?

OpenProt is a Rust-based project that provides...

## Quick Start

To get started with OpenProt:

```bash
cargo xtask build
cargo xtask test
```

For more detailed instructions, see the [Getting Started](./getting-started.md) guide.
"#)?;

    // Create getting-started.md
    let getting_started_md = src_dir.join("getting-started.md");
    sh.write_file(
        &getting_started_md,
        r#"# Getting Started

## Prerequisites

- Rust 1.70 or later
- Cargo

## Installation

Clone the repository:

```bash
git clone <repository-url>
cd openprot
```

Build the project:

```bash
cargo xtask build
```

Run tests:

```bash
cargo xtask test
```

## Next Steps

- Read the [Usage](./usage.md) guide
- Check out the [Architecture](./architecture.md) documentation
- Learn about [Contributing](./contributing.md)
"#,
    )?;

    // Create usage.md
    let usage_md = src_dir.join("usage.md");
    sh.write_file(
        &usage_md,
        r#"# Usage

## Available Commands

The project uses xtask for automation. Here are the available commands:

### Build Commands

```bash
cargo xtask build      # Build the project
cargo xtask check      # Run cargo check
cargo xtask clippy     # Run clippy lints
```

### Test Commands

```bash
cargo xtask test       # Run all tests
```

### Formatting Commands

```bash
cargo xtask fmt        # Format code
cargo xtask fmt --check # Check formatting
```

### Distribution Commands

```bash
cargo xtask dist       # Create distribution
```

### Documentation Commands

```bash
cargo xtask docs       # Build documentation
```

### Utility Commands

```bash
cargo xtask clean      # Clean build artifacts
cargo xtask cargo-lock # Manage Cargo.lock
```
"#,
    )?;

    // Create architecture.md
    let arch_md = src_dir.join("architecture.md");
    sh.write_file(
        &arch_md,
        r#"# Architecture

## Project Structure

```
openprot/
├── openprot/          # Main application
│   ├── src/
│   │   ├── lib.rs     # Library code
│   │   └── main.rs    # Binary entry point
│   └── Cargo.toml
├── xtask/             # Build automation
│   ├── src/
│   │   ├── main.rs    # Task runner
│   │   ├── cargo_lock.rs # Cargo.lock management
│   │   └── docs.rs    # Documentation generation
│   └── Cargo.toml
├── docs/              # Documentation source
├── .cargo/            # Cargo configuration
└── Cargo.toml         # Workspace configuration
```

## Components

### Main Application (`openprot/`)

The main application provides...

### Build System (`xtask/`)

The xtask system provides automated build tasks including:

- Building and testing
- Code formatting and linting
- Distribution creation
- Documentation generation
- Dependency management

### Documentation (`docs/`)

Documentation is built using mdbook and includes:

- User guides
- Developer documentation
- API references
- Architecture documentation
"#,
    )?;

    // Create contributing.md
    let contrib_md = src_dir.join("contributing.md");
    sh.write_file(
        &contrib_md,
        r#"# Contributing

## Development Setup

1. Clone the repository
2. Install dependencies: `cargo xtask check`
3. Run tests: `cargo xtask test`
4. Format code: `cargo xtask fmt`

## Code Style

- Use `cargo xtask fmt` to format code
- Run `cargo xtask clippy` to check for lints
- Ensure all tests pass with `cargo xtask test`

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
"#,
    )?;

    println!(
        "Created default documentation structure in {}",
        docs_dir.display()
    );
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}
