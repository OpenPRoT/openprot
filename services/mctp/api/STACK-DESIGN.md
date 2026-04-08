# MCTP Stack Design

## Pattern: Facade + Factory Method + Strategy

`Stack<C>` is a **Facade** over any `MctpClient` implementation. It exposes three
factory methods (`req`, `listener`, via listener's `recv` for resp) that produce
typed channel handles. The `C: MctpClient` bound is a compile-time **Strategy**,
making the transport completely swappable without changing call-site code.

---

## Type Hierarchy

```
                       «trait»
                      MctpClient
                    ┌────────────┐
                    │ req()      │
                    │ listener() │
                    │ send()     │
                    │ recv()     │
                    │ drop_handle│
                    │ get/set_eid│
                    └─────┬──────┘
                          │ implemented by
              ┌───────────┼───────────┐
              │           │           │
       IpcMctpClient  LinuxClient  (future)
        (Hubris IPC)  (sockets)
```

---

## Stack Facade

```
 Application code
      │
      │  only sees traits:
      │  MctpReqChannel / MctpListener / MctpRespChannel
      │
      ▼
┌─────────────────────────────────────────────────┐
│                  Stack<C: MctpClient>            │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │  + new(client: C) → Stack<C>             │   │
│  │  + get_eid() → u8                        │   │
│  │  + set_eid(eid) → Result                 │   │
│  │                                          │   │  ◄── Facade
│  │  + req(eid, timeout)                     │   │
│  │      → StackReqChannel<'_, C>            │   │
│  │                                          │   │
│  │  + listener(msg_type, timeout)           │   │
│  │      → StackListener<'_, C>              │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  client: C  ◄── Strategy (hidden from callers)  │
└─────────────────────────────────────────────────┘
```

---

## Channel Products (Factory Method)

```
Stack::req()                           Stack::listener()
     │                                       │
     ▼                                       ▼
┌──────────────────────┐        ┌───────────────────────┐
│  StackReqChannel<C>  │        │   StackListener<C>    │
│                      │        │                       │
│  handle: Handle      │        │  handle: Handle       │
│  eid: u8             │        │  timeout: u32         │
│  sent_tag: Option<u8>│        │  stack: &Stack<C>     │
│  timeout: u32        │        └────────────┬──────────┘
│  stack: &Stack<C>    │                     │
└──────────┬───────────┘                     │ recv() returns
           │                                 ▼
           │ implements           ┌──────────────────────┐
           ▼                      │  StackRespChannel<C> │
    «trait»                       │                      │
  MctpReqChannel                  │  stack: &Stack<C>    │
  ┌──────────────┐                │  eid: u8             │
  │ send()       │                │  msg_type: u8        │
  │ recv()       │                │  tag: u8             │
  │ remote_eid() │                └──────────┬───────────┘
  └──────────────┘                           │ implements
                                             ▼
                                      «trait»
                                    MctpRespChannel
                                    ┌─────────────┐
                                    │ send()      │
                                    │ remote_eid()│
                                    └─────────────┘
```

---

## Full Call-Flow: Server (Listener) Path

```
  App                  Stack<C>           StackListener<C>       MctpClient (C)
   │                      │                     │                      │
   │  listener(type, t)   │                     │                      │
   │─────────────────────►│                     │                      │
   │                      │── listener(type) ──►│                      │
   │                      │                     │─── client.listener() ►│
   │                      │                     │◄── Handle ────────────│
   │◄── StackListener ────│                     │                      │
   │                      │                     │                      │
   │  recv(&mut buf)       │                     │                      │
   │────────────────────────────────────────────►│                      │
   │                      │                     │── client.recv() ─────►│
   │                      │                     │◄── RecvMetadata ──────│
   │                      │                     │  builds StackRespChannel
   │◄── (meta, payload, StackRespChannel) ───────│                      │
   │                      │                     │                      │
   │  resp.send(&reply)   │                     │                      │
   │─────────────────────────────────────────────────────────────────► │
   │                      │                     │  client.send(None,..) │
   │◄── Ok(()) ────────────────────────────────────────────────────────│
   │                      │                     │                      │
   │  [drop listener]     │                     │                      │
   │────────────────────────────────────────────►│                      │
   │                      │                     │── client.drop_handle()►│
```

---

## Full Call-Flow: Client (Request) Path

```
  App                  Stack<C>         StackReqChannel<C>      MctpClient (C)
   │                      │                    │                      │
   │  req(eid, timeout)   │                    │                      │
   │─────────────────────►│                    │                      │
   │                      │── client.req(eid) ──────────────────────► │
   │                      │◄── Handle ─────────────────────────────── │
   │◄── StackReqChannel ──│                    │                      │
   │                      │                    │                      │
   │  send(msg_type, buf) │                    │                      │
   │────────────────────────────────────────── ►│                      │
   │                      │                    │── client.send(..) ───►│
   │                      │                    │◄── tag ───────────────│
   │                      │                    │ sent_tag = Some(tag)  │
   │◄── Ok(()) ───────────────────────────── ──│                      │
   │                      │                    │                      │
   │  recv(&mut buf)      │                    │                      │
   │────────────────────────────────────────── ►│                      │
   │                      │                    │── client.recv(..) ───►│
   │                      │                    │◄── RecvMetadata ──────│
   │◄── (meta, payload) ──────────────────── ──│                      │
   │                      │                    │                      │
   │  [drop channel]      │                    │                      │
   │────────────────────────────────────────── ►│                      │
   │                      │                    │── client.drop_handle()►│
```

---

## Design Patterns Summary

| Pattern | Where | Effect |
|---------|-------|--------|
| **Facade** | `Stack<C>` | Single entry point; hides `MctpClient` complexity and handle lifecycle |
| **Factory Method** | `Stack::req()`, `Stack::listener()` | Produces typed channel structs with lifetime-bound borrows |
| **Strategy** | `C: MctpClient` generic | Transport swapped at compile time — IPC, sockets, mock, etc. |
| **RAII / Handle Guard** | `Drop` on `StackReqChannel` & `StackListener` | Handles are released automatically; no explicit cleanup needed |

### Why Facade fits perfectly here

The classic Facade pattern calls for:

1. A complex subsystem with many low-level operations — ✓ (`MctpClient`: `req`, `listener`, `send`, `recv`, `drop_handle`, `set_eid`)
2. A simplified, cohesive interface for clients — ✓ (`Stack`: `req()` / `listener()` / `get_eid()` / `set_eid()`)
3. The subsystem remaining accessible directly if needed — ✓ (callers can use `MctpClient` directly; `Stack` does not prevent it)

The Strategy (generic `C`) is a natural complement: the Facade is stable, the strategy behind it changes per platform.
