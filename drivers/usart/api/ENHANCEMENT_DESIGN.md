# USART API Enhancement Design

## 1. Overview

This document proposes enhancements to `drivers/usart/api` so the API can
support richer userspace transports (including MCTP-over-serial) while
remaining platform-agnostic.

Current API strengths:
- Small wire protocol and clear error model.
- Backend trait already abstracts per-platform UART details.
- Interrupt enable/disable already modeled (`IrqMask`).

Current API limitations:
- No bounded-read or non-blocking read semantics.
- No TX drain/flush operation.
- `line_status` has no documented bit contract.
- No capability query (`max payload`, feature flags, etc.).
- `UsartConfig` is underspecified for full serial configuration use cases.

## 2. Goals

- Preserve existing protocol compatibility where possible.
- Add explicit APIs needed by chunk/event-driven transports.
- Keep backend contract generic and implementable on constrained targets.
- Avoid exposing process topology in this API layer.

## 3. Non-Goals

- Replacing the IPC transport format.
- Requiring per-byte notifications.
- Making this API equivalent to full embedded-hal serial API.

## 4. Current State (Reference)

Current wire ops (`UsartOp`):
- `Configure`
- `Write`
- `Read`
- `GetLineStatus`
- `EnableInterrupts`
- `DisableInterrupts`

Current backend trait (`UsartBackend`):
- `configure`
- `write`
- `read`
- `line_status`
- `enable_interrupts`
- `disable_interrupts`

## 5. Proposed Wire Protocol Extensions

### 5.1 New Ops

Add new operation IDs (example values shown):
- `TryRead = 0x07`
- `ReadWithTimeout = 0x08`
- `FlushTx = 0x09`
- `GetCapabilities = 0x0A`
- `ConfigureEx = 0x0B`
- `WaitReadable = 0x0C`

Notes:
- Existing op codes remain unchanged.
- Older clients/servers continue to interoperate with legacy ops.

### 5.2 Request/Response Semantics

`TryRead`
- Input: `arg0=max_bytes`.
- Output: success + payload with zero or more bytes.
- Never blocks.

`ReadWithTimeout`
- Input: `arg0=max_bytes`, `arg1=timeout_ms` (or payload u32 if needed).
- Output: payload on success, `Timeout` on timeout.

`FlushTx`
- Input: `arg0=timeout_ms`.
- Output: success when TX path is drained enough to preserve frame ordering.

`GetCapabilities`
- Output payload: compact capability struct (versioned) containing:
  - `max_payload`
  - `supports_try_read`
  - `supports_timeout_read`
  - `supports_flush_tx`
  - `supports_wait_readable`
  - `supports_configure_ex`

`ConfigureEx`
- Input payload: extended serial config structure.
- Keeps legacy `Configure` for backward compatibility.

`WaitReadable`
- Input: `arg0=timeout_ms`.
- Output: success + single-byte boolean (or zero-length success with status in flags).

## 6. Proposed Composable Backend Traits

### 6.1 Base Trait (unchanged)

Keep `UsartBackend` as the mandatory baseline for compatibility.

### 6.2 Capability Traits

Instead of one large extension trait, split optional behavior into focused traits.

```rust
pub trait UsartReadNonBlocking {
    fn try_read(&mut self, out: &mut [u8]) -> Result<usize, BackendError>;
}

pub trait UsartReadWithTimeout {
    fn read_with_timeout(
        &mut self,
        out: &mut [u8],
        timeout_ms: u32,
    ) -> Result<usize, BackendError>;
}

pub trait UsartTxDrain {
    fn flush_tx(&mut self, timeout_ms: u32) -> Result<(), BackendError>;
}

pub trait UsartReadableWait {
    fn wait_readable(&mut self, timeout_ms: u32) -> Result<bool, BackendError>;
}

pub trait UsartExtendedConfig {
    fn configure_ex(&mut self, config: UsartConfigEx) -> Result<(), BackendError>;
}

pub trait UsartCapabilityProvider {
    fn capabilities(&self) -> UsartCapabilities;
}
```

Rationale:
- Keeps each optional feature independently implementable.
- Allows lightweight backends to implement only what they can support.
- Simplifies testing and conformance per behavior.
- Avoids trait bloat and forced method stubs.

### 6.3 Capability Type

```rust
pub struct UsartCapabilities {
    pub max_payload: u16,
    pub supports_try_read: bool,
    pub supports_timeout_read: bool,
    pub supports_flush_tx: bool,
    pub supports_wait_readable: bool,
    pub supports_configure_ex: bool,
}
```

### 6.4 Optional Convenience Alias

For code that prefers a single bound, define a composed trait alias-like wrapper:

```rust
pub trait UsartBackendFull:
    UsartBackend
    + UsartReadNonBlocking
    + UsartReadWithTimeout
    + UsartTxDrain
    + UsartReadableWait
    + UsartExtendedConfig
    + UsartCapabilityProvider
{}

impl<T> UsartBackendFull for T where
    T: UsartBackend
        + UsartReadNonBlocking
        + UsartReadWithTimeout
        + UsartTxDrain
        + UsartReadableWait
        + UsartExtendedConfig
        + UsartCapabilityProvider
{}
```

## 7. Data Contract Clarifications

### 7.1 Line Status Contract

Define portable `LineStatus` bits in API docs/constants:
- `RX_DATA_READY`
- `TX_EMPTY`
- `TX_IDLE`
- `OVERRUN`
- `PARITY_ERR`
- `FRAMING_ERR`
- `BREAK_DETECTED`

This makes `GetLineStatus` useful across backends and clients.

### 7.2 Extended Configuration

Add:

```rust
pub struct UsartConfigEx {
    pub baud_rate: u32,
    pub parity: Parity,
    pub stop_bits: u8,
    pub data_bits: u8,
    pub flow_control: FlowControl,
}
```

And:

```rust
pub enum FlowControl {
    None,
    RtsCts,
}
```

## 8. Error Model

Continue using `UsartError` and `BackendError` mapping.

Guidance:
- Timeout-like wait/read failures -> `Timeout`.
- Immediate no-data on non-blocking path (`TryRead`) -> success with 0 bytes.
- Unsupported new operation -> `InvalidOperation`.

## 9. Backward Compatibility Strategy

1. Keep existing opcodes and structures untouched.
2. Add new opcodes only.
3. Add composable optional traits instead of modifying base trait.
4. Servers dispatch new ops only when the corresponding trait behavior is available.
5. Clients probe `GetCapabilities` and degrade gracefully.

## 10. Migration Plan

Phase 1: API additions
- Add new op enums and wire payload structs.
- Add `UsartCapabilities`, `UsartConfigEx`, `FlowControl`.
- Add composable optional traits (`UsartReadNonBlocking`, `UsartReadWithTimeout`, etc.).

Phase 2: Server support
- Update dispatcher to decode new ops.
- If backend lacks required capability trait for an op, return `InvalidOperation`.

Phase 3: Client support
- Add wrappers for `try_read`, `read_with_timeout`, `flush_tx`, `wait_readable`.
- Add optional capability probing and behavior selection.

Phase 4: Backend rollout
- Implement traits incrementally on AST10x0 backend first.
- Expand to other backends as available.

Phase 5: Transport adoption
- Update serial transport layers to use new bounded/event APIs.

## 11. Testing Strategy

- Unit tests for wire encoding/decoding of all new ops.
- Server dispatch tests for fallback behavior when extension is absent.
- Backend conformance tests per capability trait (timeout/read/flush/wait/config).
- Integration tests validating chunked receive and frame-safe transmit.

## 12. Open Questions

1. Should `ReadWithTimeout` return partial data on timeout or strict timeout error?
2. Should `WaitReadable` be level-triggered by contract?
3. Is hardware TX shift-register drain required for `FlushTx`, or FIFO drain sufficient?
4. Should `max_payload` in capabilities be protocol payload cap or effective backend cap?

## 13. Summary

The enhancement keeps existing USART API behavior stable while adding the
minimum primitives needed for robust userspace serial transports:
- bounded/non-blocking receive,
- explicit TX drain,
- capability discovery,
- richer serial configuration,
- documented status contracts.
