# openprot-mctp-transport-i2c

I2C transport binding for the MCTP server.

## Overview

This crate implements MCTP-over-I2C transport. It provides the `Sender` implementation for outbound packets and a receiver/decoder for inbound target-mode frames. It uses the drivers/i2c userspace stack for transport.

## Key Types

- `I2cSender<C>` — implements `mctp_lib::Sender` for I2C; handles fragmentation, encoding, and PEC via `mctp_lib::i2c::MctpI2cEncap`
- `MctpI2cReceiver` — decodes inbound I2C target-mode frames into MCTP packets

## Dependencies

- `openprot-mctp-api` — API traits
- `i2c_api` (drivers/i2c) — protocol and transport seam types
- `mctp-lib` — `Sender` trait, I2C encapsulation/decapsulation
- `mctp` — core MCTP types
- `embedded-hal` — hardware abstraction
- `heapless` — `no_std` collections
