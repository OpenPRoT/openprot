# Hardware Implementations

This section covers platform-specific driver implementations.

## AST1060

### Adapter Pattern

The low-level AST1060 driver (`Ast1060I2c`) manages **one controller** per instance:

```rust
pub struct Ast1060I2c<'a> {
    controller: &'a I2cController<'a>,  // Bound to ONE controller
    xfer_mode: I2cXferMode,
    multi_master: bool,
}
```

The `I2cHardware` trait expects **one object** to manage **all controllers**, so we use an adapter:

```rust
pub struct Ast1060I2cAdapter<'a> {
    controllers: [Option<Ast1060I2c<'a>>; 14],
    slave_states: [SlaveState; 14],
}
```

The adapter provides:
1. **Controller Routing**: Selects the right instance based on `Controller` enum
2. **Software Buffering**: Adds target message buffer
3. **State Management**: Tracks per-controller target configuration
4. **Unified Interface**: Matches the trait contract

### Transfer Modes

The AST1060 supports two transfer modes:

| Mode | Description | Use Case |
|------|-------------|----------|
| Byte Mode | Transfer one byte at a time | Small transfers |
| Buffer Mode | Use hardware 32-byte buffer | Larger payloads (default) |

### Implementation Files

| File | Purpose |
|------|---------|
| `drv/ast1060-i2c/src/server_driver.rs` | I2cHardware trait implementation |
| `drv/ast1060-i2c/src/slave.rs` | Target mode hardware operations |
| `drv/ast1060-i2c/src/master.rs` | Controller mode hardware operations |
| `drv/ast1060-i2c/src/transfer.rs` | Low-level transfer modes |

## Adding New Hardware Support

To add support for a new I2C controller:

1. Create a new driver crate (e.g., `drv/newchip-i2c/`)
2. Implement the `I2cHardware` trait
3. Handle hardware-specific initialization and interrupt management
4. Add feature flag to `openprot-i2c-server`
