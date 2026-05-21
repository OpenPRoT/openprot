# PLDM Service and MctpClient Integration

## Overview

`openprot-pldm-service` is the glue between the PLDM protocol layer
(`pldm-interface`) and the MCTP transport layer (`openprot-mctp-api`).
It is designed to be platform-independent: all MCTP interaction is
expressed through the `MctpClient` trait, so the same PLDM service
code runs on bare-metal Hubris IPC, Linux sockets, or any other
transport that implements the trait.

## Layer diagram

```
┌─────────────────────────────────────┐
│  Application / Firmware             │
│  • declares ProtocolCapability[]    │
│  • allocates a static buffer        │
│  • calls PldmResponder::run_once()  │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│  openprot-pldm-service              │
│  PldmResponder                      │
│  • adjusts the buffer layout        │
│  • delegates dispatch to            │
│    CmdInterface (pldm-interface)    │
└──────────────┬──────────────────────┘
               │  MctpListener / MctpRespChannel
               ▼
┌─────────────────────────────────────┐
│  openprot-mctp-api                  │
│  Stack<C: MctpClient>               │
│  • hides handle allocation          │
│  • implements MctpListener,         │
│    MctpReqChannel, MctpRespChannel  │
└──────────────┬──────────────────────┘
               │  concrete IPC / socket calls
               ▼
┌─────────────────────────────────────┐
│  MctpClient implementation          │
│  (Hubris IPC, Linux sockets, …)     │
└─────────────────────────────────────┘
```

## Key types and their roles

| Type | Crate | Role |
|------|-------|------|
| `PldmResponder<'a>` | `openprot-pldm-service` | Owns a `CmdInterface`; call `run_once()` in a loop to process one message per call |
| `PldmServiceError` | `openprot-pldm-service` | Wraps `MctpError`, `MsgHandlerError`, or an `Overflow` sentinel |
| `Stack<C>` | `openprot-mctp-api` | Facade that wraps any `MctpClient` and exposes `MctpListener` / `MctpReqChannel` |
| `MctpClient` | `openprot-mctp-api` | Trait implemented per platform; owns raw handle allocation and send/recv |
| `MctpListener` | `openprot-mctp-api` | Trait returned by `Stack::listener()`; delivers one incoming message |
| `MctpRespChannel` | `openprot-mctp-api` | Trait returned alongside received data; used to send the reply |
| `CmdInterface<'a>` | `pldm-interface` | Stateful PLDM dispatcher; handles command routing and response generation |
| `ProtocolCapability<'a>` | `pldm-interface` | Describes one PLDM type (e.g. Base), its version, and its supported commands |

## The `MctpClient` trait

```rust
pub trait MctpClient {
    fn req(&self, eid: u8) -> Result<Handle, MctpError>;
    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError>;
    fn get_eid(&self) -> u8;
    fn set_eid(&self, eid: u8) -> Result<(), MctpError>;
    fn recv(&self, handle: Handle, timeout_millis: u32, buf: &mut [u8])
        -> Result<RecvMetadata, MctpError>;
    fn send(&self, handle: Option<Handle>, msg_type: u8,
            eid: Option<u8>, tag: Option<u8>,
            integrity_check: bool, buf: &[u8]) -> Result<u8, MctpError>;
    fn drop_handle(&self, handle: Handle);
}
```

All methods take `&self` (not `&mut self`), so the trait uses interior
mutability internally. This allows a single `Stack<C>` reference to be
shared across listener and request channels simultaneously.

`PldmResponder` never calls `MctpClient` directly. Instead it calls
`Stack::listener()`, which internally calls `MctpClient::listener()` and
wraps the resulting `Handle` in a `StackListener`. When the `StackListener`
is dropped (at the end of `run_once`), it automatically calls
`MctpClient::drop_handle()` through the `Drop` impl on `StackListener`.

## What `run_once` does step by step

```
run_once(&stack, buf, timeout_millis)
│
├─ 1. stack.listener(PLDM_MSG_TYPE=0x01, timeout)
│      → MctpClient::listener() allocates a Handle
│      → returns StackListener (holds the Handle)
│
├─ 2. listener.recv(buf[1..])
│      → MctpClient::recv() blocks until a PLDM message arrives
│      → writes PLDM bytes into buf[1..]  (no MCTP framing byte)
│      → returns (RecvMetadata, payload_slice, StackRespChannel)
│
├─ 3. buf[0] = 0x01  (prepend MCTP message-type byte)
│      buf[0..1+payload_size] is now the full CmdInterface input
│
├─ 4. cmd_interface.handle_responder_msg(buf[0..total_len])
│      → CmdInterface routes the command (GetTid, SetTid, …)
│      → overwrites the same buffer with the PLDM response
│      → returns resp_len (total bytes written, including buf[0])
│
├─ 5. resp_channel.send(buf[1..resp_len])
│      → MctpClient::send() transmits the PLDM response bytes
│        (handle=None signals a response, not a new request)
│
└─ 6. StackListener is dropped → MctpClient::drop_handle() called
```

## Buffer layout

`CmdInterface` expects a single contiguous buffer where byte 0 is the
MCTP message-type byte and the PLDM header + payload follow immediately:

```
buf[0]       : MCTP message-type (0x01 for PLDM)
buf[1]       : PLDM header byte 0
buf[2]       : PLDM header byte 1
buf[3]       : PLDM header byte 2  (command code)
buf[4..]     : PLDM payload (command-specific)
```

`MctpClient::recv()` writes only the PLDM bytes (no framing byte) into
the supplied buffer. `run_once` therefore receives into `buf[1..]` and
sets `buf[0] = 0x01` before handing the whole slice to `CmdInterface`.
After dispatch, the response is also written in-place starting at
`buf[0]`, and `buf[1..resp_len]` (i.e. without the MCTP type byte) is
passed to `MctpClient::send()`.

The minimum useful buffer size is:

```
1 (MCTP type byte) + 3 (PLDM header) + max_payload_bytes
```

A 64-byte buffer covers all Base PLDM commands. Larger commands (e.g.
`GetPldmVersion` with multi-part transfers) may need more.

## Error handling

`run_once` returns `Result<(), PldmServiceError>` where:

| Variant | Cause |
|---------|-------|
| `PldmServiceError::Mctp(e)` | Any `MctpClient` call failed (timeout, server restart, no space, …) |
| `PldmServiceError::MsgHandler(e)` | `CmdInterface::handle_responder_msg` could not process the message |
| `PldmServiceError::Overflow` | `buf` is too small to hold the MCTP type byte plus the received payload |

All errors are recoverable: the caller may log the error and call
`run_once` again on the next iteration.

## Implementing `MctpClient` for a new platform

To use `PldmResponder` on a new platform, implement `MctpClient`:

```rust
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata};

struct MyMctpClient { /* platform-specific state */ }

impl MctpClient for MyMctpClient {
    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        // Register with the MCTP server for msg_type 0x01 (PLDM)
        // Return an opaque Handle
    }
    fn recv(&self, handle: Handle, timeout_millis: u32, buf: &mut [u8])
        -> Result<RecvMetadata, MctpError>
    {
        // Block until a message arrives, write PLDM bytes into buf
        // Return RecvMetadata { payload_size, remote_eid, msg_tag, … }
    }
    fn send(&self, handle: Option<Handle>, msg_type: u8,
            eid: Option<u8>, tag: Option<u8>,
            _integrity_check: bool, buf: &[u8]) -> Result<u8, MctpError>
    {
        // handle=None means this is a response; use eid+tag to route it back
        // Transmit buf as the PLDM payload
    }
    fn drop_handle(&self, handle: Handle) {
        // Unregister the handle from the MCTP server
    }
    // req / get_eid / set_eid are not used by PldmResponder
    fn req(&self, _eid: u8) -> Result<Handle, MctpError> { unimplemented!() }
    fn get_eid(&self) -> u8 { 0 }
    fn set_eid(&self, _eid: u8) -> Result<(), MctpError> { Ok(()) }
}
```

Then wire it in:

```rust
use openprot_mctp_api::Stack;
use openprot_pldm_service::responder::PldmResponder;

let stack = Stack::new(MyMctpClient::new());
let mut responder = PldmResponder::new(&CAPS);
let mut buf = [0u8; 128];

loop {
    if let Err(e) = responder.run_once(&stack, &mut buf, 0) {
        // log e and continue
    }
}
```
