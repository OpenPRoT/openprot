<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# AST10x0 PFR Crate (`ast10x0_pfr`)

This crate provides the AST10x0 PFR (Platform Firmware Resilience) building
blocks. Its current public surface is [`SwmbxCtrl`](./swmbx_ctrl.rs), the
in-memory controller for the **software mailbox** (swmbx).

`SwmbxCtrl` models a small block of shared memory that two host agents poke at
over SMBus/I²C, and layers three behaviors on top of that memory: **write
protection**, **change notification**, and **FIFO-backed registers**.

The crate is `no_std` and allocation-free: all state lives in fixed-size arrays
sized by compile-time constants.

---

## 1. The mental model

Picture a 256-byte register file that is visible to more than one master:

```
            address 0x00 ........................... 0xFF
           +----+----+----+----+--   --+----+----+----+
  buffer:  | b0 | b1 | b2 | b3 |  ...  |    |    |bFF |   <- flat shared memory
           +----+----+----+----+--   --+----+----+----+
              ^                                   (lives at `buffer_base`)
              |
   port 0 (e.g. BMC) ---+
                        +---> both ports see the SAME 256 bytes
   port 1 (e.g. PCH) ---+
```

* A **port** is one host-side master. There are `SWMBX_DEV_COUNT = 2` of them
  (think BMC and PCH/CPU). Both ports share the one underlying buffer, but each
  port has its *own* per-address policy and its own transaction state.
* An **address** (`u8`, `0x00..=0xFF`) selects one byte/register. Each address is
  called a **node**; there are `SWMBX_NODE_COUNT = 256` nodes per port.
* Some addresses can be **remapped** away from the flat buffer to a **FIFO** so
  that repeated writes to the same address stream into a queue instead of
  overwriting one byte. There are `SWMBX_FIFO_COUNT = 4` FIFOs, each up to
  `SWMBX_FIFO_DEPTH = 256` entries deep.

`SwmbxCtrl` is the *policy/state* layer only. It does not own a real bus; the
shared memory is reached through a caller-supplied buffer address (see §7).

---

## 2. State at a glance

```
SwmbxCtrl
├── mbx_en:           u8                        // GLOBAL feature switches
├── node:             [[SwmbxNode; 256]; 2]     // per-port, per-address policy
├── fifo:             [SwmbxFifo<256>; 4]       // the four FIFO endpoints
├── mbx_fifo_execute: [bool; 2]                 // per-port: in a FIFO transaction?
├── mbx_fifo_addr:    [u8;   2]                 // per-port: address that opened it
├── mbx_fifo_idx:     [u8;   2]                 // per-port: which FIFO is active
├── buffer_base:      usize                     // address of the 256-byte buffer
└── buffer_size:      usize                     // length of the buffer region
```

### Global switches — `mbx_en`

A bitmask of three independent features, toggled with `enable_behavior`:

| Flag            | Bit      | Meaning when set globally                      |
|-----------------|----------|------------------------------------------------|
| `SWMBX_PROTECT` | `1 << 0` | Per-node write protection is *armed*           |
| `SWMBX_NOTIFY`  | `1 << 1` | Per-node / FIFO notifications are *armed*       |
| `SWMBX_FIFO`    | `1 << 2` | FIFO remapping is *armed*                       |

A feature only takes effect when **both** the global switch *and* the relevant
per-node bit are set. This two-level gating lets firmware arm/disarm a whole
class of behavior with one write without losing the per-address configuration.

### Per-node policy — `SwmbxNode`

```rust
struct SwmbxNode {
    notify_flag:   bool, // a pending "this node changed" event
    enabled_flags: u8,   // per-node PROTECT / NOTIFY bits
}
```

`enabled_flags` reuses the `SWMBX_PROTECT` / `SWMBX_NOTIFY` bit positions, but at
*per-address* granularity. `notify_flag` is the latched event for that node.

### A FIFO endpoint — `SwmbxFifo<N>`

```rust
struct SwmbxFifo<N> {
    queue:         Queue<SwmbxFifoEntry, N>, // heapless SPSC ring of bytes
    notify_flag:   u8,    // CONFIG: which of START/STOP notifies are enabled
    notify_start:  bool,  // runtime: has START notify already fired this txn?
    fifo_write:    bool,  // runtime: was anything written this txn?
    fifo_offset:   u8,    // the address this FIFO is bound to
    enabled:       bool,  // is this FIFO active?
    msg_index:     usize, // write cursor
    max_msg_count: usize, // effective depth (<= N)
}
```

> Note the name collision: `SwmbxNode::notify_flag` is a **bool event latch**,
> while `SwmbxFifo::notify_flag` is a **u8 configuration mask** of
> `SWMBX_FIFO_NOTIFY_START (1<<0)` / `SWMBX_FIFO_NOTIFY_STOP (1<<1)`. They are
> unrelated despite sharing a field name.

---

## 3. Configuration API

These set up policy before traffic flows:

| Method                                   | Effect                                                            |
|------------------------------------------|------------------------------------------------------------------|
| `new_with_regions(size, buf)`            | Construct, recording the buffer region address. **`unsafe`** (§7). |
| `enable_behavior(flag, enable)`          | Flip one or more global switches in `mbx_en`.                     |
| `update_protect(port, addr, enable)`     | Set/clear the per-node `PROTECT` bit for one address.            |
| `apply_protect(port, bitmap, start_idx)` | Bulk-set protection from packed 32-bit words (see below).        |
| `update_notify(port, addr, enable)`      | Set/clear the per-node `NOTIFY` bit for one address.            |
| `update_fifo(idx, addr, depth, notify, enable)` | Bind FIFO `idx` to `addr`, set depth + notify config, (re)enable. |
| `flush_fifo(idx)`                        | Drain a FIFO and reset its transient flags.                     |

### `apply_protect` and the bitmap geometry

Protection for all 256 nodes fits in `256 / 32 = 8` words of 32 bits each
(`PROTECT_WORD_COUNT = 8`, `PROTECT_BITS_PER_WORD = 32`). Each word covers a
contiguous block of 32 addresses; `start_idx` says which word the slice begins at:

```
node address = (start_idx + word_index) * 32 + bit_index
                                         └─ PROTECT_WORD_SHIFT = log2(32) = 5
```

Bit set → node protected, bit clear → node unprotected. The range is validated
with checked arithmetic, so an out-of-range or overflowing `start_idx` returns
`InvalidProtectRange` instead of panicking or wrapping.

---

## 4. The two data paths

Every read/write resolves to one of two backends:

```
                     ┌─────────────────────────── send_msg / get_msg ───────────────────────────┐
                     │                                                                            │
   in a FIFO txn AND │                              else                                          │
   SWMBX_FIFO armed? │                                                                            │
            yes ─────┤                                                                       no ──┤
                     ▼                                                                            ▼
              ┌─────────────┐                                                          ┌──────────────────┐
              │ FIFO path   │                                                          │ flat-buffer path │
              │ append/pop  │                                                          │ write/read byte  │
              │ on fifo[i]  │                                                          │ at buffer_base   │
              └─────────────┘                                                          └──────────────────┘
```

The selector is: `mbx_fifo_execute[port] && (mbx_en & SWMBX_FIFO) != 0`.

---

## 5. Transaction lifecycle (FIFO path)

FIFO routing is *per transaction*, bracketed by `send_start` / `send_stop`:

```
 send_start(port, addr)
   │  check_fifo(addr): is there an ENABLED fifo whose fifo_offset == addr?
   ├─ yes → mbx_fifo_execute[port] = true
   │        mbx_fifo_addr[port]    = addr      // remember which node opened it
   │        mbx_fifo_idx[port]     = i         // remember which FIFO
   └─ no  → mbx_fifo_execute[port] = false     // this txn uses the flat buffer
   │
   ▼
 send_msg(port, addr, val)   (repeatable)
   │  FIFO path → fifo[i].append_write(val)
   │             • on first write, if NOTIFY armed + node NOTIFY set +
   │               fifo START-notify configured  → latch node notify_flag,
   │               mark notify_start so it fires only once
   │             • on FIFO full → latch node notify_flag, return FifoFull
   │
 get_msg(port, addr)         (repeatable)
   │  FIFO path → fifo[i].dequeue()  // removes (pops) the oldest byte
   │
   ▼
 send_stop(port)
   │  if STOP-notify configured + something was written this txn +
   │     node NOTIFY set  → latch node notify_flag
   │  reset notify_start / fifo_write, clear mbx_fifo_* for this port
```

So a typical streamed-register exchange is:
`send_start → send_msg × N → (peer) get_msg × N → send_stop`.

---

## 6. Write protection & notification (flat-buffer path)

When not in a FIFO transaction, `send_msg` runs the node policy:

* **Protection.** The byte is written to the buffer **unless** it is protected —
  i.e. it is dropped only when *both* the node's `PROTECT` bit *and* the global
  `SWMBX_PROTECT` switch are set. A protected write is silently discarded (the
  call still returns `Ok`); the buffer keeps its old value.
* **Notification.** If the global `SWMBX_NOTIFY` switch *and* the node's `NOTIFY`
  bit are set, the node's `notify_flag` is latched to `true`.

`get_msg` on this path simply reads the byte from the buffer.

> **Current limitation:** `notify_flag` on a node is *written* but the crate
> exposes no API to *read or consume* it, so notifications are presently
> unobservable from outside. Wiring up a consumer (e.g. `take_notify(port, addr)`)
> is a known follow-up.

### Direct helpers

`swmbx_write(fifo, addr, val)` / `swmbx_read(fifo, addr)` bypass transaction
state entirely: with `fifo = true` they act on the FIFO mapped to `addr` (error
`FifoNotMapped` if none), otherwise straight on the flat buffer. These are the
"the firmware itself wants to poke the mailbox" path, as opposed to modeling a
host master.

---

## 7. Shared buffer region and safety

`SwmbxCtrl` never embeds the mailbox memory; it stores the buffer **address**
(`buffer_base`) and reaches it through `SharedRegion<T>`, a thin wrapper that does
**volatile** reads/writes through a `NonNull<T>`.

Why this exists: on the target, the buffer is a fixed physical/SRAM region shared
with other masters, so the controller must reference it by address rather than
own it.

Because raw addresses are involved, the **soundness boundary is explicit**:

* `SharedRegion::from_addr` and `SwmbxCtrl::new_with_regions` are **`unsafe`** —
  the caller promises the address is valid, aligned, mapped, and uniquely owned.
  Each has a `# Safety` section spelling out the contract.
* Once a controller has been constructed under that promise, the *accessor*
  methods (`send_msg`, `get_msg`, `swmbx_read/write`, the `update_*` family) are
  **safe** — using the mailbox requires no further `unsafe`.
* `from_addr` additionally asserts non-null and correct alignment. For a
  *compile-time-constant* address these asserts become **compile errors**; for a
  runtime address they are runtime panics (a null/misaligned base is treated as
  an unrecoverable configuration bug, not a returnable error).

### Concurrency model: single-context, fed serially over IPC

`SwmbxCtrl` is **not** internally synchronized, and deliberately so. It is driven
from a **single execution context**: the i2c service runs a cooperative event
loop (`object_wait` over a `WaitGroup`) in which the hardware interrupt is just
another wake-up — the IRQ only drains the slave-RX latch, `interrupt_ack`s, and
signals the client. The mailbox itself is then driven **serially** by one
consumer (one IPC channel per bus); the IRQ never reentrantly touches controller
state. Because of this:

* All mutators take `&mut self`; `mbx_en` is a plain `u8` (no `Cell`, no atomics,
  no lock). The type is intentionally `!Sync`, so any accidental attempt to share
  one controller across contexts is a **compile error** rather than a silent data
  race.
* The two **ports** are *external* bus masters (e.g. BMC and PCH), not internal
  threads. They race on the shared **buffer memory**, which is why `SharedRegion`
  uses **volatile** accesses; the controller's own fields (`mbx_en`, `node`,
  `fifo`) live in firmware RAM and are touched only by the single consumer.

---

## 8. Error model

`SwmbxError` is a `#[repr(u16)]` enum with stable diagnostic codes (`code()` →
`u16`) suitable for firmware telemetry:

| Variant                | Code     | Raised when …                                    |
|------------------------|----------|--------------------------------------------------|
| `InvalidPort`          | `0x1001` | `port >= SWMBX_DEV_COUNT`                         |
| `InvalidAddress`       | `0x1002` | address outside the buffer / node range          |
| `InvalidFifoIndex`     | `0x1003` | `index >= SWMBX_FIFO_COUNT`                       |
| `InvalidFifoDepth`     | `0x1004` | configured depth is 0 or `> SWMBX_FIFO_DEPTH`     |
| `InvalidFlagMask`      | `0x1005` | `enable_behavior` flag has no known bits          |
| `InvalidProtectRange`  | `0x1006` | `apply_protect` slice out of range / overflows    |
| `FifoFull`             | `0x1007` | append to a full FIFO                              |
| `FifoEmpty`            | `0x1008` | read from an empty FIFO                            |
| *(0x1009 retired)*     | —        | was `NullRegionBase`; null base is now a panic     |
| `FifoNotMapped`        | `0x100A` | direct FIFO access to an unmapped address          |

Codes are an external ABI: `0x1009` is intentionally left reserved so existing
code values don't shift.

---

## 9. Worked example

```rust
const BUF_LEN: usize = 256;
let mut backing = [0u8; BUF_LEN];

// SAFETY: `backing` is live, uniquely-owned memory for the controller's life.
let mut ctrl = unsafe {
    SwmbxCtrl::new_with_regions(BUF_LEN, backing.as_mut_ptr() as usize, 0)
};

// --- Flat register with write-protect + notify -------------------------------
ctrl.enable_behavior(SWMBX_PROTECT | SWMBX_NOTIFY, true)?;
ctrl.update_protect(0, 0x10, true)?;            // node 0x10 is read-only to port 0
ctrl.send_msg(0, 0x10, 0xAA)?;                  // dropped: stays 0x00
ctrl.update_protect(0, 0x10, false)?;
ctrl.send_msg(0, 0x10, 0xAA)?;                  // now written
assert_eq!(ctrl.get_msg(0, 0x10)?, 0xAA);

// --- Streaming register backed by a FIFO -------------------------------------
ctrl.enable_behavior(SWMBX_FIFO, true)?;
ctrl.update_fifo(0, 0x0D, 4, SWMBX_FIFO_NOTIFY_START | SWMBX_FIFO_NOTIFY_STOP, true)?;

ctrl.send_start(0, 0x0D)?;                       // 0x0D resolves to FIFO 0
ctrl.send_msg(0, 0x0D, 0x11)?;
ctrl.send_msg(0, 0x0D, 0x22)?;
assert_eq!(ctrl.get_msg(0, 0x0D)?, 0x11);       // pops in order
assert_eq!(ctrl.get_msg(0, 0x0D)?, 0x22);
assert_eq!(ctrl.get_msg(0, 0x0D), Err(SwmbxError::FifoEmpty));
ctrl.send_stop(0)?;
```

---

## 10. Build & test

```sh
# Build the (target-only) library
bazel build //target/ast10x0/pfr:pfr

# Run the host unit tests
bazel test //target/ast10x0/pfr:swmbx_ctrl_host_test
```

---

## 11. Quick reference: glossary

| Term            | In code                              | Meaning                                   |
|-----------------|--------------------------------------|-------------------------------------------|
| Port / device   | `port: usize`, `SWMBX_DEV_COUNT`     | One host master (e.g. BMC, PCH).          |
| Node            | `SwmbxNode`, indexed by `addr: u8`   | Per-address policy + event latch.         |
| Buffer          | `buffer_base` + `SharedRegion`       | The flat 256-byte shared register file.   |
| FIFO endpoint   | `SwmbxFifo`, `fifo[idx]`             | A queue bound to one address.             |
| Transaction     | `send_start` … `send_stop`           | The window during which FIFO routing applies. |
| Global switch   | `mbx_en` bits                        | Arms a whole feature class.               |
| Per-node bit    | `SwmbxNode::enabled_flags`           | Arms a feature for one address.           |

---

## 12. How SWMBX sits in Zephyr PFR

```
                 ASPEED Zephyr PFR (apps/aspeed-pfr)
┌──────────────────────────────────────────────────────────────────────────────┐
│ Application logic                                                           │
│  - AspeedStateMachine.c                                                     │
│  - mctp/plat_mctp.c, mctp/mctp_smbus.c                                     │
│        |                                                                    │
│        | calls swmbx_* API (enable/update/read/write/send_start/stop)      │
│        v                                                                    │
│  include/zephyr/drivers/i2c/pfr/swmbx.h                                    │
└────────┬─────────────────────────────────────────────────────────────────────┘
      │ API contract
      v
┌──────────────────────────────────────────────────────────────────────────────┐
│ SW mailbox controller driver                                                │
│  drivers/i2c/pfr/swmbx.c                                                    │
│                                                                              │
│  swmbx_ctrl_data:                                                            │
│   - mbx_en                                                                    │
│   - node[2][256]  (protect/notify bits + sem ptr)                           │
│   - fifo[4]      (addr mapping, queue state, sem ptr)                       │
│   - buffer ptr -> SWMBX_BUF_BASE                                             │
│                                                                              │
│  global indirection:                                                         │
│   swmbx_info @ SWMBX_INFO_BASE --> pointer to swmbx_ctrl_data               │
└────────┬─────────────────────────────────────────────────────────────────────┘
      │
      │ data path (flat mailbox bytes)
      v
┌──────────────────────────────────────────────────────────────────────────────┐
│ Shared mailbox memory                                                        │
│  SWMBX_BUF_BASE = 0x7e7b0e00                                                 │
│  256-byte register window (node 0x00..0xFF)                                 │
└────────┬─────────────────────────────────────────────────────────────────────┘
      │
      │ transport path (I2C/SMBus target endpoints)
      v
┌──────────────────────────────────────────────────────────────────────────────┐
│ I2C target side                                                              │
│  drivers/i2c/target/swmbx_target.c                                          │
│  dts bindings: aspeed,swmbx-ctrl + aspeed,swmbx-dev                         │
│  board overlays instantiate swmbx0/swmbx1 endpoints                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Layering explained

1. **Application layer**
  PFR state machine and MCTP code call `swmbx_*` APIs to configure behavior
  (protect/notify/fifo) and move mailbox bytes.

2. **Driver policy/state layer (`swmbx.c`)**
  Holds all policy and runtime state (`mbx_en`, per-node flags, FIFO mapping,
  transaction state). This is where address-level protection, notify latching,
  and FIFO transaction semantics are implemented.

3. **Shared memory layer (`SWMBX_BUF_BASE`)**
  Backing storage for the flat mailbox register file (`0x00..0xFF`). Non-FIFO
  accesses read/write this byte array directly.

4. **Transport layer (I2C target endpoints)**
  `swmbx_target.c` and DT overlays expose SWMBX endpoints on SMBus/I2C so
  external masters (for example BMC and CPU/PCH) can drive mailbox traffic.

5. **Cross-context handoff (`SWMBX_INFO_BASE`)**
  The C implementation stores a pointer to `swmbx_ctrl_data` in shared info
  memory so non-device-handle entry points (`swmbx_send_*`) can locate the
  active controller state.

In short: **application calls API -> driver enforces policy -> bytes land in
shared mailbox memory -> transport exposes that memory to external SMBus/I2C
masters**.
