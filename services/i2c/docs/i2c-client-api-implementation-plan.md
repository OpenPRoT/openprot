# I2C Client API Implementation Plan

**Version:** 0.1.0  
**Date:** February 15, 2026  
**Status:** Planning  

---

## Overview

This document outlines the implementation plan for the I2C Client API as specified
in [i2c-client-api.md](./i2c-client-api.md).

---

## Phase 1: Foundation Types

**Files:** `services/i2c/api/src/address.rs`, `services/i2c/api/src/error.rs`

### 1.1 AddressError enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressError {
    OutOfRange(u8),
    Reserved(u8),
}
```

- `Display` impl for error messages
- `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` derives

### 1.2 I2cAddress newtype

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I2cAddress(u8);
```

Methods:
- `new(addr: u8) -> Result<Self, AddressError>` — validate 0x08-0x77
- `new_unchecked(addr: u8) -> Self` — for reserved addresses
- `value(self) -> u8` — raw accessor
- `write_address(self) -> u8` — `(addr << 1) | 0`
- `read_address(self) -> u8` — `(addr << 1) | 1`
- `TryFrom<u8>` impl

Unit tests:
- Valid range acceptance
- Reserved range rejection (0x00-0x07, 0x78-0x7F)
- Out of range rejection (> 0x7F)
- Unchecked escape hatch

### 1.3 I2cError struct

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cError {
    pub code: ResponseCode,
    pub kind: Option<I2cErrorKind>,
}
```

- `impl embedded_hal::i2c::Error` with `kind()` mapping
- `impl Display` for human-readable messages
- Helper constructors: `from_code()`, `from_kind()`

### 1.4 ResponseCode enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseCode {
    Success = 0,
    NoDevice = 1,
    NackData = 2,
    ArbitrationLost = 3,
    BusStuck = 4,
    Timeout = 5,
    InvalidBus = 6,
    InvalidAddress = 7,
    BufferTooSmall = 8,
    BufferTooLarge = 9,
    NotInitialized = 10,
    Busy = 11,
    Unauthorized = 12,
    IoError = 13,
    ServerError = 14,
}
```

- `from_u8(u8) -> Option<Self>` for wire decoding

### 1.5 I2cErrorKind enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cErrorKind {
    Bus,
    ArbitrationLoss,
    NoAcknowledge(NoAcknowledgeSource),
    Overrun,
    Other,
}
```

- Re-export `embedded_hal::i2c::NoAcknowledgeSource`

---

## Phase 2: Core Traits

**Files:** `services/i2c/api/src/client.rs`, `services/i2c/api/src/operation.rs`

### 2.1 BusIndex newtype

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusIndex(u8);
```

- `new(index: u8) -> Self`
- `value(self) -> u8`
- Constants: `BUS_0`, `BUS_1`, `BUS_2`

### 2.2 Operation enum

```rust
#[derive(Debug)]
pub enum Operation<'a> {
    Write(&'a [u8]),
    Read(&'a mut [u8]),
}
```

### 2.3 I2cClient trait

```rust
use embedded_hal::i2c::ErrorType;

pub trait I2cClient: ErrorType {
    fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error>;

    fn transaction(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error>;
}
```

### 2.4 I2cClientBlocking trait

```rust
pub trait I2cClientBlocking: I2cClient {
    fn write(...) -> Result<(), Self::Error>;
    fn read(...) -> Result<usize, Self::Error>;
    fn read_register(register: u8, buffer: &mut [u8]) -> Result<usize, Self::Error>;
    fn write_register(register: u8, value: &[u8]) -> Result<(), Self::Error>;
    fn probe(...) -> Result<bool, Self::Error>;
}

impl<T: I2cClient> I2cClientBlocking for T {}
```

**Note:** `write_register` concatenates register + value into single contiguous write.

---

## Phase 3: Target Mode

**File:** `services/i2c/api/src/target.rs`

### 3.1 TargetMessage struct

```rust
#[derive(Debug, Clone)]
pub struct TargetMessage {
    pub source_address: I2cAddress,
    data: [u8; 255],
    len: u8,
}
```

Methods:
- `data() -> &[u8]`
- `len() -> usize`
- `is_empty() -> bool`
- `Default` impl

### 3.2 I2cTargetClient trait

```rust
use embedded_hal::i2c::ErrorType;

pub trait I2cTargetClient: ErrorType {
    fn configure_target_address(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
    ) -> Result<(), Self::Error>;

    fn enable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error>;

    fn disable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error>;

    fn wait_for_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
        timeout: Option<Duration>,
    ) -> Result<usize, Self::Error>;

    fn register_notification(
        &mut self,
        bus: BusIndex,
        notification_mask: u32,
    ) -> Result<(), Self::Error>;

    fn get_pending_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
    ) -> Result<usize, Self::Error>;
}
```

---

## Phase 4: Crate Structure

**Files:** `services/i2c/api/src/lib.rs`, `services/i2c/api/Cargo.toml`

### 4.1 lib.rs

```rust
#![no_std]

mod address;
mod client;
mod error;
mod operation;
mod target;

pub use address::{AddressError, I2cAddress};
pub use client::{BusIndex, I2cClient, I2cClientBlocking};
pub use error::{I2cError, I2cErrorKind, ResponseCode};
pub use operation::Operation;
pub use target::{I2cTargetClient, TargetMessage};

// Re-export for convenience
pub use embedded_hal::i2c::{ErrorType, NoAcknowledgeSource};
```

### 4.2 Cargo.toml

```toml
[package]
name = "i2c-api"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
description = "I2C client API traits for OpenPRoT"

[dependencies]
embedded-hal = "1.0"

[features]
default = []
std = []
mock = ["std"]

[dev-dependencies]
```

---

## Phase 5: Build Configuration

**File:** `services/i2c/api/BUILD.bazel`

```python
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "i2c_api",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "i2c_api",
    deps = [
        "@crates//:embedded-hal",
    ],
    visibility = ["//visibility:public"],
)

rust_test(
    name = "i2c_api_test",
    crate = ":i2c_api",
)
```

---

## Phase 6: Mock Client (Testing Support)

**File:** `services/i2c/api/src/mock.rs` (feature-gated with `mock`)

### 6.1 ExpectedCall struct

```rust
pub struct ExpectedCall {
    pub bus: BusIndex,
    pub address: I2cAddress,
    pub write_data: Vec<u8>,
    pub response: Vec<u8>,
    pub result: Result<usize, I2cError>,
}
```

### 6.2 MockI2cClient

```rust
pub struct MockI2cClient {
    expected_calls: Vec<ExpectedCall>,
    call_index: usize,
}

impl embedded_hal::i2c::ErrorType for MockI2cClient {
    type Error = I2cError;
}

impl I2cClient for MockI2cClient {
    // Verify calls match expectations
}
```

Builder API:
- `new() -> Self`
- `expect_write_read(...) -> &mut Self`
- `expect_transaction(...) -> &mut Self`
- `verify(&self)` — assert all expected calls were made

---

## Phase 7: IPC Client Stub

**Directory:** `services/i2c/client/`

### 7.1 Structure

```
services/i2c/client/
├── Cargo.toml
├── BUILD.bazel
└── src/
    └── lib.rs
```

### 7.2 I2cChannelClient

```rust
pub struct I2cChannelClient {
    server_channel: pw_channel::Channel,
}

impl embedded_hal::i2c::ErrorType for I2cChannelClient {
    type Error = I2cError;
}

impl I2cClient for I2cChannelClient {
    // Wire protocol implementation
}
```

Wire protocol:
- Request: `[op, bus, addr, write_len, write_data..., read_len]`
- Response: `[status, data...]`

**Dependencies:**
- `i2c-api` crate
- `pw_channel` (when available)

---

## Task Checklist

| # | Task | File | Est. | Status |
|---|------|------|------|--------|
| 1 | Create `address.rs` with `AddressError`, `I2cAddress` | `src/address.rs` | 1h | ☐ |
| 2 | Create `error.rs` with `I2cError`, `ResponseCode`, `I2cErrorKind` | `src/error.rs` | 1h | ☐ |
| 3 | Create `operation.rs` with `Operation` enum | `src/operation.rs` | 15m | ☐ |
| 4 | Create `client.rs` with `BusIndex`, `I2cClient`, `I2cClientBlocking` | `src/client.rs` | 2h | ☐ |
| 5 | Create `target.rs` with `TargetMessage`, `I2cTargetClient` | `src/target.rs` | 1.5h | ☐ |
| 6 | Create `lib.rs` with re-exports | `src/lib.rs` | 15m | ☐ |
| 7 | Create `Cargo.toml` | `Cargo.toml` | 15m | ☐ |
| 8 | Create `BUILD.bazel` | `BUILD.bazel` | 30m | ☐ |
| 9 | Add unit tests for address validation | `src/address.rs` | 30m | ☐ |
| 10 | Add unit tests for error conversions | `src/error.rs` | 30m | ☐ |
| 11 | Create `mock.rs` with `MockI2cClient` | `src/mock.rs` | 1h | ☐ |
| 12 | Create IPC client crate stub | `services/i2c/client/` | 1h | ☐ |

**Total estimate:** ~9 hours

---

## Dependency Graph

```
embedded-hal (external)
        │
        ▼
   ┌─────────┐
   │ i2c-api │  ◄── Phases 1-6
   └─────────┘
        │
        ▼
┌──────────────┐
│ i2c-client   │  ◄── Phase 7 (IPC implementation)
│ (pw_channel) │
└──────────────┘
        │
        ▼
┌──────────────┐
│ Application  │  (sensor drivers, MCTP, etc.)
└──────────────┘
```

---

## Validation Criteria

- [ ] `cargo build --no-default-features` succeeds (`no_std`)
- [ ] `cargo test` passes all unit tests
- [ ] `bazel build //services/i2c/api:i2c_api` succeeds
- [ ] `I2cError` implements `embedded_hal::i2c::Error`
- [ ] `I2cClient` uses `embedded_hal::i2c::ErrorType` as supertrait
- [ ] `write_register` sends single contiguous wire write (verified by mock)
- [ ] No `Vec`, `String`, or heap allocation in core crate (except `mock` feature)

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| `pw_channel` API not finalized | Abstract behind trait; stub implementation |
| `embedded-hal` 1.x breaking changes | Pin version in `Cargo.toml`, test on update |
| Target mode notification design | Align with existing `hal/blocking` patterns |
| IPC buffer size limits | Document 255-byte limit; provide direct `write`/`transaction` for larger |

---

## References

- [I2C Client API Design](./i2c-client-api.md)
- [Hubris I2C Architecture Review](./hubris-i2c-architecture-review.md)
- [embedded-hal I2C traits](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/)
- [hal/blocking/src/i2c_device.rs](../../../hal/blocking/src/i2c_device.rs)
- [hal/blocking/src/i2c_hardware.rs](../../../hal/blocking/src/i2c_hardware.rs)
