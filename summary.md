# SPDM Req-Resp Test — Crypto Integration Summary

## What was done

### 1. Add crypto server to the virt system image (`1ca5fd6`)

- `target/virt-ast1060-evb/spdm-req-resp-test/system.json5` — added `crypto_server` app (64KB flash at `0x28000`, 64KB RAM); reduced `spdm_requester` RAM 96KB→32KB to stay within the 640KB limit
- `services/crypto/server/BUILD.bazel` — added `exports_files(["src/main.rs"])`
- `target/virt-ast1060-evb/spdm-req-resp-test/BUILD.bazel` — added `crypto_server_app` target wiring directly to `services/crypto/server/src/main.rs`
- Test passes: crypto server starts, blocks on `object_wait`, SPDM exchange completes, QEMU exits cleanly

### 2. Wire SPDM requester/responder to the crypto server (`c50a38a`)

- `mock_platform.rs` extracted into a `rust_library` Bazel target (`//target/ast1060-evb/spdm-req-resp-test:mock_platform`) — no duplication, shared by all four binaries (ast1060 + virt, requester + responder)
- `target/virt-ast1060-evb/spdm-req-resp-test/spdm_requester.rs` created — uses `SpdmCryptoHash` and `SpdmCryptoRng` (IPC-backed) instead of mocks
- `target/virt-ast1060-evb/spdm-req-resp-test/spdm_responder.rs` created — same
- `system.json5` updated — `CRYPTO` channel_initiator added to both requester and responder processes
- VCA exchange (GET_VERSION, GET_CAPABILITIES, NEGOTIATE_ALGORITHMS) passes

### 3. Add GET_DIGESTS + fix crypto session isolation (`7fe9bc6`)

**Problem discovered:** The M1 transcript hash (`m1_hash`) opens a streaming SHA-384 session during ALGORITHMS and keeps it open across message boundaries. When GET_DIGESTS runs, `hash` tries to open another session on the same crypto server → `SessionBusy` → responder fails.

**Fix:** Give each `SpdmCryptoHash` instance its own dedicated IPC channel handle:

| Handle | Used by |
|---|---|
| `CRYPTO_REQ` | requester `hash` + `rng` |
| `CRYPTO_REQ_M1` | requester `m1_hash` |
| `CRYPTO_REQ_L1` | requester `l1_hash` |
| `CRYPTO_RESP` | responder `hash` + `rng` |
| `CRYPTO_RESP_M1` | responder `m1_hash` |
| `CRYPTO_RESP_L1` | responder `l1_hash` |

The crypto server now has 6 `channel_handler` objects and a `wait_group`, serving each with its own independent `HashSession` state.

**GET_DIGESTS added to the requester flow** — first step that actually exercises the crypto server (SHA-384 hash over the responder's certificate chain via IPC).

## Final test result

```
//target/virt-ast1060-evb/spdm-req-resp-test:spdm_req_resp_test_test  PASSED
```

SPDM exchange flow: GET_VERSION → GET_CAPABILITIES → NEGOTIATE_ALGORITHMS → GET_DIGESTS
All hash and RNG operations served by the crypto server over IPC using RustCrypto backend.

## Memory layout (virt-ast1060-evb)

```
0x00000500 - 0x00010000  Kernel flash         (~62KB)
0x00010000 - 0x00018000  mctp_loopback_server  (32KB)
0x00018000 - 0x00020000  spdm_requester        (32KB)
0x00020000 - 0x00028000  spdm_responder        (32KB)
0x00028000 - 0x00038000  crypto_server         (64KB)
0x00058000 - 0x00070000  Kernel RAM            (96KB)
0x00070000 - 0x00078000  mctp_loopback_server  (32KB)
0x00078000 - 0x00080000  spdm_requester        (32KB)
0x00080000 - 0x00090000  spdm_responder        (64KB)
0x00090000 - 0x000A0000  crypto_server         (64KB)
Total: 640KB
```
