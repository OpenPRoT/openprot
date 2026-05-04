# flash_api

Shared wire protocol and backend trait for the OpenPRoT flash driver.

Bazel target: `//drivers/flash/api:flash_api`

## Purpose

`flash_api` is the contract crate consumed by both sides of the flash
IPC boundary:

- the userspace IPC facade ([`drivers/flash/client`](../client/)), which
  serializes requests and parses responses,
- the platform server (out of tree in this review repo), which dispatches
  opcodes onto a `FlashBackend` impl.

It owns the on-wire byte layout, the opcode set, the error code map,
the discovery value types, and the backend trait surface. No transport,
no syscalls, no platform code — pure data definitions plus one trait.

## Layer position

```
Application task
      │
      ▼
FlashClient  ─────────►  flash_api  ◄───────── FlashServer
                       (wire types,
                        backend trait)
                              │
                              ▼
                       PlatformFlashBackend
                              │
                              ▼
                          SMC / FMC
```

## Glossary

A few domain terms are used throughout this crate, the client, and the
server:

**Backend** — the platform-side code that actually talks to flash
silicon. Implements the `FlashBackend` trait. There is exactly one
backend per physical controller (e.g. an `Ast10x0FlashBackend` for the
AST10x0 SMC/FMC). The backend is what gives meaning to a `Read` or an
`Erase`; the wire protocol just shuttles the request to it.

**Geometry** — the *static shape* of a flash device, described by the
`FlashGeometry` value type: total capacity, write-page granularity,
which erase opcodes the part supports (4 KiB sector, 32 KiB block,
64 KiB block, …), the smallest required alignment, the addressing mode
(3-byte vs 4-byte addressing), and capability bits (e.g. whether the
backend can satisfy a server-side cross-flash `Copy`). One geometry
record per device. Authored statically by the backend; surfaced to
clients via the `GetGeometry` opcode so a portable BMC tool doesn't
need to hard-code the chip type per board.

**Region** — a *logical sub-range* of a flash device, described by the
`FlashRegion` value type: a base offset, a length, a logical handle
(`route_key`), and attribute bits (read-only? filter-protected?
hash-eligible?). A 64 MiB BMC flash might expose itself as one
whole-chip region; an OpenPRoT-internal flash typically carves itself
into four (active firmware / recovery / runtime state / AFM). One
device, many regions; surfaced to clients via the `GetRegions` opcode.

**Route key** — a `u32` naming a logical flash target — either the
whole device this channel is bound to, or one of the regions within
it. Carried inside `FlashRegion` so a region can be addressed
independently when handed to another service.

**Capability flag** — a bit in `GeometryFlags` (per device) or
`RegionAttrs` (per region) that says "this thing can do X" — e.g.
`HASH_ELIGIBLE` advertises that a server-side hash consumer can ingest
from this device or region without streaming bytes through the client
channel.

## Wire protocol

### Frame layout

Every request frame is a `FlashRequestHeader` (16 bytes, little-endian,
packed) followed by an opcode-specific payload of up to
`MAX_PAYLOAD_SIZE` (256) bytes. Every response frame is a
`FlashResponseHeader` (8 bytes, little-endian, packed) followed by an
opcode-specific payload of up to `MAX_PAYLOAD_SIZE` bytes.

```rust
#[repr(C, packed)]
pub struct FlashRequestHeader {
    pub op_code: u8,
    pub flags: u8,
    pub payload_len: u16,
    pub address: u32,
    pub length: u32,
    pub reserved: u32,
}                                  // = 16 bytes

#[repr(C, packed)]
pub struct FlashResponseHeader {
    pub status: u8,                // 0 = Success; otherwise FlashError
    pub reserved: u8,
    pub payload_len: u16,
    pub value: u32,                // op-specific (capacity, byte count, ...)
}                                  // = 8 bytes
```

Both headers derive `zerocopy::{FromBytes, IntoBytes, Immutable,
KnownLayout}` and ship `new`/`success`/`error` builders plus
little-endian-aware accessors (`address_value()`, `length_value()`,
`value_word()`, `payload_length()`, …) so neither side needs to
hand-roll byte twiddling.

### Opcodes

| Op | Value | Request shape | Response shape |
|---|---|---|---|
| `Exists` | 0x01 | header only | `value` = 0/1 |
| `GetCapacity` | 0x02 | header only | `value` = bytes |
| `Read` | 0x03 | header (`address`, `length`) | `value` = byte count, payload = bytes read |
| `Write` | 0x04 | header (`address`, `length`, `payload_len`) + payload | `value` = byte count |
| `Erase` | 0x05 | header (`address`, `length`) | empty |
| `GetGeometry` | 0x06 | header only | payload = `FlashGeometry` (24 B) |
| `GetRegions` | 0x07 | header (`length` = max records) | `value` = count, payload = N × `FlashRegion` (16 B) |

`MAX_PAYLOAD_SIZE` is a protocol constant: every backend honours the
same value, so clients reference it directly rather than querying for
it.

## Discovery value types

### `FlashGeometry` (24 B)

Returned in the `GetGeometry` response payload.

```rust
pub struct FlashGeometry {
    pub capacity: u32,
    pub page_size: u32,           // write granularity (typically 256)
    pub erase_sizes: u32,         // bitmap; bit n set => 1 << n bytes supported
    pub min_erase_align: u32,
    pub address_width: u8,        // 3 or 4
    pub flags: u8,                // GeometryFlags bits
    pub _rsv: [u8; 6],
}
```

`erase_sizes` as a bitmap lets the client pick the largest aligned
erase opcode per stride (e.g. 4 KiB | 32 KiB | 64 KiB =
`(1<<12) | (1<<15) | (1<<16)`).

`GeometryFlags` (`bitflags!`):

| Bit | Name | Meaning |
|---|---|---|
| 0 | `DMA_ELIGIBLE` | Backend can satisfy a server-side cross-flash byte copy without per-chunk client round-trips. |
| 1 | `HASH_ELIGIBLE` | A server-side hash consumer can ingest from this device without streaming bytes through the client channel. |

### `FlashRegion` (16 B)

Returned in the `GetRegions` response payload — one entry per logical
region exposed by the device.

```rust
pub struct FlashRegion {
    pub route_key: u32,           // logical handle naming this region
    pub base: u32,
    pub length: u32,
    pub attrs: u32,               // RegionAttrs bits
}
```

A backend with no carved sub-regions returns a single entry with
`RegionAttrs::WHOLE_CHIP` set spanning `[0, capacity)`.

`RegionAttrs` (`bitflags!`):

| Bit | Name | Meaning |
|---|---|---|
| 0 | `FILTER_PROTECTED` | Server enforces an access policy over this region (mechanism is platform-specific). |
| 1 | `HASH_ELIGIBLE`    | A server-side hash consumer can ingest this region without streaming bytes through the client. |
| 2 | `READ_ONLY`        | Server refuses `Write`/`Erase` against this region. |
| 3 | `WHOLE_CHIP`       | Region spans the whole physical chip. |

## Backend trait

```rust
pub trait FlashBackend {
    type RouteKey: Copy;

    fn info(&self, key: Self::RouteKey) -> FlashInfo;

    fn geometry(&self, key: Self::RouteKey)
        -> Result<FlashGeometry, BackendError>;     // default derives from info()

    fn regions(&self, key: Self::RouteKey, out: &mut [FlashRegion])
        -> Result<usize, BackendError>;             // default = 1 whole-chip entry

    fn exists(&mut self, key: Self::RouteKey)
        -> Result<bool, BackendError>;              // default Ok(true)

    fn read (&mut self, key: Self::RouteKey, address: u32, out:  &mut [u8])
        -> Result<usize, BackendError>;
    fn write(&mut self, key: Self::RouteKey, address: u32, data: &[u8])
        -> Result<usize, BackendError>;
    fn erase(&mut self, key: Self::RouteKey, address: u32, length: u32)
        -> Result<(),    BackendError>;

    fn enable_interrupts (&mut self) -> Result<(), BackendError>;
    fn disable_interrupts(&mut self) -> Result<(), BackendError>;
}
```

Discovery methods (`info`, `geometry`, `regions`) take `&self` — they
report static authoring on the server side and don't need exclusive
access. `geometry` and `regions` ship default impls so existing
single-region single-erase-granule backends stay source-compatible
without writing boilerplate.

`RouteKey` is an associated type. Single-CS backends set it to `()`;
multi-CS controllers set it to a chip-select index. Channel-implicit
routing keeps the wire header free of routing fields — each
`FlashClient` is bound to one CS via its IPC handle, and the server
maps channel → backend → `RouteKey`. Where routing genuinely crosses a
device boundary, the routing data lives inside the relevant struct
(e.g. the `route_key` field on `FlashRegion`).

## Errors

`FlashError` is the wire status code carried in
`FlashResponseHeader::status`:

| Variant | Code | Meaning |
|---|---|---|
| `Success` | 0x00 | OK |
| `InvalidOperation` | 0x01 | Unknown opcode |
| `InvalidAddress` | 0x02 | Address out of range |
| `InvalidLength` | 0x03 | Length zero, overflow, or misaligned |
| `BufferTooSmall` | 0x04 | Server-side buffer constraint |
| `Busy` | 0x05 | Backend busy |
| `Timeout` | 0x06 | Operation timed out |
| `WouldBlock` | 0x07 | Could not complete synchronously; retry after IRQ |
| `IoError` | 0x08 | Media-level failure |
| `NotPermitted` | 0x09 | Write-protected or restricted region |
| `InternalError` | 0xFF | Unclassified server fault |

`BackendError` is the trait-level error backends produce; an `impl
From<BackendError> for FlashError` provides the canonical mapping for
the server's response-encoding path.

## Tests

Host-side unit tests cover each wire type at the encoder/decoder
level: opcode and error-code round-trips (known values + unknown-byte
fallthrough), `new`-and-accessor round-trips for the request and
response headers as well as `FlashGeometry` and `FlashRegion`,
explicit little-endian byte-position asserts, and short-buffer
rejection on header decode.

```
bazel test //drivers/flash/api:flash_api_test
```

## Constraints

- `no_std` — no allocator, no I/O.
- Pure data + one trait. No syscalls, no clocks, no platform deps.
- Host-buildable — picked up by the CI `//...` wildcard.

## Dependencies

| Crate | Role |
|---|---|
| `bitflags` | `GeometryFlags`, `RegionAttrs` |
| `zerocopy` | `FromBytes` / `IntoBytes` derives on wire structs |
