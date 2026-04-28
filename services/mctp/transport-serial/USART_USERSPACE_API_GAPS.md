# USART Userspace API Gaps for MCTP Serial Transport

## Purpose

This document lists the API gaps between what MCTP serial transport needs and
what the current userspace USART client exposes.

Scope:
- MCTP transport in `services/mctp/transport-serial`
- Current userspace USART client in `drivers/usart/client`

## Current USART Userspace Surface

The current userspace client exposes only:
- `configure(baud_rate)`
- `write(data)`
- `read(out)`

Also relevant:
- USART wire protocol `MAX_PAYLOAD_SIZE` is 256 bytes.
- Protocol supports `GetLineStatus`, `EnableInterrupts`, and `DisableInterrupts`,
  but these are not exposed by the userspace client wrapper yet.

## What MCTP Serial Transport Needs

MCTP serial transport needs a robust byte-stream interface for:
- Framed transmit (`MctpSerialHandler::send_sync(...)`) plus drain/flush behavior.
- Incremental receive and decode with bounded wait behavior.
- Error visibility for frame/parity/overrun conditions.
- Event-driven receive (or equivalent) to avoid blocking forever.
- Payload sizing/chunking that is compatible with framed packet sizes.

## Missing APIs (Priority Ordered)

### 1. Read timeout or non-blocking read (required)

Problem:
- Current `read(out)` path is effectively unbounded wait from the client side.
- MCTP service loops need bounded wait to multiplex IPC, timers, and serial RX.

Needed API:
- `read_with_timeout(out, timeout)` OR
- `try_read(out)` returning `WouldBlock` OR
- `read_exactly_available(out)` with immediate return semantics.

### 2. TX drain/flush visibility (required)

Problem:
- MCTP framing sender calls flush/drain semantics after frame emission.
- Current userspace client has no explicit "transmitter drained" API.

Needed API:
- `flush_tx(timeout)` or `wait_tx_idle(timeout)`.

### 3. Line/error status access (required)

Problem:
- MCTP receive path should be able to inspect and react to UART line errors
  (frame/parity/overrun) to avoid forwarding corrupt data.

Needed API:
- `line_status()` wrapper in userspace client (protocol operation already exists).
- Optional: `clear_error_status()` if required by hardware behavior.

### 4. Interrupt control wrappers (high value)

Problem:
- Protocol supports interrupt enable/disable, but client wrapper does not expose
  this. Event-driven RX currently has to poll.

Needed API:
- `enable_interrupts(mask)`
- `disable_interrupts(mask)`

### 5. RX-ready wait primitive (high value)

Problem:
- No explicit API to wait for readability in a bounded way from the USART
  service boundary.

Needed API:
- `wait_readable(timeout)` or equivalent event/notification bridge.

### 6. Payload-size compatibility for framed writes (required for robustness)

Problem:
- USART protocol payload cap is 256 bytes.
- MCTP serial frame chunks may exceed this cap depending on MTU/framing.
- Current transport layer must rely on implicit chunk sizing assumptions.

Needed API/options:
- Increase USART IPC payload cap, OR
- Expose a guaranteed `max_write_chunk()` capability, OR
- Provide an explicit `write_vectored/chunked` API contract.

### 7. Full serial configuration surface (optional but recommended)

Problem:
- Current configure API only sets baud rate.
- Serial transport may eventually require explicit framing configuration.

Needed API:
- `configure_serial({ baud, data_bits, parity, stop_bits, flow_control })`.

## Minimal API Set to Unblock MCTP

If we want the smallest increment that makes MCTP serial practical:

1. `read_with_timeout` (or `try_read`)
2. `wait_tx_idle` (or `flush_tx`)
3. `line_status`
4. `enable_interrupts`/`disable_interrupts`
5. One explicit write-size contract (`max_write_chunk` or larger payload cap)

## Suggested Implementation Order

1. Add userspace client wrappers for existing protocol ops:
   - `line_status`, `enable_interrupts`, `disable_interrupts`
2. Add bounded-read capability (`read_with_timeout` or `try_read`)
3. Add TX drain API (`wait_tx_idle`)
4. Add explicit write-size contract/capability query
5. Add richer serial configuration only if needed by integration tests
