# SPDM Loopback Integration Tests

This test application validates SPDM protocol implementation using in-memory MCTP loopback transport.

## Overview

Tests SPDM requester ↔ responder communication without hardware dependencies by using:
- `MctpSpdmTransport` with `LoopbackClient` from `mctp-transport-loopback`
- Mock platform implementations for cryptographic operations
- Pre-fabricated SPDM messages with assertion-based validation

## Architecture

```
┌─────────────────┐         LoopbackPair         ┌─────────────────┐
│ SPDM Requester  │◄────────────────────────────►│ SPDM Responder  │
│                 │  (in-memory transport)        │                 │
│ - Send requests │                               │ - SpdmContext   │
│ - Parse replies │                               │ - Mock platform │
└─────────────────┘                               └─────────────────┘
```

## Tests

1. **GET_VERSION** - SPDM version negotiation
2. **GET_CAPABILITIES** - Device capability discovery
3. **NEGOTIATE_ALGORITHMS** - Cryptographic algorithm selection

## Building

```bash
bazel build --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-loopback-test:spdm_loopback_test
```

## Running on Hardware

```bash
bazel run --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-loopback-test:spdm_loopback_test
```

## Running in QEMU

```bash
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-loopback-test:spdm_loopback_test_qemu
```
