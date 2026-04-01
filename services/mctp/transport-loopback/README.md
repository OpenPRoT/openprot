# openprot-mctp-transport-loopback

Loopback transport binding for the MCTP server, enabling direct server-to-server communication without physical transport.

## Overview

This crate implements MCTP loopback transport for testing and development. It provides a shared buffer mechanism that allows two MCTP servers to communicate directly, replacing the I2C transport layer with an in-memory packet exchange.

This is particularly useful for:
- Unit testing MCTP protocols without hardware
- SPDM loopback configurations where requester and responder run on the same device
- Development and debugging of MCTP applications

## Key Types

- `LoopbackSender` — implements `mctp_lib::Sender` for outbound packets; writes to a shared buffer
- `LoopbackPair` — manages the bidirectional communication between two endpoints

## Architecture

Unlike I2C transport which requires encoding/decoding with headers and PEC, the loopback transport works with raw MCTP packets:

```
Server A → LoopbackSender → SharedBuffer → Server B.inbound()
Server B → LoopbackSender → SharedBuffer → Server A.inbound()
```

Each endpoint gets a `LoopbackSender` that writes to the peer's receive buffer. The application is responsible for polling and transferring packets between endpoints.

## Usage

```rust
use std::cell::RefCell;
use mctp::Eid;
use openprot_mctp_server::Server;
use openprot_mctp_transport_loopback::{LoopbackPair, LoopbackSender};

// Create a bidirectional loopback pair
let pair = LoopbackPair::new();

// Create senders for each endpoint
let sender_a = LoopbackSender::new(&pair.a_to_b);
let sender_b = LoopbackSender::new(&pair.b_to_a);

// Create two MCTP servers
let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));
let server_b: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(42), 0, sender_b));

// Send from A to B (using the MctpClient trait via a wrapper)
// ...

// Transfer packets A→B
while let Some(pkt) = pair.pop_a_to_b() {
    server_b.borrow_mut().inbound(&pkt).unwrap();
}

// Transfer packets B→A
while let Some(pkt) = pair.pop_b_to_a() {
    server_a.borrow_mut().inbound(&pkt).unwrap();
}
```

See `tests/loopback.rs` for complete examples including echo roundtrips.

## Dependencies

- `openprot-mctp-api` — API traits
- `mctp-lib` — `Sender` trait, fragmentation
- `mctp` — core MCTP types
- `heapless` — `no_std` collections
