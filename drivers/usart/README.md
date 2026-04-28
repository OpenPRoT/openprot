# USART Driver Model

This document describes the architecture of the layered USART userspace driver
under `drivers/usart/` and how it integrates with the AST10x0 platform codebase.

## 1. Layer Overview

```
┌──────────────────────────────────────────────────────────┐
│  Application / Client                                    │
│  UsartClient  (drivers/usart/client)                     │
│  channel_transact(request) → response                    │
└────────────────────────┬─────────────────────────────────┘
                         │  Pigweed IPC channel
                         ▼
┌──────────────────────────────────────────────────────────┐
│  Server Binary  (drivers/usart/server:usart_server_bin)  │
│  rust_app — dispatch loop                                │
│  object_wait → channel_read → dispatch_request           │
│               → channel_respond                          │
│                                                          │
│  Backend resolved at compile-time via label_flag:        │
│  usart_backend::Backend (crate_name = "usart_backend")   │
└────────────────────────┬─────────────────────────────────┘
                         │  UsartBackend trait
                         ▼
┌──────────────────────────────────────────────────────────┐
│  Platform Backend  (target/ast10x0/backend/usart)        │
│  Ast10x0UsartBackend : UsartBackend                      │
│  pub type Backend = Ast10x0UsartBackend                  │
│  [TODO: wire to ast10x0_peripherals::uart::Usart]        │
└────────────────────────┬─────────────────────────────────┘
                         │  embedded-io / embedded-hal-nb
                         ▼
┌──────────────────────────────────────────────────────────┐
│  PAC-level UART driver  (target/ast10x0/peripherals)     │
│  Usart — raw MMIO handle over ast1060_pac::RegisterBlock │
└──────────────────────────────────────────────────────────┘
```

## 2. Crate Map

| Bazel target | Crate | Role |
|---|---|---|
| `//drivers/usart/api` | `usart_api` | Wire protocol + backend trait contract |
| `//drivers/usart/server:usart_server` | `usart_server` | Pure dispatch library (host-testable) |
| `//drivers/usart/server:usart_server_bin` | binary | IPC entry point (`rust_app`) |
| `//drivers/usart/client` | `usart_client` | Client facade for callers |
| `//target/ast10x0/backend/usart` | `usart_backend` | AST10x0 backend implementation |
| `//target/ast10x0/peripherals` | `ast10x0_peripherals` | Raw MMIO UART driver |
| `//drivers/usart/tests` | — | Dispatch smoke tests with mock backend |

## 3. Wire Protocol  (`usart_api::protocol`)

Operations are identified by a 1-byte opcode in `UsartRequestHeader` (8 bytes, `repr(C, packed)`):

| Op | Value | Args |
|---|---|---|
| `Configure` | 0x01 | baud_rate split across `arg0`/`arg1` |
| `Write` | 0x02 | payload bytes |
| `Read` | 0x03 | `arg0` = max bytes to read |
| `GetLineStatus` | 0x04 | — |
| `EnableInterrupts` | 0x05 | `arg0` = mask |
| `DisableInterrupts` | 0x06 | `arg0` = mask |

`UsartResponseHeader` (4 bytes) carries a `UsartError` status code plus a payload length.
All structures implement `zerocopy` traits for zero-copy serialization.

## 4. Backend Trait  (`usart_api::backend`)

```rust
pub trait UsartBackend {
    fn configure(&mut self, config: UsartConfig) -> Result<(), BackendError>;
    fn write(&mut self, data: &[u8]) -> Result<usize, BackendError>;
    fn read(&mut self, out: &mut [u8]) -> Result<usize, BackendError>;
    fn line_status(&self) -> Result<LineStatus, BackendError>;
    fn enable_interrupts(&mut self, mask: u16) -> Result<(), BackendError>;
    fn disable_interrupts(&mut self, mask: u16) -> Result<(), BackendError>;
}
```

`BackendError` maps 1-to-1 onto `UsartError` via `From<BackendError> for UsartError`.

## 5. Compile-Time Backend Selection

The server binary's backend is selected through a Bazel `label_flag`:

```python
# drivers/usart/server/BUILD.bazel
label_flag(
    name = "backend",
    build_setting_default = "//target/ast10x0/backend/usart:usart_backend_ast10x0",
)
```

Every backend target must:
- Use `crate_name = "usart_backend"`
- Export `pub type Backend = <concrete type implementing UsartBackend>`

`main.rs` is backend-agnostic:

```rust
use usart_backend::Backend;
let mut backend = Backend::new();
```

### Platform-Driven Selection

The preferred integration path sets the flag inside the platform definition so
`--platforms=//target/ast10x0:ast10x0` alone resolves the backend — no extra
command-line flags needed:

```python
# target/ast10x0/BUILD.bazel
flags = flags_from_dict(
    KERNEL_DEVICE_COMMON_FLAGS | {
        ...
        "//drivers/usart/server:backend":
            "//target/ast10x0/backend/usart:usart_backend_ast10x0",
    },
)
```

A new platform adds only its own entry here with a different backend target.

## 6. Dispatch Library (`usart_server`)

`dispatch_request<B: UsartBackend>(backend, request, response) -> usize` is a
pure function with no IPC or OS dependency — it can be built and tested on the
host. The IPC loop lives exclusively in `main.rs`.

## 7. System Image

The server binary is packaged as a kernel system image via:

```
//target/ast10x0/usart:usart  (system_image)
  kernel = :target
  apps   = [//drivers/usart/server:usart_server_bin]
  system_config = :system_config   (target/ast10x0/usart/system.json5)
```

`system.json5` defines the app's memory layout, the `usart` IPC channel handle,
the UART5 MMIO mapping (`0x7e784000`), and the server thread stack size.
The handle constant `handle::USART` is generated by `rust_app`/`app_usart_server`
codegen from the object named `usart` in the config.

## 8. Extension Points

- **New platform**: add `"//drivers/usart/server:backend": "//target/<plat>/backend/usart:..."` to the platform's `flags_from_dict`.
- **New operation**: add opcode to `UsartOp`, extend `UsartBackend`, add arm to `dispatch_request`, update protocol doc.
- **Real hardware wiring**: implement `UsartBackend` for `ast10x0_peripherals::uart::Usart` inside `target/ast10x0/backend/usart/src/lib.rs`.

