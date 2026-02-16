# I2C Service Framework

## Overview

The I2C service is a microkernel-style peripheral server built on the Pigweed
kernel.  A dedicated userspace **server** process owns the I2C hardware
registers and exposes operations to **client** processes over IPC channels.
Clients never touch hardware directly — they serialize requests into a compact
wire protocol, transact over a kernel channel, and deserialize the response.

```text
┌─────────────────────┐        IPC channel        ┌─────────────────────┐
│   Client process    │◄─────────────────────────►│   Server process    │
│  (i2c_client lib)   │   channel_transact()       │  (i2c_server bin)   │
│                     │                            │                     │
│  IpcI2cClient       │                            │  AspeedI2cBackend   │
│    .write()         │                            │    .write()         │
│    .read()          │                            │    .read()          │
│    .write_read()    │                            │    .write_read()    │
│    .probe()         │                            │    .recover_bus()   │
└─────────────────────┘                            └──────────┬──────────┘
                                                              │
                                                   aspeed-ddk │ Ast1060I2c
                                                              ▼
                                                   ┌──────────────────┐
                                                   │  I2C Controller  │
                                                   │  (MMIO regs)     │
                                                   └──────────────────┘
```

---

## Crate Layers

### 1. `i2c_api` — Wire Protocol & Types

Platform-independent crate defining the types shared between client and server.

| Item | Purpose |
|------|---------|
| `I2cRequestHeader` / `I2cResponseHeader` | Fixed-size headers serialized into IPC payloads |
| `I2cOp` | Enum: `Write`, `Read`, `WriteRead`, `Probe`, `Recover` |
| `ResponseCode` | Wire-level result codes (`Ok`, `NoDevice`, `Busy`, …) |
| `BusIndex`, `I2cAddress` | Validated newtypes for bus number and 7-bit address |
| `I2cClient` trait | `write`, `read`, `write_read`, `probe` |
| `encode_*` / `decode_*` | Request/response serialization helpers |

**No dependencies on kernel, IPC, or hardware.**

```
//services/i2c/api:i2c_api
  deps: embedded-hal
```

### 2. `i2c_client` — IPC Client Library

Userspace library implementing `I2cClient` over Pigweed IPC.
Each method encodes a request, calls `channel_transact()`, and decodes the
server's response.

```
//services/i2c/client:i2c_client
  deps: i2c_api, userspace, embedded-hal
```

Usage:

```rust
let client = IpcI2cClient::new(handle::I2C);  // handle from app_package
client.write(BusIndex::BUS_0, addr, &data)?;
```

### 3. `i2c_backend_aspeed` — Hardware Backend

Server-side adapter wrapping `aspeed-ddk` to drive AST1060 I2C controllers.

**Two-layer initialization model:**

| Layer | Registers | When | Where |
|-------|-----------|------|-------|
| Platform | SCU reset, I2CG0C/I2CG10, SCU4xx pinmux | Boot (single-threaded) | `entry.rs` |
| Per-bus | I2CC00, timing, I2CM10/I2CM14 | Server startup | `backend.init_bus(n)` |
| Per-operation | None (zero register writes) | Each IPC request | `Ast1060I2c::from_initialized()` |

Platform init sets up shared SCU/global registers that cannot be safely
modified from multiple processes.  Per-bus init configures the individual
controller assigned to this server.  Per-operation access creates a transient
handle on the stack with no register writes.

```
//services/i2c/backend-aspeed:i2c_backend_aspeed
  deps: i2c_api, aspeed-ddk, ast1060-pac, pw_log
```

### 4. `i2c_server` — Server Binary

Userspace process that owns the I2C hardware.  Runs a single-threaded
dispatch loop:

1. `object_wait(handle::I2C, READABLE)` — block until client request
2. `channel_read()` — deserialize `I2cRequestHeader`
3. Dispatch to `AspeedI2cBackend` method
4. `channel_respond()` — serialize `I2cResponseHeader` + data

```
//services/i2c/server:i2c_server
  deps: app_i2c_server, i2c_api, i2c_backend_aspeed,
        syscall_user, userspace, pw_log, pw_status
```

### 5. `i2c_client_test` — Test Binary

Userspace process that exercises the server through the client library.
Runs a sequence of test operations (probe, write, read, invalid bus) and
calls `debug_shutdown()` with the result.

```
//services/i2c/tests:i2c_client_test
  deps: app_i2c_client, i2c_api, i2c_client,
        syscall_user, userspace, pw_log, pw_status
```

---

## Bazel Build Structure

### Dependency Graph

```text
target/ast1060-evb/i2c:i2c          (system_image)
  ├── :target                        (rust_binary — kernel)
  │   ├── :codegen                   (target_codegen)
  │   ├── :linker_script             (target_linker_script)
  │   ├── //target/ast1060-evb:entry
  │   └── @pigweed//pw_kernel/...
  │
  ├── //services/i2c/server:i2c_server   (rust_binary — app)
  │   ├── :app_i2c_server                (app_package)
  │   ├── //services/i2c/api
  │   └── //services/i2c/backend-aspeed
  │       ├── //services/i2c/api
  │       ├── @oot_crates_no_std//:aspeed-ddk
  │       └── @oot_crates_no_std//:ast1060-pac
  │
  └── //services/i2c/tests:i2c_client_test  (rust_binary — app)
      ├── :app_i2c_client                   (app_package)
      ├── //services/i2c/api
      └── //services/i2c/client
```

### Key Bazel Rules

#### `system_image`

Combines kernel binary + app binaries into a single flashable image.
Applies a Bazel configuration transition that sets the target platform
and system config flag for all transitive dependencies.

```starlark
system_image(
    name = "i2c",
    apps = [
        "//services/i2c/tests:i2c_client_test",
        "//services/i2c/server:i2c_server",
    ],
    kernel = ":target",
    platform = "//target/ast1060-evb",
    system_config = ":system_config",
)
```

#### `app_package`

Generates a Rust crate from `system.json5` containing typed handle constants.
Each process's `objects` array entry becomes a `pub const` in the generated
`handle` module (index 0 → `handle::I2C = 0`).

```starlark
app_package(
    name = "app_i2c_server",
    app_name = "i2c_server",        # must match system.json5 app name
    system_config = "//target/ast1060-evb/i2c:system_config",
)
```

The `system_config` label is **hardcoded** per target, not the generic Bazel
flag.  The `system_image` rule's transition sets the flag independently.

#### `target_codegen`

Generates a `codegen` crate with a `start()` function that boots all
userspace processes.  The kernel's `target.rs` calls `codegen::start()` to
hand off to userspace.

#### `target_linker_script`

Generates a linker script from `system.json5` memory layout definitions,
placing vector table, kernel code, app code, kernel RAM, and app RAM
in their configured regions.

---

## System Configuration (`system.json5`)

The system configuration declares the memory map, process layout, kernel
objects, and thread stacks for the entire image.

### Memory Layout (AST1060-EVB)

```
0x00000000 ┌──────────────────────┐
           │ Vector Table +       │ 1184 bytes (0x4A0)
           │ Kernel Annotations   │
0x000004A0 ├──────────────────────┤
           │ Kernel Code          │ ~126 KB
0x00020000 ├──────────────────────┤
           │ I2C Server App       │ 128 KB
0x00040000 ├──────────────────────┤
           │ I2C Client App       │ 128 KB
0x00060000 ├──────────────────────┤
           │ Kernel RAM           │ 128 KB
0x00080000 ├──────────────────────┤
           │ Server RAM           │  64 KB
0x00090000 ├──────────────────────┤
           │ Client RAM           │  64 KB
0x000A0000 └──────────────────────┘
           Total: 640 KB / 768 KB SRAM
```

### IPC Channel Objects

Kernel objects are declared per-process and linked across apps:

```json5
// Server process
objects: [{ name: "I2C", type: "channel_handler" }]

// Client process
objects: [{
    name: "I2C",
    type: "channel_initiator",
    handler_app: "i2c_server",
    handler_object_name: "I2C",
}]
```

The `channel_handler` / `channel_initiator` pair creates a bidirectional IPC
channel.  The kernel connects them at boot based on the `handler_app` and
`handler_object_name` references.

---

## Build Commands

```bash
# Build the complete system image
bazel build --config=k_ast1060_evb //target/ast1060-evb/i2c:i2c

# Run QEMU test (virtual target)
bazel test --config=virt_ast1060_evb //target/ast1060-evb/i2c:i2c_test

# Flash to physical board via UART
bazel run --config=k_ast1060_evb //target/ast1060-evb/i2c:upload_i2c

# Build API library and run host tests
bazel test //services/i2c/api:i2c_api_test
```

---

## Adding a New I2C Target

To create a system image for a different board:

1. Create `target/<board>/i2c/system.json5` with the board's memory map
   and the same app/channel object structure.

2. Create `target/<board>/i2c/BUILD.bazel` with `system_image`, `codegen`,
   `linker_script`, and `target` rules pointing to the board's platform
   and entry crate.

3. Create `target/<board>/i2c/target.rs` with the board's `TargetInterface`
   impl (typically just `codegen::start()` + `shutdown()`).

4. Update `app_package` rules in `services/i2c/server/BUILD.bazel` and
   `services/i2c/tests/BUILD.bazel` if needed, or add per-board
   `app_package` targets with the new `system_config` label.

The `services/i2c/api`, `services/i2c/client`, and
`services/i2c/backend-aspeed` crates are target-independent and shared
across all board configurations.
