# MCTP Server Test Plan (without I2C transport)

Branch: ocp-emea-demo

## Core Insight
`Server<S: Sender, const N>` is generic over `mctp_lib::Sender`.
I2C transport is just one `Sender` impl + a caller of `server.inbound()`.
No mocking framework, feature flags, or cfg(test) shims needed.

---

## File Layout

```
services/mctp/server/tests/
├── common/
│   └── mod.rs         ← shared BufferSender, transfer(), DirectClient
├── echo.rs            ← already exists; refactor to use common/
├── dispatch.rs        ← already exists; add missing cases
├── server_unit.rs     ← NEW: Layer 2 unit tests
└── integration.rs     ← NEW: Layer 4 multi-fragment / concurrency
```

---

## Layer 1 — Shared Test Fixtures (common/mod.rs)

- [x] Extract `BufferSender<'_>` from echo.rs and dispatch.rs into `tests/common/mod.rs`
- [x] Add `DroppingBufferSender` (discards writes, always returns Ok) for tests that only care about inbound routing
- [x] Extract `transfer(from, to)` helper into common
- [x] Extract `DirectClient<'a, S, N>` into common (wraps &RefCell<Server> as MctpClient)

---

## Layer 2 — Server Unit Tests (server_unit.rs)

- [x] `req()` + `unbind()` — handle allocation/deallocation
- [x] `listener()` duplicate msg_type — expect AlreadyBound error
- [x] `try_recv()` before any `inbound()` — returns None
- [x] `inbound(raw_pkt)` + `try_recv()` — full routing path (via `deliver_to` helper using a temporary sender Server)
- [x] `register_recv()` + `update(now + timeout)` — timeout fires RecvResult::TimedOut
- [x] `set_eid()` / `get_eid()` — EID round-trip
- [x] `send()` with payload > MAX_PAYLOAD — expect NoSpace error

---

## Layer 3 — Dispatch Unit Tests (dispatch.rs additions)

- [x] Malformed wire request → BadArgument
- [x] `MctpOp::Send` via response path (no handle, HAS_EID flag, explicit tag)
- [x] `MctpOp::Unbind` for never-allocated handle → idempotent success
- [x] `MctpOp::Recv` when no message ready → TimedOut

---

## Layer 4 — Integration Tests (integration.rs)

- [x] Multi-fragment roundtrip: set `get_mtu()=64`, send 200-byte payload, verify reassembly
- [x] Multiple concurrent listeners: two msg_type values, cross-deliver, verify no cross-talk
- [x] Response-without-handle: verify tag & EID threading through echo
- [ ] Interleaved requests from two senders: tag collision avoidance ← not yet implemented

---

## Layer 5 — MctpClient Trait Tests (via DirectClient / DirectListener)

- [x] `MctpListener::recv()` + `MctpRespChannel::send()` — echo via trait (`echo_via_mctplistener_trait`)
- [x] `MctpReqChannel::send()` + `recv()` — full request-response cycle (`req_channel_send_recv`)
- [x] `drop_handle` mid-flight — verify outstanding entry is cleared (`drop_handle_mid_flight_clears_entry`)

---

## Out of Scope (belongs in other crates)

| Concern | Owner crate |
|---|---|
| MCTP-over-I2C framing/PEC | openprot-mctp-transport-i2c |
| MctpI2cReceiver::decode | openprot-mctp-transport-i2c |
| I2cClientBlocking mock | i2c service tests |
| IpcI2cClient / handle::I2C wiring | target/platform integration |
