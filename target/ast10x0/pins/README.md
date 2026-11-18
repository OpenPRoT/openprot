# The Pin & Region Token Model

This document explains how the AST10x0 target decides which process gets to
touch which piece of hardware, and — just as importantly — how sure we
actually are that the decision holds. It covers the AST10x0 SCU pin mux,
GPIO driving, and the memory-region tokens that gate both. The generated
code in this directory (`pin_claims.rs.jinja`, `kernel_pin_routes.rs.jinja`)
is the part that actually grants authority; the mechanism it plugs into lives
in `../../../hal/src/` (`resource.rs`, `field_mux.rs`, `gpio.rs`),
`../peripherals/scu/`, and `../../../util/region/`.

Read this if you're adding a new pin, a new peripheral capability, or
reviewing a change to any of the above. The second half is written the way a
skeptical reviewer should read the first half: for every guarantee claimed,
what's actually been verified, and where the guarantee runs out.

## The problem, in plain terms

Most pins aren't wired to one fixed job. A crossbar switch inside the chip (on
AST10x0, part of the **SCU**, the System Control Unit) can route a given pin
to one of several functions: GPIO, I2C, UART, and so on. Once a pin is routed
to GPIO, driving it is just reading and writing a couple of registers —
numbered "mailboxes" at fixed memory addresses — that say things like "this
pin's bit in the output latch" or "this pin's bit in the direction register."

Here's the risk that motivates everything below: if *any* code in a process
can poke *any* register at *any* address, then a bug in one driver can
silently take over a pin that belongs to a completely different piece of
code. A typo that writes to the wrong offset, or copies the wrong bit
position, doesn't fail loudly — it just quietly drives somebody else's pin.
On a security-critical root-of-trust chip, "somebody else's pin" might be a
reset line, a chip-select, or a line into the very device this chip is
supposed to be protecting.

The fix this codebase uses is to make "which pin, doing what" a fact the
*compiler* checks, not a fact a driver author has to get right by hand.

## The golden path: writing a GPIO driver

Every physical pin this chip exposes is declared once, in
`../peripherals/scu/pins.rs`, using the `pins!` macro:

```rust
openprot_hal::pins! {
    scu410_0 { Gpio: &[clear(0x410, 0), clear(0x4b0, 0), clear(0x690, 0)] => GpioData { bit: 0, map: &ABCD } },
    // ...
}
```

This expands to a zero-sized struct `scu410_0` — a Rust value with no
runtime bytes, whose only purpose is to exist or not exist. Each pin also
gets an `impl Routes<Gpio> for scu410_0`, which carries two things as
*compile-time constants*, not runtime data:

- `ROUTE`: the exact register writes (`FieldWrite`s) that mux this physical
  pin to GPIO — e.g. clear bit 0 of the register at offset `0x410`.
- `DATA`: a `GpioData { bit, map }` — which bit this pin occupies in its GPIO
  register group, and a pointer to that group's register layout (`ABCD`,
  `EFGH`, or `IJKL` — AST10x0 groups its 32 GPIO pins into three register
  banks).

A driver binds a pin like this:

```rust
let gpio = GpioBlock::new(mmaps.gpio_regs);
let mut a0 = bind_gpio(pins.scu410_0, &gpio).into_input();
let level = a0.is_high()?;
```

`pins.scu410_0` is a `scu410_0` value the driver already owns (more on how it
got one below). `bind_gpio` consumes it and the `GpioBlock` (the GPIO
peripheral's unlocked register access) to produce a `GpioPin` — the one type
with `.is_high()`, `.set_high()`, `.into_output()`, and so on.

That's the whole golden path. The interesting part is everything that makes
`pins.scu410_0` exist in the first place, and everything that makes
`GpioPin`'s methods only ever touch *this* pin's bits.

## Tokens: ownership is free, authority is not

Because `scu410_0` is a real Rust type, Rust's own move semantics do useful
work for free: `bind_gpio` *consumes* the pin, so once it's bound you can't
also hand it to a second driver — the compiler rejects a double-use the same
way it rejects using a `String` after moving it. Two drivers racing to
control the same physical pin is a compile error, not a runtime bug.

But that only matters if getting a `scu410_0` value in the first place is
restricted. A public, parameterless constructor would make this whole model
decorative — anyone could write `scu410_0 { }`-equivalent code and claim the
pin from nothing. So pins don't have a public constructor. They have `take`:

```rust
pub const unsafe fn take<A: AuthorizesPin<Self>>(_auth: &A) -> Self
```

`take` requires a reference to *some* type `A` that implements
`AuthorizesPin<scu410_0>`.
[`AuthorizesPin<P>`](../../../hal/src/resource.rs) is an empty marker trait —
it carries no data, it's purely a yes/no fact the compiler can check: does
*some* type, anywhere this code can see, claim to authorize minting pin `P`?

## The manifest is the only source of authority

Who gets to write `impl AuthorizesPin<scu410_0> for Authority {}`? Generated
code, and only generated code, driven by `system.json5`:

```json5
constants: [{
    type: "string",
    name: "pin_claims_gpio_smoke_process",
    value: "scu410_0:gpio,scu410_1:gpio,scu410_3:gpio,scu410_4:gpio",
}],
```

`pin_claims.rs.jinja` (in this directory) reads this string, and for
`gpio_smoke_process` specifically, emits a crate containing exactly:

```rust
pub struct Authority;
impl openprot_hal::resource::AuthorizesPin<ast10x0_peripherals::scu::pins::scu410_0> for Authority {}
impl openprot_hal::resource::AuthorizesPin<ast10x0_peripherals::scu::pins::scu410_1> for Authority {}
// ... scu410_3, scu410_4, and no others

pub const unsafe fn take_pin_claims() -> PinClaims {
    PinClaims { scu410_0: unsafe { scu410_0::take(&Authority) }, /* ... */ }
}
```

A process's own `main.rs` calls `take_pin_claims()` once, at startup, and
gets back exactly the pins its own manifest entry named — nothing else. If a
driver author tries to write `pins.scu414_28` (a pin this process never
claimed), there is no `PinClaims` field by that name: it's a compile error,
caught the moment someone writes the wrong pin, not the moment the wrong pin
gets driven on real hardware. The manifest and the code cannot silently
drift apart, because the code *is* generated from the manifest.

The claim string also carries a capability tag (`scu410_0:gpio`), validated
against a fixed list in
`../../../util/hw_claim/manifest_check.rs` (`VALID_CAPABILITY_TAGS`) so a
typo'd or unsupported capability fails the manifest check rather than
silently being ignored. The app side only ever needs the pin name —
`pin_claims.rs.jinja` strips the tag back off — but the *kernel's own*
boot-time routing (next section, in `kernel_pin_routes.rs.jinja`) needs the
tag, to know which `Routes<C>` impl to flush for that pin.

## Flushing the mux: one write per register, one call per board

Pins also need to be *routed* — their `ROUTE` writes actually applied to the
SCU — before any process can usefully drive them. This happens once, from
privileged kernel boot code, via `ast10x0_peripherals::scu::route()`, called
from `kernel_pin_routes.rs.jinja`'s generated `route_claimed_pins()`, which
hands every claimed pin across the whole system to `route()` as one tuple:

```rust
ast10x0_peripherals::scu::route((
    openprot_hal::field_mux::Route::<Gpio, _, ScuBlock>::new(unsafe { scu410_0::take(&KernelAuthority) }),
    // every other claimed pin, system-wide
));
```

Two pins can share a register (two bits of the same `0x410` word, say).
Routing them independently would mean two separate read-modify-write cycles
racing each other. Instead, every pin's `ROUTE` is folded — *at compile
time* — into one [`RegBatch`](../../../hal/src/field_mux.rs): one op per
distinct register offset, with per-bit writes merged in. If two pins'
`ROUTE`s ever disagreed about the same bit (one sets it, one clears it), that
merge step (`FieldWrite::coalesce`) const-panics — a genuine compile error,
not a race you'd only find by testing both pins together on real silicon.

`RegBatch<B>` is generic over a **block** `B` — a type like `ScuBlock` that
names a real, fixed hardware register block (its base address and its real
byte size, read straight from the PAC's own generated layout). `Mmio<B>` is
the one thing that can actually read or write registers for block `B`.
Tying `RegBatch<B>` and `Mmio<B>` to the same `B` is what stops a batch built
for the SCU from ever being flushed against, say, the GPIO controller by
mistake — the types simply don't match, so it doesn't compile.

## Proving an offset belongs to its block — without a runtime check

Here's a question worth asking about any of this: register offsets are just
`u32`s. What stops a `ROUTE` (or a bug in one) from naming an offset that's
actually outside the real SCU register block?

The answer is [`ProvenInBounds`](../../../hal/src/field_mux.rs), and the
*reason* it's shaped the way it is matters more than the mechanism itself.

A first attempt put the bounds check inside `RegBatch`'s own `push` method,
as a plain `assert!`. That looks reasonable, but it's wrong in a way that's
easy to miss: `push` is a `const fn`, and a `const fn`'s evaluation timing
depends entirely on *how it's called*. Called from a `const` item's
initializer, it runs at compile time. Called from ordinary code, it runs —
and can panic — at runtime, on real hardware, like any other function. We
proved this isn't hypothetical: a test that called `push` from an ordinary
`#[test] fn` made its `assert!` fire at actual runtime, in the test harness.
A `const fn` offers no promise about *where* it runs; only about where it
*can*.

An **associated const** is different. The compiler is required to fully
evaluate a `const` item's initializer — including every function it calls —
at compile time, every time, with no exceptions, regardless of what kind of
code references it afterward. So the bounds proof lives there instead:

```rust
pub trait ProvenInBounds<B: Block>: MuxRoutes<B> {
    const PROVEN: ();
}

impl<B: Block, H: MuxRoutes<B>> ProvenInBounds<B> for H {
    const PROVEN: () = {
        let ops = H::COALESCED.as_slice();
        let mut i = 0;
        while i < ops.len() {
            assert!((ops[i].offset as usize) + 4 <= B::SIZE, "...");
            i += 1;
        }
    };
}
```

`apply()` references `<H as ProvenInBounds<B>>::PROVEN` once. That reference
is what forces the check — and forcing a `const` item's evaluation is a
compiler obligation, not a runtime event. We confirmed this the direct way:
deliberately set a test pin's route to an out-of-range offset and rebuilt.
The result was `error[E0080]: evaluation panicked: ... erroneous constant
encountered` — a build failure, not a test-harness panic — exactly the
difference the design is supposed to produce. With this in place, `apply()`
itself carries no `unsafe` keyword at all: there's nothing left for a caller
to promise, because the type system already forecloses an out-of-bounds
offset ever reaching it.

## Memory regions: the same idea, one layer down

GPIO (and other) register blocks live behind an MPU grant — a process must
hold a `Region<T>` (`../../../util/region/`) naming the mapping it was given
in `system.json5` before it can construct a `GpioBlock` at all. `Region<T>`
is move-only for the same reason pin tokens are, and minted the same way: a
`new()` that requires proof via `AuthorizesRegion<T>`, generated per-process
by `../../../util/region/regions.rs.jinja`, mirroring `pin_claims.rs.jinja`
exactly.

Binding a pin to a `GpioBlock` additionally checks, at compile time, that the
granted region actually *covers* that pin's register group
(`AssertGpioGrant::COVERS` in `../../../hal/src/gpio.rs`, over `map_extent`,
which computes the real highest offset any of that pin's register group can
name). A process holding the wrong region for a pin it's trying to drive is
a compile error, not a wrong MMIO read on real hardware.

---

## Security analysis

The rest of this document is written the way we'd want a skeptical reviewer
to read the first half: for each guarantee claimed above, what's actually
been verified, and exactly where it stops holding.

### "A process can only mint tokens for pins it's granted"

**Mechanism:** `AuthorizesPin<P>` / `AuthorizesRegion<T>`, generated
per-process from `system.json5`, as described above.

**What this actually proves:** no *normal* application code path can produce
a token for a pin or region outside that process's own manifest entry. The
manifest and the driving code cannot silently drift, because the only
`AuthorizesPin` impls that exist are the ones codegen emitted from the
manifest itself.

**Where it stops:** we tested, rather than assumed, whether this is a hard
type-system boundary. It isn't, fully. Rust's orphan rule permits
implementing a *foreign* trait for a *locally-defined* type, regardless of
the trait's other generic parameters. This compiles cleanly, with zero
resistance from the compiler, in any crate:

```rust
struct ForgedAuthority;
impl openprot_hal::resource::AuthorizesPin<ast10x0_peripherals::scu::pins::scu410_2> for ForgedAuthority {}
fn forged_take() -> ast10x0_peripherals::scu::pins::scu410_2 {
    unsafe { ast10x0_peripherals::scu::pins::scu410_2::take(&ForgedAuthority) }
}
```

We considered sealing `AuthorizesPin`/`AuthorizesRegion` the way
`MuxRoutes` is restricted below, and rejected it: sealing only works when
every legitimate implementor lives in the one crate doing the sealing. Here,
the legitimate implementor (`Authority`) is deliberately generated into a
*separate crate per process* — there is no single crate boundary that
includes every legitimate `Authority` while excluding a hand-written one,
because from the compiler's point of view a generated crate and an
attacker's hand-written crate are equally "external." This is a real,
currently-unclosed gap, not a solved problem dressed up as one.

**Why it's bounded in practice:** this stops accidental drift (a typo, a
copy-paste reaching for the wrong pin) completely. It does not stop someone
willing to hand-write a fake authority directly in source — but that person
already has write access to the exact same `system.json5` the real
`Authority` is generated from, and editing it legitimately is strictly
easier than hand-rolling a fake impl. This is a source-review-time concern,
not a runtime one; it does not help a compromised, already-compiled,
already-running process.

### "A register write can't land outside the block it targets"

**Mechanism:** `RegBatch<B>` / `ProvenInBounds<B>`, described above.

**Verified:** by deliberately breaking it and reading the actual compiler
output, not by inspection alone — see the `ProvenInBounds` section.

**Where it stops:** this proof constrains what *this program's own source
code* can express. It says nothing about a CPU whose instruction pointer has
been redirected by an external attack — fault injection, voltage/clock
glitching — directly to the one raw `write_reg` call that must exist
somewhere underneath every safe wrapper. No software technique, in this
codebase or any other, closes that; it requires hardware countermeasures
(redundant or lockstep execution, glitch detection) entirely outside what a
type system can express. We call this out explicitly rather than implying
the compile-time proof is a complete answer to a hardware attacker.

### "The MPU keeps a compromised process from touching another process's hardware"

**Mechanism:** the `Region<T>` grant checked by `AssertGpioGrant::COVERS`,
backed by the real ARMv7-M MPU.

**Verified, with real numbers, not assumed:** `gpio_smoke_process`'s actual
`system.json5` grants `gpio_regs` as `start_address: 0x7e780000,
size_bytes: 0x1000` — the entire 4KB GPIO controller — even though this
process's `pin_claims` name only 4 bits in one register group (`ABCD`).

**Where it stops, and why, concretely:**

- This core has exactly 8 MPU regions total (confirmed from a real boot log:
  `[INF] MPU regions: 8`), already spent on flash, RAM, stack, and this one
  device mapping. There's no budget to carve one region per claimed pin.
- ARMv7-M MPU regions must be power-of-two sized and self-aligned. Pigweed's
  own MPU driver (`pw_kernel/arch/arm_cortex_m/protection/protection_v7.rs`)
  does support **sub-region disable (SRD)** — splitting one region into 8
  equal chunks, independently enabled or disabled, computed automatically
  from whatever `[start, len)` a manifest requests. This is real,
  automatic, already-implemented hardware enforcement; narrowing a
  manifest's requested mapping size is the lever, not new kernel code.
- But we traced the actual register layout (`../peripherals/gpio/map.rs`)
  and found the three GPIO groups' "out_read" registers sit at `0x0c0`,
  `0x0c4`, and `0x0c8` — 8 bytes apart. The MPU's minimum sub-region
  granularity here is 32 bytes. Even a maximally narrowed grant for group
  `ABCD` would, unavoidably, also enable read/write access to `EFGH`'s and
  `IJKL`'s `out_read` registers, because all three fall inside the same
  32-byte chunk. Pigweed's own driver documents this class of trade-off by
  name ("sub-region over-provisioning").

The type system is doing register-level separation that this specific piece
of hardware cannot express at that granularity, full stop. Narrowing the
manifest's grant from whole-controller to per-group is real, bounded
follow-up work (tracked below); it was not done in this PR.

### Why we didn't add a runtime permission check

A natural-sounding idea: before every register write, check the offset
against an allow-list built from the same compile-time-proven data, and
reject anything not on it. We considered this and rejected it, for a
specific reason, not out of laziness:

A software check lives in the same trust domain as the write it's guarding.
An attacker capable of redirecting execution straight to the raw write
instruction (the fault-injection scenario above) can, by the same mechanism,
redirect straight past any software check placed in front of it — jumping
mid-function is exactly as available against a checked wrapper as against an
unchecked one. The check would only help against a *weaker* attacker who can
corrupt a value in memory but not control the instruction pointer — and
that threat is already closed, independent of any runtime check: the data
driving these decisions (`GpioMap`, `RegOp`, `COALESCED`) is `const`/`'static`
data, and `pw_kernel`'s system generator marks every process's entire flash
segment read-only for *every* privilege level, confirmed by reading
`pw_kernel::tooling::system_generator::lib.rs`'s `populate_memory_mappings`.
A runtime allow-list check would be real code, with a real maintenance cost,
protecting against a threat that's already closed by hardware. We left it
out deliberately, and are recording the reasoning here so it reads as a
decision, not an omission.

## What changed in this PR

- The SCU is relocked (`lock_scu_writes`) immediately after the boot-time
  mux is applied — previously the unlock window never closed.
- `AuthorizesPin<P>`: pins can only be minted by generated code that
  reflects a process's own `pin_claims_<process>` manifest entry.
- `pin_claims_<process>` entries carry a `pin:capability` tag
  (e.g. `scu410_0:gpio`), validated against a known-tag list.
- `Reg<const W1C: bool>` became `Reg<Kind>` with named `W1C`/`NoW1C` marker
  types.
- `RegBatch<B>` is block-typed; `ProvenInBounds<B>` proves every coalesced
  offset fits `B::SIZE` at compile time, with no runtime-reachable call
  path. `field_mux::apply()` carries no `unsafe` keyword as a result.
  `RegBlock`, a pure forwarding-wrapper trait, was removed.
- `GpioPin::apply()` reads its register offset straight from the pin's own
  static `GpioMap` instead of through a transient local copy.
- `MuxRoutes` construction is restricted to this crate (`coalesce_routes`
  and `RegBatch::new` are `pub(crate)`) rather than via a sealed marker
  trait — the same security property with no new public API surface.
- `AuthorizesRegion<T>` mirrors `AuthorizesPin<P>` for memory-mapped
  regions (`util_region::Region::new`).
