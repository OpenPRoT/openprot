# USART Non-Blocking Try-Read with Notifications - Implementation Plan

## Overview

Extend the USART wire protocol and backend trait to support non-blocking read operations with event-based notifications. This enables clients to request data reads without blocking, receive a signal when data is available, and retrieve results asynchronously.

## Current Architecture Summary

**Wire Protocol** (`UsartOp`):
- `Configure`, `Write`, `Read`, `GetLineStatus`, `EnableInterrupts`, `DisableInterrupts`
- Max payload: 256 bytes

**Backend Trait** (`UsartBackend`):
- `configure()`, `write()`, `read()`, `line_status()`, `enable_interrupts()`, `disable_interrupts()`
- All operations are synchronous/blocking

**Server Dispatch**:
- `dispatch_request()` → synchronous operation → immediate response via `channel_respond()`
- Client uses `channel_transact()` which blocks until response arrives

**Notification Model** (current):
- Implicit wait queue via Pigweed `wait_group_add()`
- IRQ signals trigger server wakeup
- Server wakes and services next channel readable

## Design Goals

1. **Non-blocking semantics**: Client request returns immediately
2. **Event-driven**: Server signals client when operation completes
3. **No client-side polling**: Notification pulls client out of wait
4. **Bounded resource usage**: Finite pending request storage
5. **Backward compatible**: Existing synchronous ops continue working
6. **Multiplex-ready**: Enables MCTP and other services to process multiple clients concurrently

## Architecture Components

### 1. Wire Protocol Extensions

Add new operation to `UsartOp` enum in `drivers/usart/api/src/protocol.rs`:

```
UsartOp::TryRead = 0x07      // Non-blocking read attempt
UsartOp::GetAsyncResult = 0x08  // Retrieve result of async operation
```

**TryRead Request**:
- `arg0`: requested read size (bytes)
- `arg1`: reserved (0)
- Payload: empty
- Returns immediately with:
  - Success (0x00): data available in response payload
  - `WouldBlock` (0x06): no data yet, operation queued; server will signal client
  - `BufferTooSmall` (0x03): requested size invalid
  - Error: line error or backend fault

**GetAsyncResult Request**:
- `arg0`: operation ID (returned from TryRead when queued)
- `arg1`: max result size
- Payload: empty
- Returns:
  - Success: data + actual bytes read
  - Error: operation failed

### 2. Backend Trait Extensions

Add to `drivers/usart/api/src/backend.rs`:

```rust
pub enum AsyncReadState {
    Idle,
    Pending,
    Ready { bytes_read: usize },
    Error { error: BackendError },
}

pub trait UsartBackend {
    // Existing methods...
    
    /// Non-blocking read attempt.
    /// Returns Ok(bytes) if data available immediately.
    /// Returns Err(BackendError::Busy) if no data ready and caller should retry.
    /// Enables interrupts internally if needed.
    fn try_read(&mut self, out: &mut [u8]) -> Result<usize, BackendError>;
    
    /// Query pending async read state (for server tracking).
    /// Returns None if no pending operation, Some(state) if pending/ready/error.
    fn async_read_state(&self) -> Option<AsyncReadState>;
    
    /// Clear pending async operation after result retrieved.
    fn clear_async_read(&mut self) -> Result<(), BackendError>;
}
```

**Backend Implementation Contract**:
- `try_read()` must return immediately (never block)
- If data available: return Ok(n) with `n > 0` bytes copied
- If no data: return Err(Busy) and arm RX interrupt
- On IRQ: update internal state to Ready
- `async_read_state()` provides visibility to server

### 3. Server Runtime Changes

**File**: `drivers/usart/server/src/runtime.rs`

**New data structures** (in runtime or separate module):

```rust
pub struct PendingAsyncOp {
    pub client_channel: u32,
    pub operation_kind: OperationKind,
    pub buffer_addr: usize,  // Client's buffer location (for later DMA or marker)
    pub requested_size: usize,
}

pub enum OperationKind {
    TryRead { size: usize },
}

pub struct AsyncOpTracker {
    pending: Vec<PendingAsyncOp, 8>,  // Fixed-size queue (up to 8 concurrent)
}
```

**New runtime loop logic**:

1. **On IRQ**: When RX_DATA_AVAILABLE fires:
   - Query backend: `async_read_state()`
   - If `Ready { bytes_read }`: signal the waiting client(s)
   - Update pending op to complete

2. **On channel_read (TryRead)**:
   - Attempt `backend.try_read()`
   - If Ok(n): respond immediately with data
   - If Err(Busy): queue operation, return WouldBlock response, DON'T respond yet
   - Track: which client maps to which buffer

3. **On signal to client**: Use `syscall::set_peer_user_signal()` or equivalent:
   - Signal the initiating channel with custom signal code
   - Client wakes from wait
   - Client issues GetAsyncResult to retrieve data

4. **On GetAsyncResult (polling)**:
   - Check tracker for completed operation
   - If ready: format response + send data
   - Clear op from tracker
   - If not ready: return Timeout (client should re-poll or set up wait)

### 4. Server Dispatch Extensions

**File**: `drivers/usart/server/src/lib.rs`

Modify `dispatch_request()` signature:

```rust
pub fn dispatch_request<B: UsartBackend>(
    backend: &mut B,
    async_tracker: &mut AsyncOpTracker,  // NEW
    request: &[u8],
    response: &mut [u8],
    client_channel: u32,  // NEW - for signal routing
) -> DispatchResult;

pub enum DispatchResult {
    /// Send response immediately via channel_respond()
    Respond(usize),
    /// Don't respond yet; server will signal client when ready
    Queued,
    /// Error; send error response
    Error(UsartError),
}
```

**Handle TryRead**:
```rust
UsartOp::TryRead => {
    let req_size = hdr.arg0_value() as usize;
    
    // Attempt non-blocking read
    match backend.try_read(&mut response[HEADER_SIZE..]) {
        Ok(n) => {
            let hdr = UsartResponseHeader::success(n as u16);
            response[..HEADER_SIZE].copy_from_slice(...);
            DispatchResult::Respond(HEADER_SIZE + n)
        }
        Err(BackendError::Busy) => {
            // Queue the operation
            async_tracker.queue(PendingAsyncOp {
                client_channel,
                operation_kind: OperationKind::TryRead { size: req_size },
                ...
            });
            DispatchResult::Queued  // NO response yet
        }
        Err(e) => DispatchResult::Error(e.into()),
    }
}
```

**Handle GetAsyncResult**:
```rust
UsartOp::GetAsyncResult => {
    let op_id = hdr.arg0_value() as usize;
    
    match async_tracker.get_result(op_id) {
        Some(result @ AsyncResult::Ready { data, n_bytes }) => {
            // Copy result to response
            response[HEADER_SIZE..HEADER_SIZE + n_bytes].copy_from_slice(data);
            async_tracker.clear(op_id);
            DispatchResult::Respond(HEADER_SIZE + n_bytes)
        }
        Some(AsyncResult::Pending) => {
            // Still waiting; signal client again or return Timeout
            DispatchResult::Error(UsartError::Timeout)
        }
        None => {
            // Invalid operation ID
            DispatchResult::Error(UsartError::InvalidOperation)
        }
    }
}
```

### 5. Client API Extensions

**File**: `drivers/usart/client/src/lib.rs`

```rust
pub struct UsartClient {
    handle: u32,
    // Track ongoing async operations
    pending_async: Vec<AsyncPending, 4>,
}

pub struct AsyncPending {
    pub operation_kind: ClientOperationKind,
    pub signal_pending: bool,  // Waiting for completion signal
}

pub enum ClientOperationKind {
    TryRead { size: usize, result_buffer: &'static mut [u8] },
}

impl UsartClient {
    /// Initiate non-blocking read.
    /// Returns:
    /// - Ok(n) if data available immediately
    /// - Err(ClientError::WouldBlock) if queued for async completion
    /// - Other errors on failure
    pub fn try_read(&mut self, out: &mut [u8]) -> Result<usize, ClientError> {
        let mut req = [0u8; MAX_BUF_SIZE];
        let mut resp = [0u8; MAX_BUF_SIZE];

        let hdr = UsartRequestHeader::new(UsartOp::TryRead, out.len() as u16, 0, 0);
        req[..UsartRequestHeader::SIZE].copy_from_slice(...);

        // Non-blocking send: don't use channel_transact
        // Instead: channel_send() with timeout 0
        let resp_len = syscall::channel_transact(
            self.handle,
            &req[..UsartRequestHeader::SIZE],
            &mut resp,
            Instant::IMMEDIATE,  // KEY: non-zero timeout needed for polling
        )?;

        let hdr_resp = UsartResponseHeader::from_bytes(&resp[..UsartResponseHeader::SIZE])?;
        
        match hdr_resp.status() {
            UsartError::Success => {
                let n = hdr_resp.payload_len();
                out[..n].copy_from_slice(&resp[UsartResponseHeader::SIZE..UsartResponseHeader::SIZE + n]);
                Ok(n)
            }
            UsartError::Busy | UsartError::Timeout => {
                // Queue locally and wait for signal
                self.pending_async.push(AsyncPending {
                    operation_kind: ClientOperationKind::TryRead { size: out.len(), ... },
                    signal_pending: true,
                });
                Err(ClientError::WouldBlock)
            }
            e => Err(ClientError::ServerError(e)),
        }
    }

    /// Poll for completion of pending async operation.
    /// Returns Ok(n) if ready, Err(WouldBlock) if still pending.
    pub fn get_async_result(&mut self, op_id: usize, out: &mut [u8]) -> Result<usize, ClientError> {
        let mut req = [0u8; MAX_BUF_SIZE];
        let mut resp = [0u8; MAX_BUF_SIZE];

        let hdr = UsartRequestHeader::new(UsartOp::GetAsyncResult, op_id as u16, out.len() as u16, 0);
        req[..UsartRequestHeader::SIZE].copy_from_slice(...);

        let resp_len = syscall::channel_transact(
            self.handle,
            &req[..UsartRequestHeader::SIZE],
            &mut resp,
            Instant::MAX,  // Can block here since we know result should be ready
        )?;

        let hdr_resp = UsartResponseHeader::from_bytes(...)?;
        match hdr_resp.status() {
            UsartError::Success => {
                let n = hdr_resp.payload_len();
                out[..n].copy_from_slice(&resp[UsartResponseHeader::SIZE..]);
                Ok(n)
            }
            e => Err(ClientError::ServerError(e)),
        }
    }

    /// Wait for async operation completion or timeout.
    /// Typically called after set_client_signal_handler().
    pub fn wait_async(&self, timeout: Instant) -> Result<(), ClientError> {
        syscall::object_wait(self.wait_set(), Signals::USER_0, timeout)?;
        Ok(())
    }
}
```

### 6. Notification Mechanism

**Option A: Signal-based (recommended)**
- Server calls `syscall::set_peer_user_signal(client_channel, Signals::USER_0)`
- Client wakes from `syscall::object_wait(..., Signals::USER_0, ...)`
- Client then calls `get_async_result()` to retrieve data

**Option B: Callback-based (if userspace supports)**
- Register callback on client
- Server invokes callback (not standard for this architecture)

**Option C: Pigweed event-based**
- Use Pigweed `pw_sync` primitives if available

**Chosen: Option A (Signal-based)**
- Standard Pigweed/userspace IPC mechanism
- Requires: new syscall or use existing interrupt masking

---

## Implementation Phases

### Phase 1: Wire Protocol & Backend Trait (Low Risk)
- Add `TryRead`, `GetAsyncResult` to `UsartOp`
- Add new errors: `WouldBlock` (or repurpose `Busy`)
- Add `try_read()`, `async_read_state()`, `clear_async_read()` to `UsartBackend`
- No server or client changes yet
- **Validation**: Compiles, no semantic changes

### Phase 2: Backend Implementation (Medium Risk)
- AST10x0 backend (`target/ast10x0/backend/usart/src/lib.rs`):
  - Implement `try_read()` with immediate return
  - Track pending read in backend state
  - Update `async_read_state()` on IRQ
- **Validation**: Backend compiles, try_read returns Busy when no data

### Phase 3: Server Dispatch & Runtime (High Risk)
- Create `AsyncOpTracker` queue structure
- Modify `dispatch_request()` to return `DispatchResult`
- Add TryRead and GetAsyncResult handling in dispatch
- Modify runtime loop to check for IRQ completion and signal clients
- **Validation**: Server compiles, basic try_read flows through

### Phase 4: Client API (Medium Risk)
- Add `try_read()` to `UsartClient`
- Add `get_async_result()` polling method
- Add `wait_async()` signal handler
- **Validation**: Client compiles, can issue try_read

### Phase 5: Integration & Testing (Highest Risk)
- Test harness for non-blocking scenario
- Verify signal delivery and polling
- Concurrent multi-client reads
- **Validation**: End-to-end async read flow works

---

## Key Decisions

### 1. Request/Response Protocol
- **Decision**: Keep existing transactional model for compatibility
- **Implication**: TryRead returns immediately; GetAsyncResult retrieves result
- **Alternative**: Separate notification channel (more complex)

### 2. Async State Storage
- **Decision**: Store in backend (AST10x0 driver owns state)
- **Implication**: Scales to multiple concurrent reads per client
- **Alternative**: Server tracker (decouples backend concern but more coordination)

### 3. Signal Mechanism
- **Decision**: Use userspace `set_peer_user_signal()` for notifications
- **Implication**: Client opts into wait_async() to be notified
- **Alternative**: Implicit signal (but requires understanding of signal dispatch)

### 4. Backward Compatibility
- **Decision**: Existing `read()` remains unchanged; new `try_read()` is additive
- **Implication**: Clients choose blocking or non-blocking
- **Alternative**: Replace `read()` with try_read everywhere (breaking change)

### 5. Buffer Management
- **Decision**: Client owns buffer; server copies data into response payload
- **Implication**: Limited by MAX_PAYLOAD_SIZE (256 bytes per response)
- **Alternative**: Shared memory region (more complex, requires coordination)

---

## Risk Analysis

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Async tracker overflow (>8 pending ops) | Medium | Fixed-size queue + return Busy if full |
| Signal delivery timing | Medium | Clear testing; verify signal_pending flag |
| Data consistency if IRQ fires mid-dispatch | High | Use atomic flags or interrupt masking in backend |
| Backward compatibility breakage | Low | Keep synchronous `read()` unchanged |
| Payload size limits (256B) | Low | Acceptable for USART line speeds; documented |

---

## Testing Strategy

### Unit Tests (dispatch logic)
- `test_try_read_immediate_data()`: Data available → immediate response
- `test_try_read_no_data()`: No data → WouldBlock response
- `test_try_read_then_async_result()`: Queue then retrieve flow

### Integration Tests (full stack)
- `test_nonblocking_read_single_client()`: One client, async read, signal delivery
- `test_nonblocking_read_concurrent()`: Multiple clients with interleaved reads
- `test_try_read_on_line_error()`: Try read with frame error detection
- `test_async_timeout()`: Client timeout waiting for signal

### Performance Tests
- Latency: Time from IRQ to signal delivery
- Throughput: Max bytes/sec with async vs. sync reads
- Resource usage: Pending op queue overhead

---

## Documentation Updates

- [ ] Update `drivers/usart/api/API.md` with TryRead / GetAsyncResult
- [ ] Add examples in `drivers/usart/client/` for async pattern
- [ ] Update server runtime documentation with async loop flow
- [ ] Publish migration guide: "From blocking to non-blocking reads"

---

## Success Criteria

1. ✅ `try_read()` returns immediately (never blocks caller)
2. ✅ Client receives signal when data available
3. ✅ No regression in synchronous `read()` behavior
4. ✅ Backward compatible: old clients continue working
5. ✅ Multiple clients can have concurrent pending reads
6. ✅ Up to 8 concurrent async operations supported
7. ✅ All integration tests pass on virt_ast10x0

---

## Future Enhancements (Out of Scope)

- [ ] Extend to `TryWrite`/async write path
- [ ] Increase MAX_PAYLOAD_SIZE beyond 256B via fragmentation
- [ ] DMA-backed zero-copy reads (requires shared memory)
- [ ] Timeout-aware `try_read_timeout(timeout)` variant
- [ ] Callback-based completion (if userspace event system evolves)
