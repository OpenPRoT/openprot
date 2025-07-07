# Usage

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
