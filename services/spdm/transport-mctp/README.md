# openprot-spdm-transport-mctp

MCTP transport layer for SPDM protocol communication.

## Overview

This crate implements the `SpdmTransport` trait from `spdm-lib` using MCTP as the underlying transport mechanism. It bridges the SPDM protocol layer with the MCTP messaging infrastructure.

## MCTP Binding

- **Message Type**: 0x05 (SPDM, per DMTF DSP0236 §4.2.1)
- **Max Message Size**: 2048 bytes
- **Fragmentation**: Handled by MCTP layer
- **Tag Correlation**: MCTP tags used for request/response matching

## Modes

### Requester Mode
Used by SPDM requesters (clients) that initiate attestation operations:
```rust
let transport = MctpSpdmTransport::new_requester(mctp_client, remote_eid);
```

### Responder Mode
Used by SPDM responders (servers) that handle attestation requests:
```rust
let transport = MctpSpdmTransport::new_responder(mctp_client);
```

## Transport Lifecycle

1. **Initialize**: `init_sequence()` establishes MCTP handles
   - Requester: Creates request handle to remote EID
   - Responder: Registers listener for SPDM messages

2. **Communication**:
   - Requester: `send_request()` → `receive_response()`
   - Responder: `receive_request()` → `send_response()`

3. **Cleanup**: Automatically releases MCTP handles on drop

## Dependencies

- `openprot-mctp-api` — MCTP client trait and types
- `spdm-lib` — SPDM protocol library with transport trait

## Usage

This crate is used by both:
- `openprot-spdm-requester` — SPDM client implementation
- `openprot-spdm-responder` — SPDM server implementation
