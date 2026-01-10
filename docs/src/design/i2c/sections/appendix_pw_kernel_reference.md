# pw_kernel Reference

This document provides definitions for key concepts and objects used in the
Pigweed kernel (pw_kernel).

## Core Concepts

### Object

An **Object** is the fundamental abstraction exposed to userspace by pw_kernel.
All kernel functionality is accessed through objects. Objects are polymorphic
and can be one of several types:

- **Channel** (Initiator or Handler)
- **Wait Group**
- **Interrupt**

Objects have:
- A set of **signals** that can be pending
- Methods that map to syscalls
- Lifecycle managed by the kernel (currently static allocation only)

### Handle

A **Handle** is a `u32` identifier that references a kernel object. Handles are
process-local, meaning handle `0` in process A may refer to a different object
than handle `0` in process B.

Handles are:
- Indexes into a process-local handle table
- Currently statically allocated at build time
- Generated from `system.json5` configuration

Example usage:
```rust
// handle::SERVER is a u32 generated from system.json5
syscall::channel_transact(handle::SERVER, &send, &mut recv, deadline)?;
```

### Signal

**Signals** are a 32-bit bitmask representing the state of an object. Each object
type defines which signals are meaningful and how they behave.

#### Common Signals (bits 0-15)

| Signal | Bit | Description |
|--------|-----|-------------|
| `READABLE` | 0 | Object has data available to read |
| `WRITEABLE` | 1 | Object is ready to accept writes |
| `ERROR` | 2 | Object is in an error state |
| `USER` | 15 | User-defined signal for custom notifications |

#### Interrupt Signals (bits 16-31)

| Signal | Bit | Description |
|--------|-----|-------------|
| `INTERRUPT_A` | 16 | First interrupt source fired |
| `INTERRUPT_B` | 17 | Second interrupt source fired |
| ... | ... | ... |
| `INTERRUPT_P` | 31 | Sixteenth interrupt source fired |

Signals are named by letter (A-P) rather than number to avoid confusion with
actual IRQ numbers.

### Process

A **Process** is an isolated execution environment containing:
- One or more **threads**
- A **handle table** mapping handles to kernel objects
- **Memory mappings** (code, data, device regions)

Processes provide memory isolation via MPU (ARM) or PMP (RISC-V).

### Thread

A **Thread** is a unit of execution within a process. Each thread has:
- Its own stack
- Scheduling state (ready, blocked, running)
- A reference to its owning process

### App

An **App** is the build-time configuration unit that produces a process at
runtime. Defined in `system.json5`, an app specifies:
- Name and memory sizes
- Objects (channels, interrupts)
- Threads
- Memory mappings

## Object Types

### Channel

A **Channel** is a unidirectional IPC connection between two asymmetric peers.
Channels support synchronous request/response transactions without intermediate
kernel buffers.

#### Channel Initiator

The **Initiator** side of a channel starts transactions:
- Calls `channel_transact()` to send a request and wait for response
- Can call `channel_async_transact()` for non-blocking operation
- Signals indicate transaction state

**Initiator Signals:**
| Signal | Meaning |
|--------|---------|
| `WRITEABLE` | No pending transaction, can start new one |
| `READABLE` | Handler has responded to pending transaction |
| `ERROR` | Pending transaction has an error |
| `USER` | Handler raised user signal via `raise_peer_user_signal()` |

#### Channel Handler

The **Handler** side of a channel responds to transactions:
- Calls `channel_read()` to read request data (zero-copy from initiator's buffer)
- Calls `channel_respond()` to complete the transaction
- Signals indicate pending transaction state

**Handler Signals:**
| Signal | Meaning |
|--------|---------|
| `READABLE` | Transaction pending, data available to read |
| `WRITEABLE` | Transaction pending, ready for response |
| `ERROR` | Transaction error (reserved for future use) |
| `USER` | Initiator raised user signal via `raise_peer_user_signal()` |

#### Channel Transaction Flow

```
Initiator                          Handler
    │                                  │
    │─── channel_transact() ──────────▶│ READABLE asserted
    │    (blocks)                      │
    │                                  │◀── channel_read()
    │                                  │    (copies from initiator buffer)
    │                                  │
    │◀── channel_respond() ───────────│
    │    (copies to initiator buffer) │
    │                                  │
    ▼ returns with response           ▼ READABLE cleared
```

### Wait Group

A **Wait Group** allows waiting on multiple objects simultaneously. Instead of
blocking on a single handle, you can:

1. Add handles to a wait group with `wait_group_add(handle, signals, user_data)`
2. Wait on the wait group with `object_wait(wait_group_handle, ...)`
3. Receive the `user_data` of whichever member signaled first

This enables the "wait on multiple event sources" pattern common in embedded
drivers (e.g., wait for either IPC request OR interrupt).

### Interrupt

An **Interrupt** object provides userspace access to hardware interrupts:
- Can handle up to 16 IRQ sources per object
- Each IRQ maps to an `INTERRUPT_A` through `INTERRUPT_P` signal
- Kernel auto-masks IRQ when it fires
- Userspace calls `interrupt_ack()` to acknowledge and re-enable

**Interrupt Flow:**
```
Hardware IRQ fires
    │
    ▼
Kernel masks IRQ, raises INTERRUPT_x signal
    │
    ▼
Userspace wakes from object_wait()
    │
    ▼
Userspace handles interrupt
    │
    ▼
Userspace calls interrupt_ack(INTERRUPT_x)
    │
    ▼
Kernel unmasks IRQ (can fire again)
```

**Note:** Unlike some RTOSes, pw_kernel does NOT provide `sys_irq_control()`
style syscalls. IRQ enable/disable is managed automatically by the kernel.

## System Configuration

### system.json5

The **system.json5** file defines the static configuration of a pw_kernel
system, including:

```json5
{
  arch: { type: "riscv" | "armv8m" | "armv7m" },
  kernel: {
    flash_start_address: 0x...,
    ram_start_address: 0x...,
    // ...
  },
  apps: [
    {
      name: "my_app",
      flash_size_bytes: ...,
      ram_size_bytes: ...,
      process: {
        objects: [
          { name: "IPC", type: "channel_handler" },
          { name: "IRQ", type: "interrupt", irqs: [...] },
        ],
        threads: [...],
        memory_mappings: [...],
      },
    },
  ],
}
```

### target_codegen

The **target_codegen** build tool processes `system.json5` and generates:
- Rust code with handle constants (`handle::IPC`, `handle::SERVER`)
- Kernel initialization code for objects
- Memory layout information

## Syscall Reference

### Generic Syscalls

| Syscall | ID | Description |
|---------|-----|-------------|
| `object_wait` | 0x0000 | Block until signals asserted or deadline expires |
| `raise_peer_user_signal` | 0x0005 | Raise USER signal on channel peer |

### Channel Initiator Syscalls

| Syscall | ID | Description |
|---------|-----|-------------|
| `channel_transact` | 0x0001 | Synchronous request/response transaction |
| `channel_async_transact` | — | Non-blocking transaction start |
| `channel_async_cancel` | — | Cancel pending async transaction |

### Channel Handler Syscalls

| Syscall | ID | Description |
|---------|-----|-------------|
| `channel_read` | 0x0002 | Read request data from initiator |
| `channel_respond` | 0x0003 | Complete transaction with response |

### Interrupt Syscalls

| Syscall | ID | Description |
|---------|-----|-------------|
| `interrupt_ack` | 0x0004 | Acknowledge interrupt(s) and re-enable |

### Debug Syscalls (0xF0xx)

| Syscall | ID | Description |
|---------|-----|-------------|
| `debug_putc` | 0xF000 | Output a single character |
| `debug_shutdown` | 0xF001 | Terminate execution (for tests) |
| `debug_log` | 0xF002 | Log a message |
| `debug_nop` | 0xF003 | No operation |
| `debug_trigger_interrupt` | 0xF004 | Trigger software interrupt (testing) |

## Comparison with Hubris

For developers familiar with Hubris, here's a mapping of key concepts:

| Hubris | pw_kernel | Notes |
|--------|-----------|-------|
| Task | App/Process | Static configuration |
| `sys_send` | `channel_transact` | Client → Server request |
| `sys_recv_open` | `object_wait` + `channel_read` | Wait for + read IPC |
| `sys_reply` | `channel_respond` | Server → Client response |
| `sys_post` | `raise_peer_user_signal` | Async notification |
| `sys_irq_control` | — | No equivalent; auto-managed |
| `sys_borrow_*` | — | Kernel-mediated copy instead |
| Notification mask | `Signals::USER` | Per-object, not per-task |

## Memory Protection

### ARM Cortex-M (MPU)

- **PMSAv8** (ARMv8-M): Flexible base/limit regions, MAIR attributes
- **PMSAv7** (ARMv7-M): Power-of-2 regions with sub-region disable (SRD)

### RISC-V (PMP)

- Physical Memory Protection with NAPOT (naturally aligned power-of-two) regions

## References

- [pw_kernel documentation](https://pigweed.dev/pw_kernel/)
- [syscall_defs.rs](../syscall/syscall_defs.rs) - Authoritative syscall definitions
- [ARM Architecture Reference Manual](https://developer.arm.com/documentation)
- [RISC-V Privileged Specification](https://riscv.org/specifications/privileged-isa/)
