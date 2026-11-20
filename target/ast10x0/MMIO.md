# AST10x0 MMIO Policy

This file adds the AST10x0 rules to the general
[MMIO policy](../../docs/src/design/mmio.md). Read that first: the invariants,
the prohibited patterns, and the "added lines are held to the policy" rule apply
here unchanged. This file only says where the boundary is on AST10x0, what the
legacy code looks like, and how the checks run.

## Boundary

Registers are accessed through `ast1060_pac`, an svd2rust-style PAC pinned in
`third_party/ast1060-pac/versions.bzl`. The boundary is the per-peripheral
`registers.rs` under `peripherals/<peripheral>/`. Only those files may call
`Pac::ptr()`, use `Pac::PTR`, dereference a register-block pointer, or use
volatile accesses. Everything above them uses the safe methods.

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

Specific to this target:

- `Pac::steal()` is allowed only in `entry.rs` (`jtag_init`), before the kernel
  runs, with a `SAFETY` comment.
- `regs()` is `pub(crate)` on several wrappers today. New code adds a typed
  method instead of calling `regs().xxx().write(|w| unsafe { w.bits(..) })`
  from a driver. Inside `registers.rs`, `bits()` closures are the normal way to
  write.
- The wrapper is `!Send + !Sync` (`PhantomData<*const ()>`), as `ScuRegisters`
  and `GpioRegisters` do.

## Ownership

The target form is one owned token per peripheral (`Peripherals::take()` guarded
by an atomic flag, `unsafe steal()` as the escape hatch), as in embassy-stm32,
embassy-nrf and esp-hal. It does not exist yet (#756). Until it does:

- Calling an existing `unsafe` wrapper constructor (`new_global*()`,
  `Backend::new()`) is allowed when a `// SAFETY:` comment names who owns the
  peripheral.
- Do not define a new `pub unsafe fn new*()` constructor on a HAL or driver
  outside `registers.rs`, whatever its name.
- A new `registers.rs` may have one `pub(crate) unsafe` constructor, because it
  is the only place a pointer can be created. It must not be `pub`, and no raw
  pointer or PAC type may appear in a public signature.
- When a token exists for the peripheral, use it instead of the constructor.

The SCU (pinmux, clocks, resets, write-protect unlock) is shared by many
drivers. Do not hold several long-lived `ScuRegisters` values. The shared-access
design is tracked in #757.

## Invariants specific to AST10x0

- **Hardware protocol.** SCU write-protected registers need the unlock key
  `0x1688_A8A8` written to `SCU000` first.
- **Aliases.** The i3c driver's `isr_alias` creates a second handle for the same
  bus and moves it to the interrupt handler with `unsafe impl Send`. That kind
  of alias needs the `SAFETY` justification and sign-off the general policy asks
  for.

Not MMIO here: `buf.as_ptr() as u32` handed to a DMA engine, static buffers
reached through `addr_of_mut!`, and `NVIC::unmask`.

## Patterns in use today

Nothing in the tree calls `Peripherals::take()`. Ownership is a `# Safety`
comment at each call site. The patterns found, best to worst:

| # | Pattern | Example | Verdict |
|---|---------|---------|---------|
| 1 | Register wrapper: `!Send + !Sync` struct over `*const RegisterBlock`, `unsafe fn new_global()`, safe `regs()` | `scu/registers.rs`, `gpio/registers.rs`, `hace/registers.rs`, `i3c/registers.rs` | Boundary form. Keep. The `unsafe` constructor is legacy. |
| 2 | Inline `unsafe { &*Pac::ptr() }` in driver code | `gpio/mod.rs` (about 14 times), `i2c/types.rs`, `i2c/global.rs`, `scu/routing.rs`, `board/src/spim_wiring.rs` | Violation. |
| 3 | `Pac::steal()` | `entry.rs` (`jtag_init`) | Allowed only as the early-boot exception above. |
| 4 | Const pointer such as `&*Secure::PTR` | `otp/mod.rs` | Violation. |
| 5 | Index-to-`ptr()` table | `backend/i2c/src/lib.rs` | Violation, legacy. |
| 6 | Raw `read_volatile` / `write_volatile` or hard-coded address casts | `smc/controller.rs`, `pfr/swmbx_ctrl.rs`, `console_backend.rs` (`0x7e78_4000 as *const _`), `backend/usart/src/lib.rs` | Violation, legacy. |
| 7 | `ptr()` in tests | `tests/**/target.rs` | Violation, legacy. See the enforcement section. |

### Bad and good

```rust
// Bad: raw MMIO in a driver (gpio/mod.rs today).
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

The Semgrep rule [`mmio.semgrep.yml`](mmio.semgrep.yml) flags `Pac::ptr()`,
`Pac::PTR`, `Pac::steal()`, `read_volatile` / `write_volatile`, hard-coded
address casts (`0x... as *const T`), and `pub unsafe fn new*` outside
`registers.rs`. The
[`ast10x0-mmio.yml`](../../.github/workflows/ast10x0-mmio.yml) workflow runs it
on pull requests that change `target/ast10x0/`. It compares against the base
commit, so only violations the pull request introduces fail, including new lines
in a file that already has legacy ones.

What it does not cover, so review checks it:

- `tests/` and `entry.rs` are not scanned. No accessor reachable from a test
  exists yet (#755). New tests should still use the safe wrappers where they can.
- Calls to existing unsafe constructors, `regs()` accessors, aliases, and
  missing `SAFETY` comments (Clippy lints are not enabled yet, #754).
- When a new violation is textually identical to existing ones in the same file,
  Semgrep reports the extra match but may point at an earlier identical line.

Run it before pushing, from the repository root. The root `.semgrepignore` skips
`target/`, so the flag below ignores it:

```console
semgrep --config target/ast10x0/mmio.semgrep.yml --error \
  --x-ignore-semgrepignore-files --baseline-commit origin/main target/ast10x0
```

## Adding or changing a peripheral

1. Put raw access in `peripherals/<peripheral>/registers.rs`, behind safe typed
   methods.
2. Keep drivers, services and backends free of PAC types and `unsafe` register
   access.
3. Prefer taking ownership over an `unsafe` constructor (see Ownership).
4. Run `./pw presubmit` and the Semgrep command above.
