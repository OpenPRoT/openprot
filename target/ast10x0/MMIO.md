# Memory-Mapped I/O (MMIO) Policy

This document defines how code in `target/ast10x0/` may access memory-mapped
hardware registers. It applies to people and to AI coding and review agents.
Raw MMIO is allowed only at one boundary, under stated invariants. Unsafe MMIO
that escapes the boundary is a violation.

This policy covers `target/ast10x0/` only. It names AST10x0 crates and paths
(`ast1060_pac`, `peripherals/<peripheral>/registers.rs`), and the legacy
examples below come from this tree. Other targets decide their own policy and
keep it in their own `target/<name>/` directory.

## Rules in one screen

1. Raw register access lives only in a per-peripheral `registers.rs`
   (`target/ast10x0/peripherals/<peripheral>/registers.rs`).
2. `registers.rs` exposes safe, typed methods. No PAC types, `ptr()` results,
   or raw pointers cross its public API.
3. New HALs and drivers receive exclusive ownership of a peripheral by value
   (an owned token). They do not call `ptr()` or define new `unsafe` constructors.
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
| 5 | Index-to-`ptr()` table | `backend/i2c/src/lib.rs` | Violation, legacy. |
| 6 | Raw `read_volatile` / `write_volatile` or hard-coded address casts | `smc/controller.rs`, `pfr/swmbx_ctrl.rs`, `console_backend.rs` (`0x7e78_4000 as *const _`), `backend/usart/src/lib.rs` | Violation. |
| 7 | `ptr()` in tests | `tests/**/target.rs` | Violation, legacy. CI does not scan `tests/`, because no sanctioned accessor is reachable from a test yet. New tests should use the safe wrappers where they can, and reviewers flag new `ptr()` calls. |

Legacy violations are tolerated, but a change must not add to them. CI compares
a pull request against its base commit, so a new violation is reported even in a
file that already has some, and so is a rewritten or newly added file at a legacy
path. Review applies the same rule: **added lines are held to this policy
wherever they are.** When you fix a legacy violation, nothing else needs to
change.

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

Until the token exists in this repo:

- Calling an existing `unsafe` wrapper constructor (`new_global*()`,
  `Backend::new()`) is allowed when a `// SAFETY:` comment names who owns the
  peripheral.
- Do not define a new `pub unsafe fn new*()` constructor on a HAL or driver
  outside `registers.rs`, whatever its name.
- A new `registers.rs` may have one `pub(crate) unsafe` constructor, because it
  is the only place a pointer can be created. It must not be `pub`, and no
  raw pointer or PAC type may appear in a public signature.
- When a token exists for the peripheral, use it instead of the constructor.

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
7. **Aliases.** A second handle to the same block (for example an ISR alias)
   needs a `// SAFETY:` comment that says why access is serialized. An
   `unsafe impl Send` on a type that holds an MMIO handle needs the same
   justification and a reviewer's explicit sign-off.

### Not MMIO

These are not register access, and a review should not flag them as such: DMA
buffer addresses (`buf.as_ptr() as u32`), static buffers reached through
`addr_of_mut!`, `NVIC::unmask`, and `#[cfg(test)]` host tests that run a driver
over a fake register buffer without `ptr()` or `PTR`.

## Prohibited outside the boundary

- `Pac::ptr()`, `&*Pac::ptr()`, and `Pac::PTR`.
- `Pac::steal()`, except the allowlisted early-boot use in `entry.rs`.
- `read_volatile`, `write_volatile`, and integer-to-pointer casts used to reach
  registers.
- A new `pub unsafe fn new*()` constructor on a HAL or driver, whatever its
  name (`new_global`, `new`, `new_with_regions`, ...).
- Register reads and writes through a PAC block reference outside
  `registers.rs`, including `regs().xxx().write(|w| unsafe { w.bits(..) })`
  through a `pub(crate) regs()` accessor. Add a typed method to `registers.rs`
  instead. Inside `registers.rs`, `bits()` closures are the normal way to write.
- PAC types or raw pointers in the public API of a driver or HAL.
- `unsafe` without a `// SAFETY:` comment directly above it that names the
  invariant. A comment separated from the block by an `if` or other code does
  not count.

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
| `ptr()`, `PTR`, `steal()`, volatile, address casts, `pub unsafe fn new*` outside `registers.rs` | Semgrep, `.semgrep/ast10x0-mmio.yml`, CI (new violations only) | The mechanical prohibited patterns above |
| `unsafe` without `// SAFETY:` | Clippy `undocumented_unsafe_blocks` (planned, per crate, not yet enabled) | Missing justification. Until then, review checks it. |
| Raw volatile in new code | Clippy `disallowed_methods` (planned, not yet enabled) | Covered by Semgrep meanwhile |
| Does the ownership design make sense? Is the `SAFETY` comment true? | Human and AI review, using `.github/copilot-instructions.md` | What tools cannot decide |

CI fails only on violations that a pull request introduces. It does not scan
`tests/`, `entry.rs`, or `registers.rs`. It cannot see `regs()` accessors, calls to
existing unsafe constructors, aliases, or missing `SAFETY` comments, so review
checks those. A review agent that sees a
new violation reports it with a pointer to this document.

## Adding or changing a peripheral

1. Put raw access in `peripherals/<peripheral>/registers.rs`, behind safe typed
   methods.
2. Expose a constructor that takes ownership. Avoid `unsafe fn new_global()`.
3. Keep drivers and services free of PAC types and `unsafe` register access.
4. Document who owns the peripheral and how shared blocks (SCU) are borrowed.
5. Run `./pw presubmit`, and check your change with
   `semgrep --config .semgrep/ast10x0-mmio.yml --baseline-commit origin/main target/ast10x0`
   before pushing.
