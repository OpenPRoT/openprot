# Memory-Mapped I/O (MMIO) Policy

This document defines how code in `target/ast10x0/` may access memory-mapped
hardware registers. It applies to people and to AI coding and review agents.
Raw MMIO is allowed only at one boundary, under stated invariants. Unsafe MMIO
that escapes the boundary is a violation.

Scope today is `target/ast10x0/`. The pattern is target-neutral, and
`target/veer/` and `target/earlgrey/` can adopt it later.

## Rules in one screen

1. Raw register access lives only in a per-peripheral `registers.rs`
   (`target/ast10x0/peripherals/<peripheral>/registers.rs`).
2. `registers.rs` exposes safe, typed methods. No PAC types, `ptr()` results,
   or raw pointers cross its public API.
3. New HALs and drivers receive exclusive ownership of a peripheral by value
   (an owned token). They do not call `unsafe` constructors or `ptr()`.
4. Every `unsafe` block carries a `// SAFETY:` comment that names the invariant
   it relies on.

## Patterns in use today

AST10x0 registers are accessed through the `ast1060_pac` crate (an
svd2rust-style PAC, pinned in `third_party/ast1060-pac/versions.bzl`). Nothing
in the tree calls `Peripherals::take()`. Ownership is a `# Safety` comment at
each call site. The patterns found, best to worst:

| # | Pattern | Example | Verdict |
|---|---------|---------|---------|
| 1 | Register wrapper: `!Send + !Sync` struct over `*const RegisterBlock`, `unsafe fn new_global()`, safe `regs()` | `scu/registers.rs`, `gpio/registers.rs`, `hace/registers.rs`, `i3c/registers.rs` | Boundary form. Keep. The `unsafe` constructor is legacy, see rule 3. |
| 2 | Inline `unsafe { &*Pac::ptr() }` in driver code | `gpio/mod.rs` (about 14 times), `i2c/types.rs`, `i2c/global.rs`, `scu/routing.rs`, `board/src/spim_wiring.rs` | Violation. |
| 3 | `Pac::steal()` | `entry.rs` (`jtag_init`) | Allowed only as a documented early-boot exception. |
| 4 | Const pointer such as `&*Secure::PTR` | `otp/mod.rs` | Violation. |
| 5 | Index-to-`ptr()` table | `backend/i2c/src/lib.rs` | Violation, covered by the baseline. |
| 6 | Raw `read_volatile` / `write_volatile` or hard-coded address casts | `smc/controller.rs`, `pfr/swmbx_ctrl.rs`, `console_backend.rs` (`0x7e78_4000 as *const _`), `backend/usart/src/lib.rs` | Violation. |
| 7 | `ptr()` in tests | `tests/**/target.rs` | Violation, covered by the baseline. |

Existing violations are listed in `.semgrep/ast10x0-mmio.yml` (the baseline).
The baseline only shrinks. Do not add files to it.

## The canonical pattern

### Boundary

The only places that may call `Pac::ptr()`, dereference a register-block
pointer, or use volatile accesses are the `registers.rs` files under
`target/ast10x0/peripherals/`. Everything above them uses the safe methods.

```rust
// peripherals/gpio/registers.rs  (the boundary)
pub struct GpioRegisters {
    base: *const device::gpio::RegisterBlock,
    // MMIO handles must not cross threads or be shared by reference.
    _not_send_sync: PhantomData<*const ()>,
}

impl GpioRegisters {
    /// Set the direction bit for `pin`.
    pub fn set_output(&self, pin: u8) {
        // ... safe typed access over self.regs() ...
    }

    fn regs(&self) -> &device::gpio::RegisterBlock {
        // SAFETY: the constructor guarantees `base` points to the GPIO block,
        // which is mapped for the lifetime of the device.
        unsafe { &*self.base }
    }
}
```

### Ownership token

Each peripheral has one owner. The model follows `embassy-stm32`,
`embassy-nrf`, and `esp-hal`:

- A single `Peripherals::take()` (guarded by an atomic flag) hands out one
  zero-sized token per peripheral. `unsafe Peripherals::steal()` is the only
  escape hatch, for example for early boot or a panic handler.
- A HAL or driver takes its token by value, for example
  `Gpio::new(p.gpio)`. Exclusivity is then checked by the compiler, and the
  constructor needs no `unsafe`.

Until the token exists in this repo, new code uses the wrapper from pattern 1
and documents who owns the peripheral. Do not add new `pub unsafe fn
new_global()` constructors outside the boundary.

### Shared blocks

The SCU (pinmux, clocks, resets, write-protect unlock) is touched by many
drivers. Model it as a short-lived borrowed handle, or a handle guarded by a
critical section. Do not hold several long-lived `ScuRegisters` values.

## Invariants at the boundary

Every `unsafe` in a `registers.rs` file justifies, in its `// SAFETY:` comment,
the invariants that apply:

1. **Address.** The pointer is the documented base of the register block and
   is mapped.
2. **Exclusivity.** No other code accesses the registers concurrently, because
   the type is a singleton or access is serialized.
3. **Width and alignment.** Accesses use the width and alignment the block
   requires.
4. **No lingering references.** A `&` or `&mut` to MMIO does not outlive the
   access. Do not store one in a struct.
5. **Ordering and side effects.** Reads and writes are volatile. Reads with side
   effects (FIFO pop, clear-on-read) are not repeated or elided.
6. **Hardware protocol.** Sequences such as the SCU unlock key `0x1688_A8A8`
   are honored.

## Prohibited outside the boundary

- `Pac::ptr()`, `&*Pac::ptr()`, and `Pac::PTR`.
- `Pac::steal()`, except the allowlisted early-boot use in `entry.rs`.
- `read_volatile`, `write_volatile`, and integer-to-pointer casts used to reach
  registers.
- `pub unsafe fn new_global()` (or similar) on new HALs.
- PAC types in the public API of a driver or HAL.
- `unsafe` without a `// SAFETY:` comment.

### Bad and good

```rust
// Bad: raw MMIO in a driver. Ownership is unchecked, and the
// unsafe block is repeated at every call site.
pub fn into_pull_down_input(self) -> Pin<Input<PullDown>> {
    let p = unsafe { &*device::Gpio::ptr() };
    p.dir().modify(|r, w| unsafe { w.bits(r.bits() & !(1 << PIN)) });
    Pin { _mode: PhantomData }
}

// Good: the driver calls a safe method. The unsafe lives once, in registers.rs.
pub fn into_pull_down_input(self, regs: &GpioRegisters) -> Pin<Input<PullDown>> {
    regs.set_input(PIN);
    Pin { _mode: PhantomData }
}
```

```rust
// Bad: constructor that asks the caller to promise exclusivity.
let dev = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };

// Good (target form): ownership is proven by moving the token in.
let dev = HaceDevice::new(peripherals.hace, |_| core::hint::spin_loop());
```

## Enforcement

| Check | Tool | Catches |
|-------|------|---------|
| `ptr()`, `PTR`, `steal()`, volatile, `new_global` outside `registers.rs` | Semgrep, `.semgrep/ast10x0-mmio.yml`, CI | The prohibited patterns above |
| `unsafe` without `// SAFETY:` | Clippy `undocumented_unsafe_blocks` | Missing justification |
| Raw volatile in new code | Clippy `disallowed_methods` | Direct `read_volatile` / `write_volatile` |
| Does the ownership design make sense? Is the `SAFETY` comment true? | Human and AI review, using `.github/copilot-instructions.md` | What tools cannot decide |

CI fails only on violations outside the baseline. A review agent that sees a
new violation reports it with a pointer to this document.

## Adding or changing a peripheral

1. Put raw access in `peripherals/<peripheral>/registers.rs`, behind safe typed
   methods.
2. Expose a constructor that takes ownership. Avoid `unsafe fn new_global()`.
3. Keep drivers and services free of PAC types and `unsafe` register access.
4. Document who owns the peripheral and how shared blocks (SCU) are borrowed.
5. Run `./pw presubmit` and the Semgrep rules before pushing.
