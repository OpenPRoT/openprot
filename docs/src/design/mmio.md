# Memory-Mapped I/O (MMIO) Policy

This document defines how OpenPRoT code may access memory-mapped hardware
registers. It applies to people and to AI coding and review agents. Raw MMIO is
allowed only at one boundary, under stated invariants. Unsafe MMIO that escapes
the boundary is a violation.

The rules here are target-neutral. A target adds its own rules, such as where
its boundary is, which register crate it uses, its exceptions, and how its
checks run, in `target/<name>/MMIO.md`. Read that file too when it exists. Where
the two differ, the target file is more specific and wins.

## Rules

1. Raw register access lives only in a designated boundary module for each
   peripheral. The target's `MMIO.md` names it.
2. The boundary exposes safe, typed methods. Raw pointers and register-crate
   types do not cross its public API.
3. Drivers and services use the safe API. They do not reach registers
   themselves.
4. Exclusive ownership of a peripheral is expressed in types where possible (an
   owned handle moved into the driver), not only in a `# Safety` comment.
5. Every `unsafe` block has a `// SAFETY:` comment directly above it that names
   the invariant it relies on.

## Invariants at the boundary

Each `unsafe` in the boundary justifies, in its `// SAFETY:` comment, the
invariants that apply:

1. **Address.** The pointer is the documented base of the register block and is
   mapped.
2. **Exclusivity.** No other code accesses the registers concurrently, because
   the handle is a singleton or access is serialized.
3. **Width and alignment.** Accesses use the width and alignment the block
   requires.
4. **No lingering references.** A `&` or `&mut` to MMIO does not outlive the
   access. Do not store one in a struct.
5. **Ordering and side effects.** Reads and writes are volatile. Reads with side
   effects (FIFO pop, clear-on-read) are not repeated or elided.
6. **Hardware protocol.** Required sequences, such as unlock keys, are honored.
7. **Aliases.** A second handle to the same block (for example an interrupt
   handler's) needs a `// SAFETY:` comment that says why access is serialized.
   An `unsafe impl Send` on a type that holds an MMIO handle needs the same
   justification and a reviewer's explicit sign-off.

## Prohibited outside the boundary

- Dereferencing a register-block pointer, and calling the register crate's
  pointer or "steal" accessors.
- `read_volatile`, `write_volatile`, and integer-to-pointer casts used to reach
  registers.
- A new `pub unsafe fn new*()` constructor on a driver, whatever its name.
- Register reads and writes through a register-block reference obtained from an
  accessor outside the boundary. Add a typed method to the boundary instead.
- Raw pointers or register-crate types in the public API of a driver.
- `unsafe` without a `// SAFETY:` comment directly above it that names the
  invariant. A comment separated from the block by other code does not count.

Not MMIO, and not to be flagged as such: DMA buffer addresses, static buffers
reached through `addr_of_mut!`, interrupt controller calls, and `#[cfg(test)]`
host tests that run a driver over a fake register buffer.

### Bad and good

```rust
// Bad: raw MMIO in a driver. Ownership is unchecked, and the unsafe block is
// repeated at every call site.
let regs = unsafe { &*pac::Uart::ptr() };
regs.ctrl().modify(|r, w| unsafe { w.bits(r.bits() | ENABLE) });

// Good: the driver calls a safe method. The unsafe lives once, in the boundary.
uart_regs.enable();
```

```rust
// Bad: a constructor that asks the caller to promise exclusivity.
let dev = unsafe { Device::new_global() };

// Good: ownership is proven by moving the handle in.
let dev = Device::new(peripherals.device);
```

## Enforcement

| Check | Tool | Catches |
|-------|------|---------|
| Raw pointers, volatile, address casts, and unsafe constructors outside the boundary | Target-specific CI check, described in the target's `MMIO.md` | The mechanical prohibited patterns above |
| `unsafe` without `// SAFETY:` | Clippy `undocumented_unsafe_blocks`, where a crate enables it | Missing justification |
| Is the ownership design sound? Is the `SAFETY` comment true? | Human and AI review, using `.github/copilot-instructions.md` | What tools cannot decide |

Automated checks fail only on violations that a change introduces. Legacy
violations are tolerated, but **added lines are held to this policy wherever
they are**, including new lines in files that already violate it. A review that
finds a new violation reports it with a pointer to this document and the
target's `MMIO.md`.

## Adding or changing a peripheral

1. Put raw access in the target's boundary module, behind safe typed methods.
2. Expose a constructor that takes ownership. Avoid `unsafe` constructors.
3. Keep drivers and services free of register-crate types and `unsafe` register
   access.
4. Document who owns the peripheral and how shared blocks are borrowed.
5. Run `./pw presubmit` and the target's MMIO check before pushing.
