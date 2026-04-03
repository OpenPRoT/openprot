# Using MCTP Transport-Loopback for SPDM Testing

This document shows how to use the MCTP transport-loopback with SPDM to create fully in-memory SPDM testing.

## Basic Pattern

```rust
use core::cell::RefCell;
use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
use openprot_spdm_transport_mctp::MctpSpdmTransport;

// 1. Create MCTP loopback infrastructure (no heap required)
let packets_a = RefCell::new(PacketBuffer::new());
let packets_b = RefCell::new(PacketBuffer::new());
let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);

// 2. Get MCTP clients for each endpoint
let client_requester = pair.client_a();  // EID 8 (requester)
let client_responder = pair.client_b();  // EID 42 (responder)

// 3. Create SPDM transports
let mut spdm_requester = MctpSpdmTransport::new_requester(client_requester, 42);
let mut spdm_responder = MctpSpdmTransport::new_responder(client_responder);

// 4. Initialize both transports
spdm_requester.init_sequence().unwrap();
spdm_responder.init_sequence().unwrap();

// 5. Send SPDM request
let mut request_buf = MessageBuf::new();
// ... populate request_buf with SPDM GET_VERSION or other message ...
spdm_requester.send_request(42, &mut request_buf).unwrap();

// 6. Transfer MCTP packets from requester to responder
pair.transfer_a_to_b();

// 7. Receive SPDM request on responder
let mut received_request = MessageBuf::new();
spdm_responder.receive_request(&mut received_request).unwrap();

// 8. Send SPDM response
let mut response_buf = MessageBuf::new();
// ... populate response_buf with SPDM response ...
spdm_responder.send_response(&mut response_buf).unwrap();

// 9. Transfer MCTP packets from responder to requester
pair.transfer_b_to_a();

// 10. Receive SPDM response on requester
let mut received_response = MessageBuf::new();
spdm_requester.receive_response(&mut received_response).unwrap();
```

## Complete SPDM Session Example

```rust
use core::cell::RefCell;
use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use spdm_lib::platform::transport::SpdmTransport;

#[test]
fn spdm_loopback_get_version() {
    // Setup loopback (no heap allocation)
    let packets_a = RefCell::new(PacketBuffer::new());
    let packets_b = RefCell::new(PacketBuffer::new());
    let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);

    let mut spdm_req = MctpSpdmTransport::new_requester(pair.client_a(), 42);
    let mut spdm_resp = MctpSpdmTransport::new_responder(pair.client_b());

    spdm_req.init_sequence().unwrap();
    spdm_resp.init_sequence().unwrap();

    // Requester: send GET_VERSION
    let mut req_buf = MessageBuf::new();
    // ... encode GET_VERSION message ...
    spdm_req.send_request(42, &mut req_buf).unwrap();

    pair.transfer_a_to_b();  // Deliver to responder

    // Responder: receive and process
    let mut recv_req = MessageBuf::new();
    spdm_resp.receive_request(&mut recv_req).unwrap();

    // ... verify request is GET_VERSION ...

    // Responder: send VERSION response
    let mut resp_buf = MessageBuf::new();
    // ... encode VERSION response ...
    spdm_resp.send_response(&mut resp_buf).unwrap();

    pair.transfer_b_to_a();  // Deliver to requester

    // Requester: receive response
    let mut recv_resp = MessageBuf::new();
    spdm_req.receive_response(&mut recv_resp).unwrap();

    // ... verify response ...
}
```

## Benefits for SPDM Testing

1. **Full Protocol Testing** - Test complete SPDM sequences without hardware
2. **Deterministic** - No timing issues or transport errors
3. **Fast** - In-memory operations are orders of magnitude faster
4. **Inspectable** - Can examine packet buffers between transfers
5. **Reproducible** - Same test will always produce same results
6. **No Heap Required** - Works on bare-metal embedded targets without allocator

## Debugging

You can inspect the packet buffers between transfers:

```rust
// After sending a request
println!("Packets in requester buffer: {}", packets_a.borrow().len());
for pkt in packets_a.borrow().iter() {
    println!("Packet: {:02x?}", pkt);
}

// Transfer
pair.transfer_a_to_b();

// Check what was delivered
println!("Packets in responder buffer: {}", packets_b.borrow().len());
```

## Clearing Buffers

For multi-step SPDM sequences, you may want to clear buffers between rounds:

```rust
// Send GET_VERSION
spdm_req.send_request(42, &mut get_version_req).unwrap();
pair.transfer_a_to_b();
pair.clear_a();  // Clear requester's buffer

// Receive and respond
spdm_resp.receive_request(&mut req).unwrap();
spdm_resp.send_response(&mut version_resp).unwrap();
pair.transfer_b_to_a();
pair.clear_b();  // Clear responder's buffer

// Continue with GET_CAPABILITIES...
```

## Alternative: Using roundtrip()

For simple request/response pairs, use `roundtrip()`:

```rust
// Send request
spdm_req.send_request(42, &mut req).unwrap();

// Process on responder
pair.transfer_a_to_b();
spdm_resp.receive_request(&mut recv_req).unwrap();
spdm_resp.send_response(&mut resp).unwrap();

// Receive response
pair.transfer_b_to_a();
spdm_req.receive_response(&mut recv_resp).unwrap();

// Clear for next round
pair.clear_a();
pair.clear_b();

// OR: just use roundtrip() if no processing needed between transfers
// (Not applicable in this case since we need to process between)
```
