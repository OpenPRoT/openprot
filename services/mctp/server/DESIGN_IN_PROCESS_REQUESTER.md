# In-Process SPDM Requester — Design Spec

Status: draft
Scope: add an in-process SPDM **requester** to the MCTP server, mirroring the
existing in-process SPDM responder at
[src/main.rs:144-446](src/main.rs#L144-L446).

---

## 1. Motivation

The responder path collapses three user-space processes (mctp_server +
spdm_responder + i2c_server) into two by running the SPDM responder inside
the MCTP server via `DirectMctpClient`. We want the symmetric capability on
the requester side: drive SPDM VCA / attestation flows from inside the MCTP
server without an IPC channel to a separate `spdm_requester` app.

Benefits (same as responder):

- No `channel_handler` / `channel_initiator` pair between mctp_server and
  spdm_requester — lower RAM, fewer kernel objects in `system.json5`.
- No IPC serialize/deserialize on every SPDM request.
- One process to flash/debug for requester-side scenarios.

---

## 2. Key Asymmetry vs. the Responder

The responder is **passive** — it consumes what arrives. Its loop is
naturally polling-friendly:

```
loop {
    drain_i2c_into_router();                    // Phase 1
    let _ = ctx.responder_process_message(..);  // Phase 2: TimedOut == normal
}
```

The requester is **active** — it initiates exchanges. `requester_process_message`
internally calls `receive_response`, which calls `client.recv(..)`.
`DirectMctpClient::recv` returns `Err(TimedOut)` immediately if the Router has
not yet assembled a full message. So a requester that calls
`requester_process_message` before I2C has delivered the response will *fail
fast* instead of blocking.

The design must therefore **interleave I2C draining with SPDM progress** while
the requester is waiting for a response. Two viable shapes:

| # | Shape | Loop call site | SPDM call site |
|---|---|---|---|
| A | External FSM with two-phase loop (same shape as responder) | mctp_server main | Only advances when I2C has produced a message |
| B | Blocking client that pumps I2C inside `recv` | mctp_server main (simpler) | `requester_process_message` looks synchronous |

We pick **Shape A** for symmetry with the responder and because it keeps
`DirectMctpClient` unchanged and decoupled from I2C. Shape B is recorded in
§9 as a future simplification.

---

## 3. Architecture

```text
┌─ I2C Hardware ───────────────────────────────────────┐
│  Slave-mode frames arrive on bus 2                   │
│  (responses from the remote SPDM responder)          │
└──────────────┬───────────────────────────────────────┘
               │ wait_for_messages() → TargetMessage
               ▼
┌─ MCTP Server Process ────────────────────────────────┐
│                                                      │
│  Phase 1 (always):                                   │
│     receiver.decode() → raw MCTP packet              │
│     server_cell.borrow_mut().inbound(pkt)            │
│        └─ Router reassembles fragments               │
│                                                      │
│  Phase 2 (requester FSM step):                       │
│     match req_state {                                │
│       SendVersion → generate_get_version             │
│                     ctx.requester_send_request()     │
│                     → I2cSender → wire               │
│                     state = AwaitVersion             │
│       AwaitVersion → ctx.requester_process_message() │
│                      Ok  → state = SendCaps          │
│                      Err → stay (TimedOut == normal) │
│       … SendCaps / AwaitCaps / SendAlgs / AwaitAlgs  │
│       Done        → log success, transition to Idle  │
│     }                                                │
│                                                      │
│     All SPDM transport I/O goes through              │
│     DirectMctpClient ↔ server_cell (no IPC).         │
└──────────────────────────────────────────────────────┘
```

Same `RefCell<Server>` shared-ownership trick as the responder
([direct_client.rs](src/direct_client.rs)).

---

## 4. Compilation Switches

### 4.1 Cargo features (services/mctp/server/Cargo.toml)

Current:

```toml
default = ["i2c-polling", "direct-client"]
i2c-polling    = []   # polling I2C loop (no WG/IRQ)
direct-client  = []   # expose DirectMctpClient
```

Add one new feature:

```toml
# Build the in-process SPDM requester inside the polling loop.
# Mutually exclusive with `in-process-responder`.
# Depends on `direct-client` (the in-process MctpClient impl).
in-process-requester = ["direct-client"]
```

Rename the existing responder role to a feature that makes the two roles
symmetric and mutually exclusive:

```toml
# Build the in-process SPDM responder inside the polling loop.
# Mutually exclusive with `in-process-requester`.
in-process-responder = ["direct-client"]
```

Keep `direct-client` as the lower-level capability (exposes
`DirectMctpClient`). Either role feature implies it.

Backwards compatibility: keep `direct-client` alone as a valid build (library
only; no SPDM role instantiated in main), and treat the old
`i2c-polling + direct-client` combination as equivalent to
`i2c-polling + in-process-responder` via a transitional alias in `Cargo.toml`:

```toml
# Transitional: prior builds selected the responder via bare `direct-client`.
# New builds should name the role explicitly.
```

The Bazel `rust_app` target keeps `direct-client` until call sites migrate to
the role-named feature.

### 4.2 Build-time guard

A `compile_error!` in [src/main.rs](src/main.rs) rejects enabling both roles:

```rust
#[cfg(all(feature = "in-process-requester", feature = "in-process-responder"))]
compile_error!(
    "features `in-process-requester` and `in-process-responder` are mutually exclusive"
);
```

### 4.3 Bazel `rust_app` target

New target `mctp_server_requester` in
[services/mctp/server/BUILD.bazel](services/mctp/server/BUILD.bazel), parallel
to the existing `mctp_server` target:

```starlark
rust_app(
    name = "mctp_server_requester",
    codegen_crate_name = "app_mctp_server",       # same handle::MCTP/I2C bindings
    srcs = ["src/main.rs"],
    crate_features = [
        "i2c-polling",
        "in-process-requester",
    ],
    edition = "2024",
    system_config = "//target/ast1060-evb/mctp:system_config",
    tags = ["kernel"],
    visibility = ["//visibility:public"],
    deps = [
        # … identical deps to mctp_server …
    ],
)
```

The existing `mctp_server` target flips to
`crate_features = ["i2c-polling", "in-process-responder"]` once downstream
system images migrate.

### 4.4 System configuration

No new kernel objects are required. The in-process requester reuses exactly
the same `system.json5` process definition as the in-process responder
([target/ast1060-evb/mctp/system.json5](../../target/ast1060-evb/mctp/system.json5)):
`MCTP` channel_handler (still unused in polling+in-process mode), `I2C`
channel_initiator, `WG` wait_group (unused in polling mode).

---

## 5. Module Layout

All new code in [services/mctp/server/src/main.rs](src/main.rs); no new files
unless a helper grows past ~50 lines.

```
src/main.rs
 ├─ imports                                 (cfg-gated by role)
 ├─ consts: OWN_EID, OWN_I2C_ADDR,
 │          REMOTE_RESPONDER_EID (new, 42 to match spdm_requester.rs)
 ├─ mctp_loop()  [feature = "i2c-polling"]
 │    ├─ I2C setup (unchanged)
 │    ├─ Server::new → RefCell<Server>    (unchanged)
 │    ├─ #[cfg(in-process-responder)]  responder setup (unchanged)
 │    ├─ #[cfg(in-process-requester)]  requester setup  (new, §6)
 │    └─ loop {
 │          Phase 1: drain I2C → server.inbound()       (unchanged)
 │          Phase 2:
 │             #[cfg(in-process-responder)] responder_step(..)
 │             #[cfg(in-process-requester)] requester_step(&mut req_state, ..)
 │       }
 ├─ mctp_loop()  [not(i2c-polling)]         (unchanged)
 └─ entry / panic_handler                   (unchanged)
```

If `requester_step` / `requester_setup` together exceed ~100 lines, move
them into a new module `src/in_process_requester.rs`, gated
`#[cfg(feature = "in-process-requester")]`, exposing:

```rust
pub(crate) struct InProcessRequester<'a, S: Sender, const N: usize> { … }
impl<…> InProcessRequester<'_, S, N> {
    pub fn new(server: &'a RefCell<Server<S, N>>, remote_eid: u8, …) -> Result<Self, Error>;
    pub fn step(&mut self);    // one FSM step; no-op when Done
    pub fn is_done(&self) -> bool;
}
```

Prefer the in-line `main.rs` layout first (matches the responder pattern);
extract only if the symmetry breaks readability.

---

## 6. Requester State Machine

### 6.1 States

```rust
#[cfg(feature = "in-process-requester")]
enum ReqState {
    SendVersion,
    AwaitVersion,
    SendCapabilities,
    AwaitCapabilities,
    SendAlgorithms,
    AwaitAlgorithms,
    Done,       // success; loop idles in Phase 2 no-op
    Failed,     // terminal; loop idles in Phase 2 no-op
}
```

Start state: `SendVersion`.

### 6.2 Step semantics (Phase 2)

One **step** = one Phase 2 invocation of the FSM from one loop iteration.
A step must return promptly so that Phase 1 keeps draining I2C on the next
iteration — a step never blocks waiting for a response.

```rust
match req_state {
    ReqState::SendVersion => {
        msg_buf.reset();
        if generate_get_version(&mut ctx, &mut msg_buf, VersionReqPayload::new(0, 0)).is_err()
           || ctx.requester_send_request(&mut msg_buf, REMOTE_RESPONDER_EID).is_err() {
            req_state = ReqState::Failed;
        } else {
            req_state = ReqState::AwaitVersion;
        }
    }
    ReqState::AwaitVersion => {
        msg_buf.reset();
        match ctx.requester_process_message(&mut msg_buf) {
            Ok(_)  => req_state = ReqState::SendCapabilities,
            Err(_) => { /* TimedOut == stay; protocol errors also stay but counted */ }
        }
    }
    // … same pattern for Capabilities, Algorithms …
    ReqState::Done | ReqState::Failed => { /* no-op */ }
}
```

### 6.3 Distinguishing "no message yet" from real protocol errors

`requester_process_message` currently returns a single `Err(_)` value that
collapses transport `TimedOut` with genuine SPDM protocol errors. Two options:

1. **Short-term**: Ride on `Err(_)` as "stay in state" — accept that a real
   protocol error would wedge the requester in an await state forever. Add
   a safety-net **retry budget** per await state (e.g. 10,000 steps) after
   which `req_state = ReqState::Failed`.

2. **Proper fix**: Plumb `TransportError` through `requester_process_message`
   so callers can discriminate. Preferred, but touches spdm-lib — track as
   a follow-up.

Ship option 1; open follow-up for option 2.

### 6.4 Retry / timeout budget

Per await state:

```rust
const AWAIT_STEP_BUDGET: u32 = 10_000;
let mut await_steps: u32 = 0;
// on each Err in an Await* state:
await_steps = await_steps.wrapping_add(1);
if await_steps >= AWAIT_STEP_BUDGET {
    req_state = ReqState::Failed;
}
// reset to 0 on state transition
```

10k steps at the measured steady-state poll rate gives an order-of-magnitude
wall-clock ceiling; the exact number is tunable once we measure.

### 6.5 Interleaving constraints

The two-phase loop's interleaving of I2C draining and SPDM FSM steps is the
mechanism that makes the design work — without Phase 1 pumping inbound
fragments into the router, Phase 2's `requester_process_message` would
never see an assembled response. Three properties must hold for this to
be safe and responsive:

1. **Latency tax per step.** Each loop iteration runs Phase 1 (one
   `wait_for_messages` call) before Phase 2 (FSM step). If the response is
   already assembled in the router, the FSM still waits out a Phase 1 poll
   cycle before consuming it — one I2C poll-budget duration of added
   latency per step. Acceptable for VCA at wire speed; revisit if
   attestation flows grow enough steps to care.

2. **Multi-fragment responses.** `wait_for_messages` reads at most one
   frame per call (`msgs: [TargetMessage; 1]`). A response that spans N
   MCTP fragments needs N Phase-1 iterations before Phase 2 can succeed,
   so `AWAIT_STEP_BUDGET` (§6.4) must cover
   `N × (I2C poll period)` for the largest expected message. The 10,000
   default is generous for VCA; recheck when GET_CERTIFICATE lands.

3. **`wait_for_messages` must be non-blocking.** The design relies on
   `wait_for_messages(..., None)` returning `Err(TimedOut)` when idle
   rather than blocking forever; otherwise Phase 2 never runs. The
   responder loop already depends on this — see the comment at
   [src/main.rs:390-393](src/main.rs#L390-L393).

Non-issues (explicitly enumerated so future readers don't reopen them):

- **Cross-flow routing.** The MCTP router dispatches by handle; unrelated
  inbound MCTP traffic cannot land in the requester's `req_handle` queue.
- **RefCell borrow conflicts.** Phase 1's `server.borrow_mut()` finishes
  before Phase 2 starts; the borrows never overlap.
- **Dropped frames during send.** `requester_send_request` blocks on IPC
  to the I2C server, but slave-mode receives are buffered in the I2C
  server process and picked up on the next `wait_for_messages`.

---

## 7. Initialization Sequence (requester mode)

Executed once before the loop, between I2C setup and the `loop { }`. Mirrors
responder setup at
[src/main.rs:182-299](src/main.rs#L182-L299); the only functional differences
are marked **[R]**.

```
1.  DirectMctpClient::new(&server)              [same]
2.  MctpSpdmTransport::new_requester(client, REMOTE_RESPONDER_EID)  [R]
3.  transport.init_sequence()                   [R: allocates req handle,
                                                     not listener]
4.  MockCertStore / MockHash×3 / MockRng / MockEvidence            [same]
5.  DemoPeerCertStore (needed for future GET_DIGESTS/CERTIFICATE)   [R]
6.  CapabilityFlags: cert_cap=1, chal_cap=1, meas_cap=0, chunk_cap=1 [R]
                                                 (no meas_fresh_cap; meas_cap=0)
7.  DeviceCapabilities { include_supported_algorithms: false }      [R]
                                (see spdm_requester.rs comment: V1.3 param1 bit 2
                                 is rejected by current responder)
8.  LocalDeviceAlgorithms (identical to responder)  [same]
9.  SpdmContext::new(..., peer_cert_store=Some(&mut demo_peer), ...) [R]
10. MessageBuf (reused across FSM states)        [same]
11. req_state = ReqState::SendVersion            [R]
```

Constants borrowed verbatim from
[target/ast1060-evb/spdm-req-resp-test/spdm_requester.rs](../../target/ast1060-evb/spdm-req-resp-test/spdm_requester.rs)
so behavior matches the existing external requester app bit-for-bit.

---

## 8. Observability

Mirror the responder’s fault-isolation counters. All `u32`, log on first
event and every Nth after to avoid flooding:

| Counter | Increment on | Log cadence |
|---|---|---|
| `i2c_pkt` | successful `receiver.decode` | already exists |
| `inbound_err` | `server.inbound()` error | already exists |
| `decode_err` | `receiver.decode()` error | already exists |
| `i2c_recv_err` | `wait_for_messages` non-timeout error | already exists |
| `idle_polls` | `wait_for_messages` timeout | already exists |
| `req_state_entered` | each ReqState transition | log every transition |
| `req_send_ok` | successful requester_send_request | every event |
| `req_send_err` | failed requester_send_request | every event (terminal) |
| `req_recv_ok` | successful requester_process_message | every event |
| `req_recv_pending` | Err in Await* state (treated as TimedOut) | first + every 256th |
| `req_recv_err_terminal` | state transitioned to Failed | every event |

A `ReqState::Done` transition logs:

```
SPDM VCA completed: version/caps/algs OK in N steps, elapsed ~M idle_polls
```

after which the loop continues stepping (Phase 1 still drains I2C so later
inbound traffic is handled; Phase 2 is a no-op).

---

## 9. Open Questions / Future Work

1. **Blocking DirectMctpClient variant (Shape B)**: add an
   `I2cPumpingDirectMctpClient` that holds `&I2cClient + &MctpI2cReceiver +
   &RefCell<Server>` and, inside `recv`, drains I2C until a message is
   assembled (or a deadline expires). With that, `spdm_requester_test()` from
   the existing app could be reused almost verbatim and the external FSM
   disappears. Requires splitting current Server RefCell borrows or wrapping
   in a struct to satisfy the borrow checker.

2. **TransportError discrimination**: plumb the underlying `TransportError`
   through `requester_process_message` so we can distinguish
   `NoRequestInFlight` / `ReceiveError(TimedOut)` from protocol errors
   without a step budget.

3. **Triggering**: when does the requester start? Currently the spec assumes
   "immediately at boot, single-shot VCA". A real use will want external
   triggers — an IPC operation (`start_spdm`, target EID as arg), a GPIO,
   or a policy. Out of scope here; add when there is a caller.

4. **Mutual exclusion of roles**: we chose mutually-exclusive features. If
   we later want a **dual role** (a device that both attests others and is
   attested), we need two SpdmContexts sharing one Server — tractable but
   not covered by this spec.

5. **Mock platform sharing**: `target/ast1060-evb/spdm-req-resp-test/mock_platform`
   is already a dep of `mctp_server`. Reuse as-is; if the cert material
   diverges between requester and responder builds, consider a
   `mock_platform_requester` / `mock_platform_responder` split.

---

## 10. Summary of Compilation Switch Matrix (post-change)

| Features | I2C IRQ | IPC served | In-process role |
|---|---|---|---|
| _(none)_ | WG + IRQ | MCTP clients | none |
| `i2c-polling` | polling | none | none |
| `i2c-polling` + `in-process-responder` | polling | none | **responder** |
| `i2c-polling` + `in-process-requester` | polling | none | **requester** |
| any combination setting both roles | — | — | `compile_error!` |

Default remains `i2c-polling + in-process-responder` (current behavior),
until system images choose otherwise.
