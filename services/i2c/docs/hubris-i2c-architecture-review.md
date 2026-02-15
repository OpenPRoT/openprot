# Hubris I2C Architecture Review

**Author:** Claude (AI Architectural Review)  
**Date:** February 15, 2026  
**Status:** Design Analysis for Pigweed Port  

---

## Executive Summary

The Hubris I2C design represents a **production-quality embedded systems architecture** that prioritizes correctness, hardware ownership, and portability. This review identifies strengths to preserve and areas where Rust ergonomics can be improved for the Pigweed port.

**Overall Grade: B+ (Strong Foundation)**

---

## 1. Modularity

**Grade: B+**

### Strengths

- **Clean 3-layer separation:**
  - `i2c_core` (portable hardware driver)
  - `drv-ast1060-i2c` (Hubris wrapper implementing `I2cHardware`)
  - `drv-openprot-i2c-server` (vendor-agnostic IPC server)

- **Trait-based abstraction:** The `I2cHardware` trait enables vendor-agnostic server logic

- **Compile-time hardware selection:** Feature flags (`#[cfg(feature = "ast1060")]`) select vendor implementations with zero runtime overhead

### Concerns

**Tight coupling in `I2cDevice`:** The client API embeds `TaskId`, `Controller`, `Port`, `Mux`, `Segment`, and `address` into a single struct. Every operation carries the entire routing context:

```rust
// Current design - everything bundled together
pub struct I2cDevice {
    pub task: TaskId,
    pub controller: Controller,
    pub port: PortIndex,
    pub segment: Option<(Mux, Segment)>,
    pub address: u8,
}
```

### Recommendation

Consider splitting `I2cDevice` into composable parts:

```rust
// Routing information (could be shared across devices)
pub struct I2cBus {
    pub task: TaskId,
    pub controller: Controller,
    pub port: PortIndex,
    pub segment: Option<(Mux, Segment)>,
}

// Validated address
pub struct I2cAddress(u8);

// Device operations take both
impl I2cBus {
    fn write_read(&self, addr: I2cAddress, ...) -> Result<...>;
}
```

---

## 2. Portability

**Grade: A-**

### Strengths

- **Zero OS dependencies:** `i2c_core` is `#![no_std]` with no alloc requirement
- **Clear initialization boundary:** Pre-kernel init in app `main.rs` vs. runtime driver operations
- **Ecosystem compatibility:** `embedded-hal` trait implementation in core driver

### Concerns

**ResponseCode leakage:** The `ResponseCode` enum has ~30 variants, some quite Hubris-specific:
- `BusLockedMux`
- `SegmentDisconnected`
- `MuxMissing`

These make sense in Hubris's multiplexer-heavy topology but add noise for simpler deployments.

### Recommendation

For Pigweed port:
1. Create a smaller, transport-agnostic error type in the API crate
2. Map to transport-specific codes (ResponseCode, pw::Status) at the IPC boundary
3. Keep the full taxonomy available for implementations that need it

---

## 3. Efficiency

**Grade: A**

### Strengths

- **`from_initialized()` pattern:** Avoids ~50 register writes per operation by assuming hardware was pre-configured

- **Zero-copy lease handling:** Server uses `sys_borrow_read`/`sys_borrow_write` for efficient data transfer

- **Appropriate mode selection:**
  - Master mode: Polling (simple, predictable timing)
  - Slave mode: Hardware IRQs via notifications (efficient)

### Analysis

The polling-based master mode is **correct** for most embedded use cases:
- Provides deterministic timing
- Avoids interrupt overhead for short transfers (~100-500µs per byte at 100kHz)
- Simpler state machine

Each `write_read()` creates a temporary `Ast1060I2c` struct. While cheap with `from_initialized()`, consider if this pattern should be maintained or if a more persistent driver handle is appropriate for Pigweed.

---

## 4. Long-term Maintenance

**Grade: B**

### Strengths

- Comprehensive documentation in design doc
- Clear migration phases with checklist
- Pre-kernel initialization documented per-register

### Concerns

**API surface explosion:** The client API has many method variants:

| Read Operations | Write Operations |
|-----------------|------------------|
| `read_reg` | `write` |
| `read_reg_into` | `write_read_reg` |
| `read` | `write_read_block` |
| `read_into` | `write_write` |
| `read_block` | `write_write_read_reg` |

This creates:
- Maintenance burden (each method has IPC marshalling)
- User confusion (which method do I use?)
- Testing overhead (all combinations need coverage)

**Zerocopy complexity:** Generic bounds create complex signatures:

```rust
fn read_reg<R, V>(&self, reg: R) -> Result<V, ResponseCode>
where
    R: IntoBytes + Immutable,
    V: IntoBytes + FromBytes,
```

Error messages become opaque when bounds aren't satisfied.

### Recommendation

Consider transaction builder pattern for Pigweed:

```rust
// More composable, fewer methods
client.transaction(address)
    .write(&reg_bytes)
    .read(&mut buffer)
    .execute()?;

// Or with operations list
client.execute(address, &mut [
    Operation::Write(&cmd),
    Operation::Read(&mut response),
])?;
```

---

## 5. Rust Ergonomics

**Grade: B-**

### Issues Identified

#### a) Raw address handling

```rust
// Current - raw u8, no compile-time validation
configure_slave_address(0x48)?;  // What if user passes 0x00?

// Better - newtype with validation
configure_slave_address(I2cAddress::try_from(0x48)?)?;
```

#### b) Fallible constructors not explicit

```rust
// Current - panics on invalid address in some paths
let device = I2cDevice::new(task, controller, port, None, 0x48);

// Better - builder with explicit error handling
let device = I2cDevice::builder()
    .controller(Controller::I2C1)
    .address(I2cAddress::new(0x48)?)
    .build(task)?;
```

#### c) Missing type-state patterns

```rust
// Current - runtime checks for state ordering
enable_slave_receive()?;  // Must call configure_slave_address first!

// Better - type-state ensures ordering at compile time
let configured: SlaveConfigured = client.configure_slave_address(0x1D)?;
let receiving: SlaveReceiving = configured.enable_receive()?;
```

#### d) Generic bounds accumulation

When multiple generic parameters have bounds, the trait bounds section grows:

```rust
fn write_write_read_reg<R, V>(
    &self,
    reg: R,
    first: &[u8],
    second: &[u8],
) -> Result<V, ResponseCode>
where
    R: IntoBytes + Immutable,
    V: IntoBytes + FromBytes,
```

---

## 6. Slave Mode Design

**Grade: A-**

### Strengths

- **Polling-based retrieval:** Avoids callback complexity and potential reentrancy issues
- **Fixed 255-byte buffer:** Matches I2C transaction limits, no dynamic allocation
- **Clear message format:** `[source_addr, length, data...]`

### Minor Concern

Large stack allocation in `get_slave_messages()`:

```rust
let mut buffer = [0u8; 1024];  // Large for embedded stacks
```

Consider:
- Making buffer size configurable via const generic
- Passing buffer as parameter
- Using smaller default with option to grow

---

## Summary: Recommendations for Pigweed Port

| Area | Current State | Recommendation |
|------|--------------|----------------|
| **Address handling** | Raw `u8` | `I2cAddress` newtype with validation ✓ |
| **Error taxonomy** | ~30 Hubris-specific codes | Smaller core errors, map at boundary |
| **API shape** | 11+ method variants | Transaction builder or operations list |
| **State management** | Runtime checks | Type-state for slave mode |
| **Generic bounds** | Complex trait bounds | Prefer concrete types where possible |
| **Client trait** | Monolithic | Split `I2cClient` + `I2cClientBlocking` ✓ |
| **Bus abstraction** | Bundled in device | Separate `BusIndex` type ✓ |

---

## Implementation Priority

### Phase 1: Core Types (Current)
- [x] `I2cAddress` - validated address newtype
- [x] `I2cError` / `ResponseCode` - error hierarchy
- [ ] `BusIndex` - bus identification
- [ ] `I2cClient` trait - client API contract

### Phase 2: Ergonomic Improvements
- [ ] Transaction builder pattern
- [ ] Type-state for slave mode lifecycle
- [ ] Reduced API surface

### Phase 3: Integration
- [ ] IPC implementation for Pigweed
- [ ] Map errors to `pw::Status`
- [ ] Server task implementation

---

## References

- [i2c-core-hubris-integration-design.md](../../../aspeed-rust/docs/i2c-core-hubris-integration-design.md) - Source architecture
- [drv-i2c-api](../../../hubris/drv/i2c-api/src/lib.rs) - Hubris client API
- [drv-i2c-types](../../../hubris/drv/i2c-types/src/lib.rs) - Hubris types
- [embedded-hal I2C traits](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/index.html)
