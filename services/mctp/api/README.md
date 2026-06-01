# openprot-mctp-api

Platform-independent MCTP types, traits, and stack facade.

## Overview

This crate defines the API contract between MCTP applications and the MCTP server.
It provides two layers:

1. **`MctpClient` trait** — low-level service-transport interface for
   `req`, `listener`, `recv`, `send`, `drop_handle`, plus local EID control
   (`get_eid` / `set_eid`). Platform-specific clients implement this trait
   over a chosen transport (for example Pigweed IPC, sockets, or test doubles).

2. **`Stack` facade** (`stack` module) — high-level entry point that wraps any `MctpClient`
   and returns typed channel objects (`StackListener`, `StackReqChannel`, `StackRespChannel`)
   that implement the `MctpListener` / `MctpReqChannel` / `MctpRespChannel` traits.

This two-layer design hides both the **concrete MCTP stack implementation** (which lives
inside the server process) and the **transport mechanism** from application code.
Applications depend only on the high-level traits; swapping the transport or stack
requires no application changes.

```text
┌─────────────────────┐
│   Application       │  uses Stack facade, then channel traits
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│   Stack (this crate)│  wraps any MctpClient, returns typed channel handles
└─────────┬───────────┘
          │ MctpClient trait
          ▼
┌─────────────────────┐
│  MctpClient impl    │  encodes wire protocol, uses a transport backend
│  (IPC/sockets/test) │
└─────────┬───────────┘
          │ Transport
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
- **Strategy trait** → `MctpClient` defines the service-transport operations (`req`, `listener`, `recv`, `send`, `drop_handle`) and local EID control (`get_eid`/`set_eid`).
- **Concrete strategies** → transport-specific `MctpClient` implementations (for example IPC clients) and test `DirectClient`

Application code enters through the `Stack` facade and then operates on channels
implementing `MctpListener` / `MctpReqChannel` / `MctpRespChannel`. During
initialization, a concrete `MctpClient` is provided to `Stack::new(client)`;
after that, protocol logic remains transport-agnostic.

This gives two independent axes of variation:

| Concern | How to swap |
|---------|-------------|
| MCTP stack implementation (mctp-lib, etc.) | Replace the server process — no API change |
| OS / IPC transport | Provide a different `MctpClient` impl to `Stack::new` |
| Application logic | Written against the high-level traits — unchanged across both |



The `wire` module implements binary request/response encoding.
It is used internally by transport client implementations and server endpoints;
applications do not use it directly.

## Dependencies

This crate currently has no external Rust crate dependencies.
It is no_std compatible and relies on core plus internal modules.