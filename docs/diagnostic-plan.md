# Diagnostic Plan: `pending recv: inbox still empty after transfer`

## Symptom

```
[DBG] try_service_pending: chan=1 wg=2 ud=1 pending_handle=0
[ERR] pending recv: inbox still empty after transfer on chan 1
[ERR] FAILURE: Failed to process VERSION resp[...]
```

`try_service_pending` is called with a parked `Recv` for `server_resp`
(chan=MCTP_RESP=1, handle=0 = listener AppCookie), but `server_resp.try_recv(Handle(0))`
returns `None`. The GET_VERSION packet should be in the inbox by this point.

---

## What We Know

### Cookie / Handle Encoding (verified from mctp-lib source)

| IPC call | Handle returned | AppCookie | Maps to |
|---|---|---|---|
| `server_resp.listener(0x05)` | `Handle(0)` | `AppCookie(0)` | `listeners[0]` |
| `server_req.req(eid=42)` | `Handle(8)` | `AppCookie(8)` | `requests[0]` (offset by MAX_LISTENERS=8) |

### mctp-lib `inbound` Routing

- `Tag::Owned` (incoming request) → matches listener by `msg.typ` → `msg.retain()` with `listener_cookie_from_index(i)`
- `Tag::Unowned` (incoming response) → matches request by cookie in packet → `msg.retain()`
- Silently drops if no match (returns `Ok(())`)

### Expected Packet Flow for GET_VERSION

```
requester.send_request(dst=42, msg_type=0x05, tag=None)
  → server_req.send(handle=Some(Handle(8)), eid=Some(42), tag=None)
  → stack.send_vectored(eid=Some(42), ...)
  → lookup_request(AppCookie(8)) → ReqHandle { eid: EID(42) }
  → fragment → BufferSender → packets_req

transfer_and_clear(packets_req → server_resp)
  → server_resp.inbound(pkt)
  → stack.receive(pkt) → MctpMessage { dest=EID(42), tag=Tag::Owned, typ=0x05 }
  → own_eid check: EID(42) == EID(42) ✓
  → match listener: listeners[0] == Some(MsgType(0x05)) ✓
  → msg.retain() with AppCookie(0) → stored
  → server_resp.try_recv(Handle(0)) → Some(message) ✓
```

If `try_service_pending` finds the inbox empty, one of these steps failed silently.

---

## Diagnostic Changes

All changes in `target/ast1060-evb/spdm-req-resp-test/mctp_loopback_server.rs`.

### 1. Instrument `transfer_and_clear`

Log how many packets are transferred and whether `inbound` succeeds:

```rust
fn transfer_and_clear<S: openprot_mctp_server::Sender, const N: usize>(
    packets: &RefCell<PacketBuffer>,
    dest: &mut openprot_mctp_server::Server<S, N>,
) {
    let pkts = packets.borrow();
    pw_log::debug!("transfer_and_clear: {} packet(s)", pkts.len() as u32);
    for pkt in pkts.iter() {
        if let Err(e) = dest.inbound(pkt) {
            pw_log::error!("transfer_and_clear: inbound failed: {}", e.code as u32);
        }
    }
    drop(pkts);
    packets.borrow_mut().clear();
}
```

**What this reveals:**
- `0 packet(s)` → `packets_req` was empty when `transfer_and_clear` ran. The Send IPC
  either didn't produce a packet or `packets_req` was cleared before this call.
- `inbound failed` → mctp-lib dropped the packet (bad EID, no listener, etc.).
- `1 packet(s)` + no error → packet was delivered; the issue is in `try_recv`.

### 2. Log the handle inside the Recv intercept

Before calling `try_recv` in the Recv fast path:

```rust
// In both the MCTP_REQ and MCTP_RESP Recv intercept blocks:
pw_log::debug!(
    "Recv intercept: ud={} handle={} has_pending_resp={} has_pending_req={}",
    ev.user_data as u32,
    h.0 as u32,
    pending_recv_resp.is_some() as u32,
    pending_recv_req.is_some() as u32,
);
```

**What this reveals:**
- Confirms which handle the Recv IPC is requesting (should be `0` for listener, `8` for req).
- Shows pending state at the moment of the intercept.

### 3. Log `try_recv` result in `try_service_pending`

Replace the existing error log with a richer one:

```rust
let resp_len = if let Some(meta) = server.try_recv(p.handle, recv_buf) {
    pw_log::debug!(
        "try_service_pending: try_recv(handle={}) hit: type={} size={}",
        p.handle.0 as u32, meta.msg_type as u32, meta.payload_size as u32
    );
    ...
} else {
    pw_log::error!(
        "try_service_pending: try_recv(handle={}) miss on chan={}",
        p.handle.0 as u32, chan_handle as u32
    );
    ...
};
```

### 4. Add `try_recv` probe after each `transfer_and_clear` (non-Recv dispatch path)

After the existing `try_service_pending` call in the non-Recv dispatch path,
add a debug log of how many pending recvs there were:

```rust
pw_log::debug!(
    "post-dispatch: pending_req={} pending_resp={}",
    pending_recv_req.is_some() as u32,
    pending_recv_resp.is_some() as u32,
);
```

---

## Expected Log After Applying Diagnostics

### Happy path (packet delivered correctly):

```
[DBG] transfer_and_clear: 1 packet(s)
[DBG] Recv intercept: ud=1 handle=0 has_pending_resp=0 has_pending_req=0
[DBG] try_service_pending: try_recv(handle=0) hit: type=5 size=10
```

### Failure path — empty transfer:

```
[DBG] transfer_and_clear: 0 packet(s)
→ packets_req was empty; Send dispatch did not produce a packet
→ check: is server_req.send() returning an error? (add error log to dispatch.rs MctpOp::Send)
```

### Failure path — inbound drops packet:

```
[DBG] transfer_and_clear: 1 packet(s)
[ERR] transfer_and_clear: inbound failed: X
→ cross-reference X with mctp::Error enum
→ likely: no listener registered (msg_type mismatch) or EID mismatch
```

### Failure path — try_recv miss despite successful inbound:

```
[DBG] transfer_and_clear: 1 packet(s)    (no inbound error)
[ERR] try_service_pending: try_recv(handle=0) miss on chan=1
→ packet was retained but with a different AppCookie (cookie encoding mismatch)
→ or: try_recv was called during an earlier Recv intercept that cleared the inbox
```

---

## Files to Change

| File | Change |
|---|---|
| `target/ast1060-evb/spdm-req-resp-test/mctp_loopback_server.rs` | Instrument `transfer_and_clear`, Recv intercept, `try_service_pending` |

No BUILD changes needed — `pw_log` is already a dep.

---

## After Diagnostics

Once the root cause is identified from the logs, the fix will fall into one of:

| Finding | Fix |
|---|---|
| `transfer_and_clear: 0 packet(s)` on req→resp path | Send dispatch path: `server_req.send()` not producing packets; check tag/EID resolution |
| `inbound failed` | EID or msg_type mismatch; check `server_resp` own_eid and listener registration |
| `try_recv miss` after successful inbound | Cookie mismatch; verify `Handle(0)` == `AppCookie(0)` in `try_recv` |
| Timing: correct path never reached | `try_service_pending` firing too early (before Send runs); may need to defer cross-side check |
