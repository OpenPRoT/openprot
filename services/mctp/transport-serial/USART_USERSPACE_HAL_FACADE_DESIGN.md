# USART Userspace HAL Facade Design for MCTP Serial

## 1. Problem Statement

`services/mctp/transport-serial` must communicate through the userspace USART
service boundary, not through a platform-specific in-process UART peripheral.

Today, the transport and sender path still rely on assumptions that are either:
- platform-specific (direct UART object access), or
- not explicit in the userspace API contract (bounded read semantics, TX drain).

We need a stable facade that hides out-of-process IPC details while preserving
serial transport semantics required by MCTP framing.

## 2. Goals

- Keep MCTP transport platform-agnostic and process-topology-agnostic.
- Expose a minimal, transport-focused serial contract.
- Keep API semantics compatible with userspace IPC constraints.
- Enable deterministic unit tests with a mock serial endpoint.
- Avoid requiring per-byte notifications.

## 3. Non-Goals

- Mirror all of embedded-hal serial APIs 1:1.
- Redesign the full USART protocol in one step.
- Introduce target-specific behavior into transport crate APIs.

## 4. Design Principles

1. Chunk/event-driven, not per-byte eventing.
2. Explicit bounded waiting APIs for RX/TX state transitions.
3. Explicit capability contracts (e.g., max write chunk).
4. Transport depends on traits; process/IPC is hidden in adapters.

## 5. Facade API

### 5.1 Data Plane Trait

```rust
pub trait SerialPort {
    type Error;

    // TX
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
    fn flush_tx(&mut self, timeout_ms: u32) -> Result<(), Self::Error>;
    fn max_write_chunk(&self) -> usize;

    // RX
    fn read(&mut self, out: &mut [u8]) -> Result<usize, Self::Error>;
    fn read_with_timeout(
        &mut self,
        out: &mut [u8],
        timeout_ms: u32,
    ) -> Result<usize, Self::Error>;
    fn wait_readable(&mut self, timeout_ms: u32) -> Result<bool, Self::Error>;

    // Health/Error visibility
    fn line_status(&mut self) -> Result<LineStatus, Self::Error>;
}
```

### 5.2 Optional Control Plane Trait

```rust
pub trait SerialControl {
    type Error;

    fn configure_serial(&mut self, cfg: SerialConfig) -> Result<(), Self::Error>;
    fn enable_interrupts(&mut self, mask: IrqMask) -> Result<(), Self::Error>;
    fn disable_interrupts(&mut self, mask: IrqMask) -> Result<(), Self::Error>;
}
```

This split is optional. If preferred, these methods can live on `SerialPort`.

## 6. Adapter Model

### 6.1 `IpcUsartPort`

Implementation over userspace USART client.

Responsibilities:
- Encode/decode IPC calls.
- Map IPC/server errors into facade error type.
- Honor bounded timeout semantics.
- Expose declared max write chunk.

### 6.2 `MockSerialPort`

Test adapter used by transport unit tests.

Responsibilities:
- Deterministic scripted RX/TX behavior.
- Timeout and readable event simulation.
- Error injection (line status, timeout, busy).

### 6.3 Optional `DirectUartPort`

Only for specialized bring-up/testing where direct peripheral access is needed.
This is not required for production userspace transport.

## 7. MCTP Transport Integration

`transport-serial` depends on `SerialPort` trait only.

TX flow:
1. MCTP fragmenter produces framed chunks.
2. Transport writes chunk(s) through `SerialPort::write`.
3. Transport calls `flush_tx(timeout)` at frame boundaries where required.

RX flow:
1. Wait using `wait_readable(timeout)`.
2. Read available bytes in chunks.
3. Feed bytes into MCTP serial decoder.
4. Forward decoded packets to server inbound path.

No per-byte notification requirement is introduced.

## 8. Required USART Userspace API Additions

Minimum additions to support the facade:

1. `read_with_timeout` (or `try_read` equivalent).
2. `flush_tx` / `wait_tx_idle`.
3. `line_status` userspace wrapper.
4. `enable_interrupts` / `disable_interrupts` userspace wrappers.
5. Explicit `max_write_chunk` contract (or larger payload cap with documented bound).
6. `wait_readable(timeout)` event primitive.

## 9. Error Model

Facade error should preserve category and origin:

```rust
pub enum SerialPortError {
    Timeout,
    WouldBlock,
    Busy,
    BufferTooSmall,
    LineStatus(LineStatus),
    Transport,
    Internal,
}
```

Mapping rules:
- Server/IPC timeout -> `Timeout`.
- No data in non-blocking mode -> `WouldBlock`.
- USART line fault indicators -> `LineStatus(...)`.
- Unknown/status mismatch -> `Internal`.

## 10. Sizing and Throughput Contract

USART userspace protocol currently advertises finite payload capacity.
The facade must avoid hidden truncation by requiring one explicit policy:

- `max_write_chunk()` returns guaranteed accepted bytes per call, or
- protocol payload cap is raised and documented as a hard minimum.

Transport uses this bound to split framed writes safely.

## 11. Compatibility and Migration Plan

### Phase 1: Introduce facade types and trait
- Add `SerialPort` trait crate/module.
- Add `IpcUsartPort` adapter over existing client methods.

### Phase 2: Add missing client wrappers/APIs
- Add line status + interrupt wrappers.
- Add bounded read and TX drain semantics.
- Add write chunk capability query/constant.

### Phase 3: Switch transport to trait-only dependency
- Refactor sender/receiver to use `&mut dyn SerialPort` or generic `P: SerialPort`.
- Remove direct peripheral coupling from transport crate.

### Phase 4: Tests
- Add transport tests with `MockSerialPort`.
- Add integration test with userspace USART service.

## 12. Open Questions

1. Should `wait_readable` be level-triggered or edge-triggered at API contract level?
2. Should `read_with_timeout` return partial data on timeout or strict timeout error?
3. Do we require separate `drain_tx_fifo` and `drain_tx_shift` semantics, or one `flush_tx`?
4. Is line status latched-clear behavior needed explicitly in API?

## 13. Decision Summary

- Yes, expose a HAL-like facade to hide out-of-process USART transport details.
- Keep it transport-focused, not full embedded-hal parity.
- Use event/chunk semantics, not per-byte notifications.
- Make sizing and timeout semantics explicit in the facade contract.
