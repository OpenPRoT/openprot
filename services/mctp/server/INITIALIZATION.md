# MCTP Server — Initialization and Variable Glossary

Source: `services/mctp/server/src/main.rs`

---

## Build Modes

The server compiles into one of several mutually exclusive modes, selected by
Cargo feature flags set in the Bazel `rust_app` target.  `in-process-requester`
and `in-process-responder` are mutually exclusive and the binary emits a
`compile_error!` if both are enabled at once.

| Feature flags | Mode | IPC served? | SPDM role in-process |
|---|---|---|---|
| _(none)_ | Notification | Yes | none |
| `i2c-polling` | Polling | No | none |
| `i2c-polling` + `direct-client` + `in-process-responder` | Polling + SPDM | No | responder |
| `i2c-polling` + `direct-client` + `in-process-requester` | Polling + SPDM | No | requester |

Build targets:

```
bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp:mctp                      # responder image
bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp-requester:mctp_requester  # requester image
```

---

## Initialization Sequence

### Shared constants (always)

```
OWN_EID = 0x08       — MCTP Endpoint ID assigned to this device
OWN_I2C_ADDR = 0x10  — I2C 7-bit slave address of this device
```

---

### Polling mode (`i2c-polling`)

Steps execute once at startup before the event loop begins.

```
1. IpcI2cClient::new(handle::I2C)
       → Opens IPC channel to the I2C server (for configuration and rx).
         Stored in `i2c`.

2. I2cAddress::new(OWN_I2C_ADDR)
       → Validates the address value fits in 7 bits.
         Stored in `addr`.

3. i2c.configure_target_address(BUS_2, addr)
       → Programs the I2C hardware to respond to OWN_I2C_ADDR on bus 2.
         FATAL if this fails (I2C server unreachable or bad address).

4. i2c.enable_receive(BUS_2)
       → Arms the I2C slave receive FIFO. After this the hardware will
         accept incoming frames into the I2C server's buffer.
         FATAL if this fails.

5. IpcI2cClient::new(handle::I2C)  [second instance]
       → A separate IPC channel to the I2C server, used exclusively for
         outbound I2C master writes. Ownership is transferred to I2cSender.

6. I2cSender::new(i2c_client_2, BUS_2, OWN_I2C_ADDR)
       → Outbound MCTP-over-I2C transport. Implements mctp_lib::Sender.
         Stored in `sender`.

7. MctpI2cReceiver::new(OWN_I2C_ADDR)
       → Inbound frame decoder. Strips I2C framing to produce raw MCTP
         packets. Stored in `receiver`.

8a. (direct-client only) RefCell::new(Server::new(Eid(OWN_EID), 0, sender))
       → MCTP router wrapped in RefCell for shared &self access.
         Stored in `server_cell`.

8b. (no direct-client) Server::new(Eid(OWN_EID), 0, sender)
       → MCTP router, exclusively owned.
         Stored in `server_plain`.

── SPDM setup (in-process-responder) ──────────────────────────────────────

9.  DirectMctpClient::new(&server_cell)
       → In-process MctpClient. Calls server_cell methods directly;
         no IPC channel to a separate process.
         Consumed immediately by step 10.

10. MctpSpdmTransport::new_responder(client)
       → SPDM transport layer in responder mode (listens for msg_type=0x05).
         Stored in `transport`.

11. transport.init_sequence()
       → Calls server.listener(0x05) in-process, reserving a listener handle
         in the router for SPDM messages.
         FATAL if the router's listener table is full (MAX_LISTENERS=8).

12. MockCertStore / MockHash / MockRng / MockEvidence
       → Placeholder platform implementations.  No crypto hardware access.
         Stored in `cert_store`, `hash`, `m1_hash`, `l1_hash`, `rng`, `evidence`.

13. CapabilityFlags { CERT_CAP, CHAL_CAP, MEAS_CAP=2, MEAS_FRESH_CAP, CHUNK_CAP }
       → Bitmask of SPDM capabilities advertised to the requester.
         Stored in `flags`, then moved into `capabilities`.

14. DeviceCapabilities { ct_exponent=0, data_transfer_size=1024,
                         max_spdm_msg_size=4096 }
       → SPDM device capability parameters. Stored in `capabilities`.

15. SUPPORTED_VERSIONS = [V12, V13]  (static)
       → SPDM protocol versions this responder accepts.

16. LocalDeviceAlgorithms { SHA-384, ECDSA-P384, DMTF measurements }
       → Cryptographic algorithm selections. Stored in `algorithms`.

17. SpdmContext::new(...)
       → Constructs the full SPDM state machine. Takes references to all
         platform impls (transport, cert_store, hash, rng, evidence).
         FATAL if any required platform impl is misconfigured.
         Stored in `spdm_ctx`.

18. [0u8; MAX_PAYLOAD_SIZE]  +  MessageBuf::new(&mut spdm_buf)
       → Fixed-size scratch buffer for SPDM message encoding/decoding.
         msg_buf wraps spdm_buf for the lifetime of the loop.
         Both declared outside the loop to avoid borrow-checker conflict.
```

── SPDM setup (in-process-requester) ──────────────────────────────────────

Mirrors steps 9–18 of the responder path; only the differences are shown.

```
9.   DirectMctpClient::new(&server_cell)     [identical to responder]

10.  MctpSpdmTransport::new_requester(client, REMOTE_RESPONDER_EID)
       → SPDM transport layer in requester mode (targets EID 42).
         `REMOTE_RESPONDER_EID = 42` matches `spdm_requester.rs`.

11.  transport.init_sequence()
       → Calls server.req(REMOTE_RESPONDER_EID) in-process, reserving a
         request handle in the router.
         FATAL if the router's request table is full.

12.  MockCertStore / MockHash×3 / MockRng / MockEvidence  [identical]
12a. DemoPeerCertStore::default()            [requester-only]
       → Provides the peer's certificate chain to SPDM; required even if
         only VCA is exercised.  Stored in `peer_cert_store`.

13.  CapabilityFlags { cert_cap=1, chal_cap=1, meas_cap=0, chunk_cap=1 }
       → No `meas_fresh_cap` (requester does not publish measurements).

14.  DeviceCapabilities { include_supported_algorithms: false }
       → V1.3 param1 bit 2 is rejected by the responder as reserved; leave
         it unset so algorithm negotiation goes through NEGOTIATE_ALGORITHMS.

15.  SUPPORTED_VERSIONS = [V12, V13]         [identical]
16.  LocalDeviceAlgorithms (SHA-384, ECDSA-P384)  [identical]

17.  SpdmContext::new(..., Some(&mut peer_cert_store), ...)
       → Requester passes `Some(peer_cert_store)` (responder passes `None`).

18.  spdm_buf + msg_buf                      [identical]

19.  req_state = ReqState::SendVersion        [requester-only]
       → Initial FSM state.  The Phase-2 step below walks through
         SendVersion → AwaitVersion → SendCapabilities → AwaitCapabilities →
         SendAlgorithms → AwaitAlgorithms → Done (or → Failed on error /
         AWAIT_STEP_BUDGET exhaustion).
```

The polling loop then runs two phases per iteration:

```
Phase 1 (always):     drain I2C → server.inbound() → router reassembly
Phase 2 (responder):  spdm_ctx.responder_process_message() — Err on no data
Phase 2 (requester):  FSM step — advance on Ok, stay on Err, transition to
                      Failed if await_steps hits AWAIT_STEP_BUDGET (10_000)
```

---

### Notification mode (default, no `i2c-polling`)

```
1. IpcI2cClient::new(handle::I2C)
       → IPC channel to I2C server for configuration, notifications, and
         pending-message retrieval. Stored in `i2c_notify`.

2-4.   Same address configuration as polling mode (steps 2-4 above).

5.  i2c_notify.register_notification(BUS_2, 0)
       → Requests the I2C server to post a USER signal to this process
         when a slave-mode frame arrives. FATAL if it fails.

6.  IpcI2cClient::new(handle::I2C)  [second instance]
       → Separate channel for outbound sends, transferred to I2cSender.

7-8.   I2cSender and MctpI2cReceiver as in polling mode.

9.  Server::new(Eid(OWN_EID), 0, sender)
       → MCTP router. Stored in `server`.

10. [0u8; MAX_REQUEST_SIZE/MAX_RESPONSE_SIZE/MAX_PAYLOAD_SIZE]
       → IPC wire buffers. Stored in `request_buf`, `response_buf`, `recv_buf`.

11. syscall::wait_group_add(WG, MCTP, READABLE, user_data=0)
       → Register the IPC channel as event source 0 in the WaitGroup.

12. syscall::wait_group_add(WG, I2C, USER, user_data=1)
       → Register the I2C notification signal as event source 1.
```

---

## Variable Glossary

### Constants

| Name | Value | Meaning |
|---|---|---|
| `OWN_EID` | `0x08` | MCTP Endpoint ID of this device |
| `OWN_I2C_ADDR` | `0x10` | I2C 7-bit slave address of this device |
| `REMOTE_RESPONDER_EID` | `42` | MCTP EID of the peer responder (requester-only) |
| `AWAIT_STEP_BUDGET` | `10_000` | Max FSM steps per Await state before Failed (requester-only) |

### Polling mode variables

| Name | Type | Lifetime | Purpose |
|---|---|---|---|
| `i2c` | `IpcI2cClient` | setup + loop | I2C IPC channel for configuration, `wait_for_messages`, rx |
| `addr` | `I2cAddress` | setup only | Validated I2C address; consumed by `configure_target_address` |
| `sender` | `I2cSender` | loop (via server) | Outbound MCTP-over-I2C; owns a second `IpcI2cClient` |
| `receiver` | `MctpI2cReceiver` | loop | Decodes I2C slave frames into raw MCTP packets |
| `server_cell` | `RefCell<Server>` | loop | MCTP router; RefCell allows shared borrow with DirectMctpClient |
| `server_plain` | `Server` | loop | MCTP router when `direct-client` is not enabled |
| `transport` | `MctpSpdmTransport` | loop (via spdm_ctx) | SPDM transport; calls server_cell through DirectMctpClient |
| `cert_store` | `MockCertStore` | loop (via spdm_ctx) | Provides certificate chain to SPDM protocol |
| `hash` | `MockHash` | loop (via spdm_ctx) | Main transcript hash |
| `m1_hash` | `MockHash` | loop (via spdm_ctx) | M1 measurement hash (measurements with signature) |
| `l1_hash` | `MockHash` | loop (via spdm_ctx) | L1 challenge hash |
| `rng` | `MockRng` | loop (via spdm_ctx) | Random number source (nonces, challenge data) |
| `evidence` | `MockEvidence` | loop (via spdm_ctx) | Device measurement evidence provider |
| `flags` | `CapabilityFlags` | setup only | SPDM capability bitmask; consumed by `capabilities` |
| `capabilities` | `DeviceCapabilities` | setup only | SPDM capability parameters; consumed by `SpdmContext::new` |
| `SUPPORTED_VERSIONS` | `[SpdmVersion; 2]` | `'static` | SPDM versions [V1.2, V1.3]; must be static due to spdm-lib lifetime |
| `algorithms` | `LocalDeviceAlgorithms` | setup only | Algorithm selections; consumed by `SpdmContext::new` |
| `spdm_ctx` | `SpdmContext` | loop | Full SPDM state machine; processes one request per `process_message` call |
| `spdm_buf` | `[u8; MAX_PAYLOAD_SIZE]` | loop | Raw byte storage for SPDM message encoding |
| `msg_buf` | `MessageBuf` | loop | Cursor over `spdm_buf`; reset each iteration via `msg_buf.reset()` |

### Notification mode variables

| Name | Type | Lifetime | Purpose |
|---|---|---|---|
| `i2c_notify` | `IpcI2cClient` | loop | I2C channel for slave config, notification registration, and `get_pending_messages` |
| `addr` | `I2cAddress` | setup only | Validated I2C address |
| `sender` | `I2cSender` | loop (via server) | Outbound MCTP-over-I2C |
| `receiver` | `MctpI2cReceiver` | loop | Decodes I2C slave frames |
| `server` | `Server` | loop | MCTP router |
| `request_buf` | `[u8; MAX_REQUEST_SIZE]` | loop | IPC receive buffer for inbound MCTP operation requests |
| `response_buf` | `[u8; MAX_RESPONSE_SIZE]` | loop | IPC transmit buffer for operation responses |
| `recv_buf` | `[u8; MAX_PAYLOAD_SIZE]` | loop | Scratch buffer passed to `dispatch_mctp_op` for message payload |

### Fault-isolation counters (polling mode, loop-scoped)

| Name | Increments on | Logged when |
|---|---|---|
| `i2c_pkt` | Successful I2C frame decode | Every packet (at `debug` level) |
| `i2c_recv_err` | `wait_for_messages` returns `Err` | 1st occurrence, then every 16th |
| `decode_err` | `receiver.decode()` failure | 1st occurrence, then every 16th |
| `inbound_err` | `server.inbound()` returns `Err` | 1st occurrence, then every 16th |
| `spdm_ok` | `responder_process_message` returns `Ok` (responder-only) | Every success (at `info` level) |
| `spdm_err` | `responder_process_message` returns `Err` (responder-only) | 1st occurrence, then every 256th (at `debug` level — mostly TimedOut noise) |

### Requester FSM variables (`in-process-requester`, loop-scoped)

| Name | Type | Purpose |
|---|---|---|
| `req_state` | `ReqState` | Current FSM state (SendVersion → AwaitAlgorithms → Done/Failed) |
| `req_send_ok` | `u32` | Successful `requester_send_request` calls (3 on happy path) |
| `req_recv_ok` | `u32` | Successful `requester_process_message` calls (3 on happy path) |
| `req_recv_pending` | `u32` | `requester_process_message` returned `Err` (mostly TimedOut) |
| `await_steps` | `u32` | Consecutive Phase-2 steps in the current Await state; reset on transition; triggers Failed at `AWAIT_STEP_BUDGET` |
| `peer_cert_store` | `DemoPeerCertStore` | Peer cert chain handed to `SpdmContext::new` (requester-only) |

---

## Handle IDs (`app_mctp_server::handle` or `app_mctp_server_requester::handle`)

Each Bazel `rust_app` target generates its own codegen crate:
`app_mctp_server` for the responder image, `app_mctp_server_requester` for the
requester image.  `src/main.rs` picks the right one with a cfg-gated `use`.
The numeric values are target-specific and should not be hardcoded.

| Handle | Used by | Purpose |
|---|---|---|
| `handle::I2C` | Both modes | IPC channel to `i2c_server` process |
| `handle::MCTP` | Notification mode only | IPC channel handler for MCTP client connections |
| `handle::WG` | Notification mode only | WaitGroup multiplexing IPC + I2C notification |
