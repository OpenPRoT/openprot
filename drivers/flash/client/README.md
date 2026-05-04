# flash_client

Userspace IPC client facade for the flash driver.

Bazel target: `//drivers/flash/client:flash_client`

## Purpose

`flash_client` gives any userspace task a simple, blocking API to read,
write, erase, and discover flash over a Pigweed IPC channel. It handles
all request serialization, response parsing, and error mapping. It has
no knowledge of the underlying hardware — that lives in the platform
backend on the server side.

## Layer position

```
Application task
      │
      ▼
FlashClient  ←  this crate
      │  channel_transact (Pigweed IPC)
      ▼
flash_server  →  PlatformFlashBackend  →  SMC / FMC controller
```

See [`drivers/flash/api`](../api/) for the wire types and backend trait
shared with the server. The api README also defines the domain
vocabulary used here (*backend*, *geometry*, *region*, *route key*,
*capability flag*).

## API

### Construction

```rust
let mut flash = FlashClient::new(handle);                    // no default timeout
// or with a per-instance default:
let mut flash = FlashClient::with_default_timeout(
    handle,
    Some(Duration::from_secs(2)),
);
flash.set_default_timeout(Some(Duration::from_millis(500)));
```

`handle: u32` is a Pigweed IPC channel handle the platform binding
hands the task. The client takes `&mut self` on every method that
touches the wire — the synchronous one-call-in-flight invariant is
enforced at the type level. Each instance owns 1 KiB of scratch
(req + resp); per-call stack overhead is ~0.

The default-timeout knob is the policy applied by every method that
doesn't accept an explicit timeout. `None` means block until the server
responds; any concrete `Duration` bounds the call. The OS clock type
does not appear on the public API surface — only `core::time::Duration`.

### Probe

```rust
if flash.exists()? {
    // backend reports a responsive device on this handle
}
```

Returns `Ok(true)` when the backend reports a present device,
`Ok(false)` when it reports absence.

### Geometry

```rust
let bytes = flash.capacity()?;            // total flash size in bytes
let chunk = FlashClient::chunk_size();    // protocol const, no IPC

let geom = flash.geometry()?;
let granules = geom.erase_sizes_bitmap(); // bit n set => 1 << n is supported
let smallest = geom.min_erase_align_value();
let width    = geom.address_width;        // 3 or 4
let caps     = geom.capabilities();       // GeometryFlags
```

`chunk_size()` is a `const fn` returning `MAX_PAYLOAD_SIZE` — the
per-call payload cap is a protocol constant, identical for every
backend, so no IPC is issued.

`geometry()` issues one IPC and returns the full `FlashGeometry`
record (capacity, page size, supported-erase-size bitmap, address
width, capability flags). A client that needs to support multiple
flash chip vendors (Macronix, Winbond, Micron, ISSI, …) consumes
`erase_sizes_bitmap()` to pick the largest aligned erase opcode per
stride, instead of hard-coding the granule per board.

### Regions

```rust
let mut regions = [FlashRegion::new(0, 0, 0, RegionAttrs::empty()); 8];
let n = flash.regions(&mut regions)?;
for r in &regions[..n] {
    // r.route_key_value(), r.base_value(), r.length_value(), r.attributes()
}
```

The backend writes up to `out.len()` records into the buffer and
returns the count. A flat (single-region) backend returns one entry
with `RegionAttrs::WHOLE_CHIP` set spanning the device. Backends with
carved sub-regions return one entry per region. The client caps the
request at `MAX_PAYLOAD_SIZE / 16 = 16` records per call.

### Read

```rust
let mut buf = [0u8; 256];
let n = flash.read(address, &mut buf)?;
// or with an explicit timeout for this one call:
let n = flash.read_with_timeout(address, &mut buf, Some(Duration::from_millis(50)))?;
```

`buf.len()` must be ≤ `FlashClient::chunk_size()`. For reads larger
than one chunk, the caller is responsible for issuing multiple calls
and advancing the address.

### Write

```rust
let written = flash.write(address, &data[..])?;
// or with an explicit timeout:
let written = flash.write_with_timeout(address, &data[..], Some(Duration::from_millis(50)))?;
```

`data.len()` must be ≤ `FlashClient::chunk_size()`.

### Erase

```rust
flash.erase(address, length)?;
// or with an explicit timeout:
flash.erase_with_timeout(address, length, Some(Duration::from_secs(1)))?;
```

Both `address` and `length` must be aligned to and a multiple of one of
the granules advertised by `flash.geometry()?.erase_sizes_bitmap()`.

## Error handling

```rust
pub enum ClientError {
    IpcError(pw_status::Error),   // transport-level failure
    ServerError(FlashError),      // server returned a flash error code
    InvalidResponse,              // response frame is malformed
    BufferTooSmall,               // caller buffer exceeds MAX_PAYLOAD_SIZE
}
```

`FlashError` variants (defined in `flash_api`):

| Variant | Meaning |
|---|---|
| `InvalidOperation` | Unrecognised opcode |
| `InvalidAddress` | Address out of range |
| `InvalidLength` | Length zero, overflow, or misaligned |
| `BufferTooSmall` | Server-side buffer constraint |
| `Busy` | Backend busy |
| `Timeout` | Operation timed out |
| `WouldBlock` | Operation could not be completed immediately |
| `IoError` | Media-level failure |
| `NotPermitted` | Write-protected or restricted region |
| `InternalError` | Unclassified server fault |

### `Busy` vs `WouldBlock`

- **`Busy`**: The backend is actively performing another operation and cannot accept new requests. Retry the entire call after waiting.
- **`WouldBlock`**: This specific operation could not be completed immediately due to resource constraints or contention, but is not blocked by global device activity. Callers can implement smarter retry logic or adjust request parameters.

## Constraints

- `no_std` — targets Pigweed kernel userspace tasks only.
- Bazel `target_compatible_with` is scoped to AST10x0 targets (`TARGET_COMPATIBLE_WITH`).
- Single call is limited to `MAX_PAYLOAD_SIZE` (256 bytes) per transaction.
- All calls are synchronous / blocking on `channel_transact`.
- One in-flight call per `FlashClient` instance (enforced by `&mut self`).

## Dependencies

| Crate | Role |
|---|---|
| `flash_api` | Wire types (`FlashOp`, headers, `FlashGeometry`, `FlashRegion`, `FlashError`) |
| `userspace` (Pigweed) | `syscall::channel_transact`, internal kernel-deadline conversion |
| `pw_status` | IPC transport error type |
| `zerocopy` | Zero-copy header / discovery-record deserialization |
| `core::time::Duration` | Public-surface timeout type (no OS clock leaks into the API) |
