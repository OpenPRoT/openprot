# Architecture

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
