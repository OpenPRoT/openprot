# openprot-mctp-api

Platform-independent MCTP types, traits, and stack facade.

## Overview

This crate defines the API contract between MCTP applications and the MCTP server.
It provides two layers:

1. **`MctpClient` trait** — low-level interface mirroring the IPC wire operations
   (req, listener, recv, send, drop_handle). Platform-specific crates such as
   `openprot-mctp-client` implement this trait using the OS transport (e.g. Pigweed IPC).

2. **`Stack` facade** (`stack` module) — high-level entry point that wraps any `MctpClient`
   and returns typed channel objects (`StackListener`, `StackReqChannel`, `StackRespChannel`)
   that implement the `MctpListener` / `MctpReqChannel` / `MctpRespChannel` traits.

This two-layer design hides both the **concrete MCTP stack implementation** (which lives
inside the server process) and the **OS / IPC transport** from application code.
Applications depend only on the high-level traits; swapping the transport or stack
requires no application changes.

```text
┌─────────────────────┐
│   Application       │  uses MctpListener / MctpReqChannel / MctpRespChannel traits
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│   Stack (this crate)│  wraps any MctpClient, returns typed channel handles
└─────────┬───────────┘
          │ MctpClient trait
          ▼
┌─────────────────────┐
│  IpcMctpClient      │  encodes wire protocol, calls OS IPC (e.g. Pigweed channel_transact)
│  (mctp-client crate)│
└─────────┬───────────┘
          │ IPC
          ▼
┌─────────────────────┐
│   MCTP Server       │  owns the concrete MCTP stack (mctp-lib, etc.)
└─────────────────────┘
```

## Key Types

- `Handle` — opaque handle for listeners, request, or response channels
- `RecvMetadata` — metadata from a successful receive (msg_type, tag, remote_eid, payload_size)
- `MctpError` / `ResponseCode` — error types (InternalError, NoSpace, AddrInUse, TimedOut, BadArgument, ServerRestarted)

## High-level API (`stack` module)

| Type | Trait | Obtained via |
|------|-------|--------------|
| `Stack<C>` | — | `Stack::new(client)` |
| `StackListener<'_, C>` | `MctpListener` | `stack.listener(msg_type, timeout)` |
| `StackReqChannel<'_, C>` | `MctpReqChannel` | `stack.req(eid, timeout)` |
| `StackRespChannel<'_, C>` | `MctpRespChannel` | returned by `StackListener::recv` |

All channel types release their server-side handle automatically on `Drop`.

## Low-level API (`MctpClient` trait)

| Method | Description |
|--------|-------------|
| `req(eid)` | Allocate a request handle for a remote EID |
| `listener(msg_type)` | Register to receive messages of a given type |
| `get_eid() / set_eid(eid)` | Read/write the local endpoint ID |
| `recv(handle, timeout, buf)` | Receive a message on a handle |
| `send(handle, msg_type, eid, tag, ic, buf)` | Send a message (request or response) |
| `drop_handle(handle)` | Release a handle |

## Design: Strategy Pattern

`Stack<C: MctpClient>` applies the **Strategy pattern**:

- **Context** → `Stack<C>` holds the strategy and exposes the high-level API
- **Strategy trait** → `MctpClient` defines the IPC operations (req, listener, recv, send, drop_handle)
- **Concrete strategies** → `IpcMctpClient` (Pigweed IPC), test `DirectClient`, future Linux socket client

Applications code against `MctpListener` / `MctpReqChannel` / `MctpRespChannel` traits and never
see the strategy. The concrete `MctpClient` implementation is injected via `Stack::new(client)`.

This gives two independent axes of variation:

| Concern | How to swap |
|---------|-------------|
| MCTP stack implementation (mctp-lib, etc.) | Replace the server process — no API change |
| OS / IPC transport | Provide a different `MctpClient` impl to `Stack::new` |
| Application logic | Written against the high-level traits — unchanged across both |



The `wire` module implements binary request/response encoding for IPC communication.
It is used internally by `openprot-mctp-client` and the server; applications do not
use it directly.

## Dependencies

- `zerocopy` — zero-copy serialization
- `heapless` — `no_std` collections

This crate is `no_std` compatible.
