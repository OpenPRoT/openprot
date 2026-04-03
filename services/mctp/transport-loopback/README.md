# MCTP Transport Loopback

In-memory loopback transport for MCTP testing using fixed-size buffers (no dynamic allocation).

## Overview

This crate provides a formalized loopback transport for MCTP that enables
two endpoints to communicate entirely in-memory without any physical transport
(I2C, PCIe, etc.).

**Key feature:** Uses fixed-size buffers to mirror the memory behavior of
`transport-i2c`, making it compatible with `no_std` embedded environments
without requiring a global allocator.

This is useful for:

- Unit and integration testing of MCTP applications
- Testing SPDM over MCTP without hardware
- Developing and testing MCTP-based protocols in pure Rust
- Running tests on bare-metal targets (AST1060, etc.)

## Architecture

The loopback transport consists of:

1. **PacketBuffer** - Fixed-size ring buffer for storing packets (max 16 packets of 255 bytes each)
2. **LoopbackPair** - A pair of connected MCTP endpoints
3. **LoopbackClient** - An `MctpClient` implementation for each endpoint
4. **BufferSender** - Captures outbound packets into a `PacketBuffer`
5. **transfer()** - Moves packets from one endpoint to another

## Memory Behavior

Mirroring `transport-i2c`:

- **Fixed-size buffers:** All packet storage uses stack-allocated arrays
- **No dynamic allocation:** Compatible with `no_std` without requiring `alloc`
- **Compile-time bounds:** Maximum 16 packets of 255 bytes each per endpoint
- **No Vec, no Box, no heap:** Can run on bare-metal AST1060-EVB

## Usage

```rust
use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
use openprot_mctp_api::MctpClient;
use core::cell::RefCell;

// Create packet buffers (stack-allocated, no heap required)
let packets_a = RefCell::new(PacketBuffer::new());
let packets_b = RefCell::new(PacketBuffer::new());

// Create a loopback pair with EIDs 8 and 42
let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);

// Get client handles for each endpoint
let client_a = pair.client_a();
let client_b = pair.client_b();

// Use clients normally
let handle_a = client_a.req(42).unwrap();
client_a.send(Some(handle_a), 1, None, None, false, b"hello").unwrap();

// Transfer packets from A to B
pair.transfer_a_to_b();

// Receive on B
let handle_b = client_b.listener(1).unwrap();
let mut buf = [0u8; 256];
let meta = client_b.recv(handle_b, 0, &mut buf).unwrap();
assert_eq!(&buf[..meta.payload_size], b"hello");
```

## Design

This transport uses the same memory pattern as `transport-i2c`:

**transport-i2c sender:**
```rust
let mut pkt = [0u8; mctp_lib::serial::MTU_MAX];  // Stack-allocated
```

**transport-loopback:**
```rust
pub struct PacketBuffer {
    packets: [[u8; 255]; 16],  // Stack-allocated
    lengths: [usize; 16],
    count: usize,
}
```

Both avoid dynamic allocation entirely, making them suitable for embedded
environments without heap allocation.

## Differences from Original Version

The original `transport-loopback` used `Vec<Vec<u8>>` which required:
- `extern crate alloc`
- `#[global_allocator]`
- Heap memory

This version uses fixed-size arrays which:
- ✅ Works in pure `no_std` without `alloc`
- ✅ No global allocator needed
- ✅ Can run on AST1060-EVB without custom allocators
- ✅ Matches memory semantics of `transport-i2c`
- ⚠️ Limited to 16 buffered packets per endpoint (configurable constant)

## Constants

```rust
pub const MAX_PACKET_SIZE: usize = 255;        // MCTP MTU
pub const MAX_BUFFERED_PACKETS: usize = 16;    // Max packets per endpoint
```

To change packet buffer depth, modify `MAX_BUFFERED_PACKETS` in `lib.rs`.

## Example with SPDM

See `examples/spdm_usage.md` for full SPDM-over-MCTP testing examples.

Quick example:

```rust
use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use core::cell::RefCell;

// Create loopback infrastructure (no heap required)
let packets_a = RefCell::new(PacketBuffer::new());
let packets_b = RefCell::new(PacketBuffer::new());
let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);

// Create SPDM transports
let mut spdm_requester = MctpSpdmTransport::new_requester(pair.client_a(), 42);
let mut spdm_responder = MctpSpdmTransport::new_responder(pair.client_b());

// Initialize
spdm_requester.init_sequence().unwrap();
spdm_responder.init_sequence().unwrap();

// Test SPDM protocol messages...
```

## Testing

The implementation includes comprehensive unit tests:

```bash
# Run tests (uses std for test harness)
bazel test //services/mctp/transport-loopback:transport_loopback_test
```

Tests verify:
- ✅ PacketBuffer operations (push, iter, clear)
- ✅ Basic unidirectional message transfer
- ✅ Bidirectional request/response flow
- ✅ Roundtrip helper method

## License

Licensed under the Apache-2.0 license.
