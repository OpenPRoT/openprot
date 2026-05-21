# SPDM Requester-Responder Integration Test

Tests SPDM requester ↔ responder communication using two separate processes
connected via an MCTP loopback server.

## Overview

Unlike the `spdm-loopback-test` (single process, manual packet transfer), this test
uses three separate userspace processes communicating over Pigweed IPC channels:

1. **MCTP Loopback Server** — Routes MCTP messages between requester and responder
   via in-memory `BufferSender` loopback (no physical transport needed)
2. **SPDM Requester** — Uses spdm-lib's requester API (`generate_get_version`,
   `requester_send_request`, `requester_process_message`) to execute the VCA flow
3. **SPDM Responder** — Uses spdm-lib's responder API (`responder_process_message`)
   to handle incoming SPDM requests

## Architecture

```
┌─ spdm_requester ──┐      ┌─ mctp_loopback_server ─────────┐      ┌─ spdm_responder ─┐
│ IpcMctpClient      │─IPC─▶│ Server(EID 8)  ←loopback→      │◀─IPC─│ IpcMctpClient     │
│ MctpSpdmTransport  │      │              Server(EID 42)     │      │ MctpSpdmTransport │
│ SpdmContext        │      │ (BufferSender cross-wired)      │      │ SpdmContext       │
└────────────────────┘      └──────────────────────────────────┘      └───────────────────┘
```

## Test Flow

The requester executes the SPDM VCA (Version, Capabilities, Algorithms) flow:

1. `GET_VERSION` → `VERSION`
2. `GET_CAPABILITIES` → `CAPABILITIES`
3. `NEGOTIATE_ALGORITHMS` → `ALGORITHMS`

On success, the requester calls `debug_shutdown(Ok(()))`.

## Building

```bash
bazel build --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-req-resp-test:spdm_req_resp_test
```

## Running in QEMU

```bash
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-req-resp-test:spdm_req_resp_test_qemu
```

## Running on Hardware

```bash
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-req-resp-test:spdm_req_resp_test_uart_test \
    --test_env=UART_DEVICE=/dev/ttyUSB0
```
