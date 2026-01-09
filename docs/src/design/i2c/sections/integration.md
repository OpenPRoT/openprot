# System Integration

This section covers how to configure and deploy I2C in a Hubris system.

## Task Configuration (app.toml)

### Basic I2C Driver Task

```toml
[tasks.i2c_driver]
name = "drv-openprot-i2c-server"
priority = 2
max-sizes = {flash = 16384, ram = 4096}
notifications = ["i2c-irq"]
features = ["ast1060"]
uses = ["i2c0", "i2c1", "scu", "i2c_global"]
```

### MCTP Server Task (using I2C)

```toml
[tasks.mctp_server]
name = "mctp-server"
priority = 3
max-sizes = {flash = 32867, ram = 16384}
task-slots = ["uart_driver", "i2c_driver"]
notifications = ["rx-data", "i2c-rx", "timer"]
features = ["serial_log", "transport_i2c"]
```

## Hubris Per-Task Device Isolation

Hubris enforces strict hardware peripheral isolation using the ARM Memory Protection Unit (MPU). Each task can **only** access the specific memory regions explicitly assigned to it.

### How Device Isolation Works

**1. Configuration (app.toml)**

Tasks declare which peripherals they need via the `uses` field:

```toml
[tasks.i2c_driver]
name = "drv-openprot-i2c-server"
priority = 2
uses = ["i2c0", "i2c1", "scu", "i2c_global"]  # Only these peripherals accessible
```

**2. Build Time Processing**

The build system converts the `uses` list into MPU region descriptors.

**3. Context Switch Enforcement**

On every context switch, the kernel loads the new task's region table into the MPU.

**4. Hardware Enforcement**

If a task attempts to access a peripheral not in its `uses` list, the MPU raises a **MemManageFault**.

### Isolation Example

```toml
# Task A can only access I2C0 and I2C1
[tasks.i2c_server_a]
uses = ["i2c0", "i2c1", "scu", "i2c_global"]

# Task B can only access I2C2 and I2C3
[tasks.i2c_server_b]
uses = ["i2c2", "i2c3", "scu", "i2c_global"]
```

With this configuration:
- Task A accessing I2C2 registers → **MemManageFault** (access denied)
- Task B accessing I2C0 registers → **MemManageFault** (access denied)
- Task A accessing I2C0 registers → Allowed
- Task B accessing I2C2 registers → Allowed

## Platform-Specific Integration

Different platforms have unique constraints and configuration requirements:

- **[AST1060 Integration](./integration_ast1060.md)** - MPU region budgets, multi-controller deployments, and AST1060-specific peripheral configuration
