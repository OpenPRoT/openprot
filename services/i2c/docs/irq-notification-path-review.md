# I2C Server IRQ Notification Path — Review & Codegen Audit

## Scope

Full review of the interrupt-driven slave-receive path from hardware fire to
server re-enable, covering:

- System configuration (`system.json5`)
- Code generation (`system_generator` + templates)
- Kernel object construction and NVIC enablement
- Runtime dispatch in the server loop
- Backend `drain_slave_rx` implementation

Targets reviewed: `target/ast1060-evb/i2c` and `target/ast1060-evb/i2c-slave`.
Generated codegen audited from:
`bazel-out/k8-fastbuild-ST-7702e646e92b/bin/target/ast1060-evb/i2c-slave/codegen_codegen.rs`

---

## Interrupt Flow (end-to-end)

```
1. IRQ 112 fires (AST1060 I2C2 combined master+slave)
      ↓
2. interrupt_handler_i2c_server_i2c2_irq_i2c2()  [generated]
      → userspace_interrupt_handler_enter(112)
          NVIC: disable_interrupt(112)           ← IRQ masked
          returns PreemptDisableGuard
      → object.interrupt(Arch, Signals::INTERRUPT_A)
          base.signal() sets bit 16, wakes server thread
      → userspace_interrupt_handler_exit(112, guard)
          handler_done() → triggers PendSV context switch
      ↓
3. Server thread wakes from object_wait(WG, READABLE)
      wait_return.user_data == WTOKEN_IRQ (1)
      ↓
4. handle_i2c_interrupt(&mut backend) → bool
      for bus in 0..14: backend.drain_slave_rx(bus)
        backend gates: returns Err(NotInitialized) for non-notification buses
        for notification-enabled bus: reads hardware slave status register once
          DataReceived → slave_read → latch into SlaveNotificationState.rx_buf
          Stop / WriteRequest / ReadRequest / DataSent / None → Ok(0)
      returns true if any bus latched data
      ↓
5. if any_data → syscall::raise_peer_user_signal(handle::I2C)
      client wakes, calls SlaveReceive IPC
      server calls get_buffered_slave_message() → copies rx_buf to response
      ↓
6. syscall::interrupt_ack(handle::I2C2_IRQ, signals::I2C2)
      InterruptObject::interrupt_ack:
        clears Signals::INTERRUPT_A from active_signals
        calls ack_irqs(INTERRUPT_A):
          signal_mask_table = [112]
          INTERRUPT_A set → userspace_interrupt_ack(112)
            NVIC: enable_interrupt(112)          ← IRQ re-enabled
```

NVIC mask window: from handler entry (step 2) until `interrupt_ack` returns
(step 6). Hardware AST1060 slave packet mode NACKs new master writes during
this window — no data loss.

---

## Codegen Audit (i2c-slave system image)

### Vector Table

```rust
static PW_KERNEL_INTERRUPT_TABLE_ARRAY: [InterruptTableEntry; 113] = { ... };
interrupt_table[112] = Some(interrupt_handler_i2c_server_i2c2_irq_i2c2);
```

- Table size: 113 (IRQ 112 + 1). ✅
- Only slot 112 populated — no spurious entries. ✅
- Placed in `.vector_table.interrupts` section. ✅

### IRQ Handler

```rust
extern "C" fn interrupt_handler_i2c_server_i2c2_irq_i2c2() {
    if let Some(object) = unsafe { INTERRUPT_OBJECT_I2C_SERVER_I2C2_IRQ.get() } {
        let preempt_guard =
            InterruptController::userspace_interrupt_handler_enter(arch::Arch, 112);
        object.interrupt(arch::Arch, Signals::INTERRUPT_A);
        InterruptController::userspace_interrupt_handler_exit(arch::Arch, 112, preempt_guard);
    }
}
```

- Guards with `if let Some` — safe if called before `codegen::start()`. ✅
- Uses `Signals::INTERRUPT_A` (bit 16) — matches `signals::I2C2` in app crate. ✅

### InterruptObject + `ack_irqs`

```rust
let signal_mask_table: [u32; 1] = [112];
for (index, irq) in signal_mask_table.iter().enumerate() {
    let interrupt_bit = 1 << (16 + index);
    if signal_mask.contains(Signals::from_bits_retain(interrupt_bit)) {
        InterruptController::userspace_interrupt_ack(*irq);  // → enable_interrupt(112)
    }
}
// Boot-time:
InterruptController::enable_interrupt(112);
```

- `ack_irqs` iterates exactly 1 entry. ✅
- Boot-time `enable_interrupt(112)` called after `StaticContext::set()`. ✅

### Object Table — i2c_server process

| Index | Object | Handle constant |
|---|---|---|
| 0 | `object_i2c_server_i2c` (ChannelHandler) | `handle::I2C = 0` |
| 1 | `object_i2c_server_i2c2_irq` (InterruptObject) | `handle::I2C2_IRQ = 1` |
| 2 | `object_i2c_server_wg` (WaitGroup) | `handle::WG = 2` |

Order matches server constants exactly. ✅

### MPU Regions — i2c_server process

| Name | Type | Range |
|---|---|---|
| kernel_code | ReadOnlyExecutable | `0x680–0x20000` |
| flash (server) | ReadOnlyExecutable | `0x20000–0x40000` |
| ram (server) | ReadWriteData | `0x80000–0x90000` |
| i2c_regs | Device | `0x7e7b0000–0x7e7b4000` |
| scu | Device | `0x7e6e2000–0x7e6e3000` |

`i2c_regs` ends at `0x7e7b4000` (0x4000 bytes) — covers all 14 controllers,
buffer regions, and filter registers. ✅

### i2c_slave_echo process

- Single `ChannelInitiatorObject` wired to `i2c_server`'s handler via
  `set_initiator`. ✅
- `raise_peer_user_signal` back-reference established at boot. ✅

---

## Issues Found and Fixed

| # | Severity | File | Issue | Fix |
|---|---|---|---|---|
| 1 | Minor | `server/main.rs` | `interrupt_ack` error silently dropped (`let _`) | Changed to `if let Err(e)` with `pw_log::warn!` |
| 2 | Minor | `server/main.rs` | `raise_peer_user_signal` called unconditionally — spurious client wakeups on non-data interrupts | `handle_i2c_interrupt` now returns `bool`; signal raised only if `true` |
| 3 | Design | `server/main.rs` | Redundant `notification_enabled: [bool; 14]` tracked in server *and* `SlaveNotificationState` | Removed server-side array; backend is single source of truth |
| 4 | Minor | `server/main.rs` | Magic numbers `0` / `1` in `wait_group_add` and `user_data` comparison | Added `WTOKEN_IPC = 0` / `WTOKEN_IRQ = 1` constants |
| 5 | Minor | `backend/lib.rs` | `drain_slave_rx` wildcard `_ => 0` masked all non-DataReceived events | Replaced with explicit arms for each `SlaveEvent` variant with comments |
| 6 | Design | `i2c/system.json5` | `i2c_controllers` mapping `0x2000` diverged from `i2c-slave/system.json5` (`0x4000`) | Updated to `0x4000` — confirmed in generated MPU region `0x7e7b4000` |

---

## Signal Bit Mapping

```
Signals::INTERRUPT_A = bit 16   →   IRQ 112 (I2C2)
```

One `InterruptObject` (`I2C2_IRQ`) owns one IRQ. Up to 15 additional IRQs
(`INTERRUPT_B`…`INTERRUPT_O`) could be added to the same object if more buses
require hardware interrupt notification.

---

## Key Architectural Properties

- **IRQ ownership**: static, build-time only. No runtime IRQ claiming.
- **NVIC protection**: MPU prevents userspace from accessing NVIC directly.
  Re-enable only via `interrupt_ack()` syscall.
- **Context switch**: deferred to PendSV via `handler_done()` — not inline in
  the IRQ handler.
- **Hardware back-pressure**: AST1060 slave packet mode NACKs new master
  writes while the buffer is unarmed — flat 255-byte `SlaveNotificationState`
  buffer is sufficient.
- **No tight loops in the IRQ path**: `drain_slave_rx` reads the slave status
  register exactly once per call. Polling loops (`slave_wait_event`,
  `slave_receive`) are only on the synchronous IPC path.

---

## Comparison with `spdm-standup` Baseline

`spdm-standup` is the older baseline; `openprot` is the branch with the full
interrupt-driven slave notification implementation.

### Server architecture divergence

| Aspect | openprot | spdm-standup |
|---|---|---|
| Event loop | `WaitGroup` multiplexing IPC + IRQ | Simple `object_wait` → `channel_read` → `channel_respond` |
| `SlaveReceive` dispatch | `get_buffered_slave_message()` (non-blocking, interrupt path) | `slave_receive()` (polling loop) |
| `EnableSlaveNotification` | Fully implemented dispatch arm | Not present |
| `DisableSlaveNotification` | Fully implemented dispatch arm | Not present |

### Backend divergence

| Aspect | openprot | spdm-standup |
|---|---|---|
| `SlaveNotificationState` struct | Present (per-bus enabled flag + 255-byte rx_buf) | Not present |
| `enable_slave_notification()` | Implemented | Not present |
| `disable_slave_notification()` | Implemented | Not present |
| `drain_slave_rx()` | Implemented (IRQ path) | Not present |
| `get_buffered_slave_message()` | Implemented | Not present |
| `I2cConfig` construction | `I2cConfig::default()` | `I2cConfig::with_static_clocks()` |

### API divergence

| Aspect | openprot | spdm-standup |
|---|---|---|
| `I2cOp::EnableSlaveNotification = 13` | Present | Not present |
| `I2cOp::DisableSlaveNotification = 14` | Present | Not present |
| `I2cClient::enable_slave_notification()` | Full implementation | Stubbed — returns `Err(ServerError)` |

### System configuration divergence

| Target | File | Aspect | openprot | spdm-standup |
|---|---|---|---|---|
| `i2c-slave` | `system.json5` | `vector_table_size_bytes` | `1664` (0x680) | `1280` (0x500) |
| `i2c-slave` | `system.json5` | `flash_start_address` | `0x00000680` | `0x00000500` |
| `i2c-slave` | `system.json5` | i2c_server objects | `I2C`, `I2C2_IRQ`, `WG` | `I2C` only |
| `i2c` | `system.json5` | `vector_table_size_bytes` | `1664` (0x680) | `1280` (0x500) |
| `i2c` | `system.json5` | i2c_server objects | `I2C`, `I2C2_IRQ`, `WG` | `I2C` only |
| `i2c` | `system.json5` | `i2c_controllers` size | `0x4000` | `0x2000` |
| `i2c` | `system.json5` | App count | 2 (server + client) | 3 (server + client + spdm_responder) |

The 384-byte increase in `vector_table_size_bytes` (0x500 → 0x680) is caused
by `PW_KERNEL_INTERRUPT_TABLE_ARRAY` growing from 0 entries to 113 entries
(IRQ 112 + 1) × 4 bytes = 452 bytes, placed in `.vector_table.interrupts`
inside the `VECTOR_TABLE` linker region.

### Files unique to openprot

- `services/i2c/docs/irq-notification-path-review.md` — this document

### Files unique to spdm-standup

- `services/i2c/mctp-transport/` — MCTP transport crate (not yet ported)

---

## pw_kernel Annotations — Clarification

`pw_kernel` annotations (`ThreadAnnotation`, `StackAnnotation`,
`ProcessAnnotation`) are **ELF-only debug metadata**. They are stored in
`(INFO)` sections at address `0x0` in the linker script, meaning they exist
in the ELF file but are **never loaded into RAM** at runtime:

```ld
.pw_kernel.annotations.stack 0x0 (INFO) { KEEP(*(.pw_kernel.annotations.stack.*)) }
.pw_kernel.annotations.thread 0x0 (INFO) { ... }
.pw_kernel.annotations.process 0x0 (INFO) { ... }
```

Each annotation is a small packed `#[repr(C)]` struct emitted by macros called
during kernel init (`annotate_thread_from_address!`, `annotate_stack!`,
`annotate_process_from_address!`). The `pw_kernel k` tooling parses these ELF
sections directly to reconstruct thread/stack/process names for debug output.

The `// includes vector table + annotations` comment in
`i2c-slave/system.json5` is therefore imprecise. The `vector_table_size_bytes`
growth is driven entirely by `PW_KERNEL_INTERRUPT_TABLE_ARRAY` in
`.vector_table.interrupts`, not by annotation sections.
