# Design Review: `i2c-api` Wire Protocol Module (`wire.rs`)

**Date:** 2026-02-15  
**Module:** `services/i2c/api/src/wire.rs`  
**Reviewer:** AI-assisted review  
**Status:** Review complete — actionable findings

---

## 1. Purpose & Scope

`wire.rs` defines the binary IPC wire protocol for I2C operations. It provides:

- Operation code enum (`I2cOp`)
- Request header (8 bytes) and response header (4 bytes)
- Manual little-endian encoding/decoding (no external dependencies)
- Free-standing encode/decode helper functions
- `no_std` compatible, zero-copy decode

### Wire Layout

```text
Request (8 bytes header + payload):
┌────┬─────┬──────┬─────┬──────────┬──────────┐
│ op │ bus │ addr │ res │ write_len│ read_len │  + [write data]
│ 1B │ 1B  │ 1B   │ 1B  │  2B LE   │  2B LE   │
└────┴─────┴──────┴─────┴──────────┴──────────┘

Response (4 bytes header + payload):
┌──────┬─────┬──────────┐
│ code │ res │ data_len │  + [read data]
│ 1B   │ 1B  │  2B LE   │
└──────┴─────┴──────────┘
```

---

## 2. What's Done Well

| Aspect | Detail |
|--------|--------|
| **Compact layout** | 8-byte request / 4-byte response headers with explicit reserved bytes for alignment — efficient for IPC on constrained targets |
| **`const fn` constructors** | `I2cRequestHeader::write()`, `::read()`, etc. allow compile-time header construction for static request tables |
| **Explicit LE encoding** | Uses `to_le_bytes()` / `from_le_bytes()` throughout — no reliance on platform endianness |
| **Zero-copy decode** | `get_response_data()` / `get_request_payload()` return `&[u8]` slices into the caller's buffer with correct lifetime annotations (`'a`) |
| **Good test coverage** | Roundtrip tests for both header types, encode helpers, and `I2cOp::from_u8` edge cases |
| **Clear documentation** | ASCII wire diagram in module doc; doc comments on all public items |
| **No `unsafe`** | Entirely safe Rust |

---

## 3. Findings

### 3.1 HIGH — Raw `u8` Fields Bypass the Type System

**Location:** `I2cRequestHeader` (lines 80–90), `I2cResponseHeader` (lines 175–182)

**Problem:** The header structs store `op: u8`, `address: u8`, and `code: u8` rather than the validated domain types `I2cOp`, `I2cAddress`, and `ResponseCode` that the crate already defines. This means:

- A caller can construct `I2cRequestHeader { op: 255, .. }` — an invalid opcode with no compile-time or runtime error.
- The `address` field completely bypasses `I2cAddress` validation (reserved-range checks, 7-bit range enforcement).
- `I2cResponseHeader.code` can hold values not in `ResponseCode`; the `response_code()` method silently maps unknowns to `ServerError`.
- The `operation()` method returns `Option<I2cOp>`, proving the struct itself doesn't guarantee validity.

**Recommendation:** Store the typed enums/newtypes in the struct fields. Convert to/from `u8` only inside `to_bytes()` / `from_bytes()`:

```rust
pub struct I2cRequestHeader {
    pub op: I2cOp,           // was u8
    pub bus: BusIndex,       // was u8
    pub address: I2cAddress, // was u8
    pub write_len: u16,
    pub read_len: u16,
}

impl I2cRequestHeader {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WireError> {
        // ...
        let op = I2cOp::from_u8(bytes[0])
            .ok_or(WireError::InvalidOpcode(bytes[0]))?;
        // ...
    }
}
```

This makes illegal states unrepresentable at the struct level.

---

### 3.2 HIGH — Silent `as u16` Truncation

**Location:** Lines 280, 312, 370 — `data.len() as u16`

**Problem:** If `data.len() > 65535`, the cast silently wraps. The header will advertise a truncated length while the buffer contains more data, causing the receiver to misinterpret the payload boundary.

While unlikely on a small embedded target, this is a **correctness bug** — it violates the principle that encoding should either succeed faithfully or fail.

**Recommendation (option A — fail-safe):**

```rust
let write_len = u16::try_from(data.len()).ok()?;
```

**Recommendation (option B — debug guard):**

```rust
debug_assert!(data.len() <= u16::MAX as usize);
```

---

### 3.3 MEDIUM — `MAX_PAYLOAD_SIZE` Declared But Never Enforced

**Location:** Line 255 — `pub const MAX_PAYLOAD_SIZE: usize = 256;`

**Problem:** The constant is defined and documented but none of the `encode_*` functions check against it. A caller can encode a payload larger than 256 bytes; the serialization will succeed, but the receiver's fixed-size buffer will overflow or reject it.

**Recommendation:** Add validation at the top of each encode helper:

```rust
pub fn encode_write_request(buf: &mut [u8], bus: u8, address: u8, data: &[u8]) -> Option<usize> {
    if data.len() > MAX_PAYLOAD_SIZE {
        return None; // or Err(WireError::PayloadTooLarge)
    }
    // ...
}
```

---

### 3.4 MEDIUM — `Option` Return Type Loses Error Information

**Location:** All encode/decode functions

**Problem:** Every function returns `Option<...>`. Callers cannot distinguish between:
- Buffer too small
- Payload exceeds `MAX_PAYLOAD_SIZE`
- Invalid opcode on decode
- `u16` length overflow

This makes debugging and logging difficult in production.

**Recommendation:** Introduce a dedicated error enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Output buffer too small for the encoded message.
    BufferTooSmall,
    /// Payload exceeds MAX_PAYLOAD_SIZE.
    PayloadTooLarge,
    /// Unrecognized operation code during decode.
    InvalidOpcode(u8),
    /// Unrecognized response code during decode.
    InvalidResponseCode(u8),
    /// Input buffer too short to contain a complete header.
    Truncated,
}
```

Change return types from `Option<usize>` to `Result<usize, WireError>`.

---

### 3.5 MEDIUM — Encode Helpers Accept Raw `u8` Parameters

**Location:** Lines 268–330 — `encode_write_request()`, `encode_read_request()`, etc.

**Problem:** The free-standing helpers take `bus: u8, address: u8` as raw bytes. This defeats the type safety provided by `BusIndex` and `I2cAddress` elsewhere in the crate. A caller can easily swap the two arguments without a compiler warning.

**Recommendation:** Accept the domain types:

```rust
pub fn encode_write_request(
    buf: &mut [u8],
    bus: BusIndex,
    address: I2cAddress,
    data: &[u8],
) -> Result<usize, WireError> { ... }
```

---

### 3.6 LOW — Redundant Free-Standing Wrapper Functions

**Location:** Lines 331–358 — `decode_response_header()`, `decode_request_header()`, `get_response_data()`, `get_request_payload()`

**Problem:** These are trivial one-line delegates to existing methods:

```rust
pub fn decode_response_header(buf: &[u8]) -> Option<I2cResponseHeader> {
    I2cResponseHeader::from_bytes(buf)
}
```

They add API surface without adding value. Callers can (and some already do) call the methods directly.

**Recommendation:** Remove these wrappers. If a flat-function facade is desired, mark them `#[inline]` and add a doc note explaining why they exist.

---

### 3.7 LOW — Missing Encode Helpers for Declared Operations

**Location:** `I2cOp` enum vs. encode helpers

**Problem:** `I2cOp` declares seven variants, but encode helpers exist for only four:

| Op | Constructor | Encode helper |
|----|------------|---------------|
| `Write` | ✅ | ✅ `encode_write_request` |
| `Read` | ✅ | ✅ `encode_read_request` |
| `WriteRead` | ✅ | ✅ `encode_write_read_request` |
| `Transaction` | ❌ | ❌ |
| `Probe` | ✅ | ✅ `encode_probe_request` |
| `ConfigureSpeed` | ❌ | ❌ |
| `RecoverBus` | ✅ | ❌ |

Callers targeting `Transaction`, `ConfigureSpeed`, or `RecoverBus` must manually construct headers, which is error-prone and inconsistent.

**Recommendation:** Either add the missing helpers or document that these ops are not yet supported/implemented.

---

### 3.8 LOW — Reserved Byte Handling Is Asymmetric

**Location:** `from_bytes()` implementations

**Problem:** On decode, the reserved byte (`bytes[3]` in request, `bytes[1]` in response) is silently ignored. This is correct today, but if a future protocol version uses these bytes, old decoders will silently accept new-format messages without detecting the version mismatch.

**Recommendation:** Consider asserting `bytes[reserved] == 0` during decode, or reserving one bit as a version flag for forward compatibility.

---

## 4. Security Considerations

- **No `unsafe` code** — all operations are safe.
- **No panics on malformed input** — all decode paths return `Option`/`None` for undersized buffers.
- **Bounds-checked slicing** — all `buf[..]` accesses are preceded by length checks.
- **Risk:** The silent truncation (finding 3.2) could theoretically be used to craft a message where the header length disagrees with actual data, but this requires the sender to be compromised, which is outside the I2C IPC threat model.

---

## 5. Prioritized Action Items

| # | Priority | Finding | Effort |
|---|----------|---------|--------|
| 1 | **High** | 3.1 — Use typed enums/newtypes in header structs | Medium — requires updating all call sites |
| 2 | **High** | 3.2 — Replace `as u16` with `try_from` | Small — mechanical change |
| 3 | **Medium** | 3.3 — Enforce `MAX_PAYLOAD_SIZE` in encode functions | Small |
| 4 | **Medium** | 3.4 — Introduce `WireError` enum | Medium — touches return types |
| 5 | **Medium** | 3.5 — Accept `BusIndex`/`I2cAddress` in helpers | Small — depends on 3.1 |
| 6 | **Low** | 3.6 — Remove redundant wrappers | Trivial |
| 7 | **Low** | 3.7 — Add missing encode helpers | Small |
| 8 | **Low** | 3.8 — Validate reserved bytes | Trivial |

---

## 6. Conclusion

The module is functional, safe, well-documented, and appropriate for its `no_std` IPC context. The primary weakness is that the wire layer bypasses the type safety the rest of the crate carefully establishes — accepting and storing raw `u8` where validated domain types exist. Fixing findings 3.1 and 3.2 would bring the wire protocol in line with the crate's overall design philosophy of making illegal states unrepresentable. The remaining findings are incremental improvements.
