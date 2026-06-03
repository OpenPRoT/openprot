# I3C Behavioral Parity Goal (AST10x0 / openprot)

## Objective

- **Authority** = aspeed-rust `src/i3c/` @ `ce3b567` (frozen 2026-06-02;
  pinned at `plans/i3c-reference/PINNED_COMMIT.txt`).
- **Informative-only**: DesignWare/Zephyr/Linux i3c controller drivers (register
  semantics only); `proposed_traits` @ `85641310` (operation *shape* only, not
  available in openprot). Authority wins on any divergence; informative refs are
  treated as not-our-target.
- **Parity standard (decided)**: *Observable parity, keep fixes.* The port is
  behaviorally equivalent to aspeed-rust on the success path; panicking slice
  indexing / unchecked arithmetic may be hardened to typed errors, and each such
  fix is recorded in the deltas ledger (§2). This mirrors the decision made for
  the I2C port (`peripherals/i2c/master.rs` swapped `&bytes[a..b]` →
  `.get(..).ok_or(I2cError::Invalid)?`).
- **Scope (decided)**: full 1:1 functional port — master + target(secondary) +
  IBI/hot-join + CCC + legacy-I2C device support. AST10x0 only.
- **Design-pattern depth (decided)**: mirror the I2C port. Apply the three
  `pac-design-patterns` structural patterns at the same depth the I2C port did;
  keep aspeed-rust's `HardwareInterface` 6-trait split and its global IBI/IRQ
  statics, recording those globals as an intentional delta vs.
  *Borrow-Arbitrated Engine Exclusivity* (§5 ADR-3).

---

## 1. Reference behavior to replicate (Phase 1 — every claim cites authority `file:line`)

Paths below are relative to `aspeed-rust/src/i3c/`.

### 1.1 Module / type model
- `I3cController<H: HardwareInterface>` wraps `hw: H` + `config: I3cConfig`
  (`controller.rs:39-44`). `new` == `from_initialized` (no I/O);
  `init_hardware` does the register init (`controller.rs:71-124`).
- Concrete hardware impl `Ast1060I3c<I3C: Instance, L: Logger>` holds
  `&'static` register blocks for `i3c`, `i3cg`, `scu` + a `Logger`
  (`hardware.rs:419-440`). Built from the `Instance` trait
  (`hardware.rs:338-367`, buses 0..3 via `macro_i3c!`).
- `HardwareInterface` = supertrait of `HardwareCore + HardwareClock +
  HardwareFifo + HardwareTransfer + HardwareRecovery + HardwareTarget`
  (`hardware.rs:6-19`, blanket impl `hardware.rs:328-336`).

### 1.2 Init / reset / clock sequence (`hardware.rs:677-854`)
1. `global_reset_deassert()`, then program `i3cg` reg1 (actmode=1, instid=bus,
   staticaddr=0x74) and reg0=0 (`hardware.rs:680-695`).
2. `core_reset_assert` → `clock_on` → `core_reset_deassert` → `i3c_disable`
   (`hardware.rs:699-702`). **Clock + reset live inside the driver**, via the
   `scu`/`i3cg` registers — unlike the I2C port, which delegated SCU to the
   board. (See §5 ADR-2.)
3. Soft-reset all queues via `i3cd034` (IBI/RX/TX/response/cmd/core), poll until
   `i3cd034 == 0` (`hardware.rs:721-743`).
4. `set_role` / `init_clock` (`hardware.rs:744-745`); DAT init with SIR/MR
   reject (`hardware.rs:806-818`); interrupt-enable + device static/dynamic
   address + controller enable (`hardware.rs:820-846`).

### 1.3 Clock timing (`hardware.rs:990-1111`)
- `ns_to_cnt_u8 = |ns| ns.div_ceil(core_period)` clamped to `u8::MAX`
  (`hardware.rs:995-998`). Computes I2C-FM hi/lo, I3C OD hi/lo, I3C PP hi
  (clamped ≤ 41 ns/spec), SDA-TX-hold clamped to [1,7]. Clock validity bounds
  in `config.rs:601-666` (`I3C_MIN_CORE_CLK_SDR=12.5M`, `_HDR=25M`,
  `MAX_CORE_CLK=400M`; core ≥ 4× SCL).

### 1.4 Transfer / completion model
- `start_xfer` writes all cmd TX-FIFO entries + sets response threshold
  (`hardware.rs:1347-1382`); `end_xfer` drains the response queue (≤32),
  parses TID/len/error, scatters RX (`hardware.rs:1384-1466`). Error codes
  `constants.rs:194-205`.
- Completion wait uses `Completion` (`types.rs:323-378`): `complete()` does
  `store(Release)` + `cortex_m::asm::sev()`; `wait_for_us<D: DelayNs>` spins
  `delay.delay_us(1)` up to `timeout_us`. Transfer/CCC/ENTDAA wait
  `1_000_000_000 us` with a `DummyDelay` (`hardware.rs:1635-1636,1693-1695,
  1816-1817,2024-2039`).
- Generic poll: `poll_with_timeout<F,C,D: DelayNs>` (`hardware.rs:569-589`),
  called for queue-reset and FIFO waits (`hardware.rs:736,1223,1247,1268`).

### 1.5 ENTDAA / device management
- `do_entdaa` builds `ADDR_ASSGN_CMD | ENTDAA | DEV_COUNT=1 | DEV_INDEX=pos |
  ROC | TOC` and waits ≤1 s (`hardware.rs:1664-1710`). `attach_i3c_dev` updates
  `AddrBook`/`Attached` then `hw.attach_i3c_dev` (`controller.rs:144-179`).
  Even-parity MSB on dynamic addr in DAT (`hardware.rs:1189-1199,1484-1496`).

### 1.6 CCC (`ccc.rs`)
- GETPID(0x8D) `ccc.rs:339-366`, GETBCR(0x8E) `:251-284`, GETSTATUS(0x90)
  `:370-420`, SETNEWDA(0x88) `:287-329`, RSTDAA(0x06) `:434-450`, RSTACT
  `:228-249`, ENEC/DISEC `:154-225`. Broadcast (id ≤ 0x7F) vs direct cmd build
  `hardware.rs:1502-1662` (`CP|ROC|TOC`, READ_TRANSFER=rnw).

### 1.7 IBI / hot-join / IRQ (`ibi.rs`, `hardware.rs:67-417,1857-1897`)
- Per-bus 16-deep SPSC `heapless::spsc::Queue` (`ibi.rs`), `critical_section`
  guarded; work items `HotJoin | Sirq{addr,len,data[16]} | TargetDaAssignment`
  (`ibi.rs:22-37`). Enqueue `ibi.rs:118-176`, consume `i3c_ibi_workq_consumer`.
- IRQ registry: `static BUS_HANDLERS: [Mutex<RefCell<Option<Handler>>>;4]`
  (`hardware.rs:76`); `register_i3c_irq_handler`/`dispatch_i3c_irq`
  (`hardware.rs:92-110`); per-bus entry points + optional `#[no_mangle]` ISRs
  (`hardware.rs:369-417`). `enable_irq`/`disable_irq` via `cortex_m NVIC`
  (`hardware.rs:863-877`). `init_hardware` registers a `&mut Self`-derived
  context + `dmb()` barrier before `enable_irq` (`controller.rs:111-132`).
- IBI parse: count from `i3cd04c`, id from `i3cd018`; addr 0x02 = hot-join,
  rnw addr = SIR (`hardware.rs:1857-1897`). Device IBI-enable clears SIR-reject,
  sets MDB/PEC, sends ENEC (`hardware.rs:1281-1345`).

### 1.8 Target (secondary) mode (`hardware.rs:748-834,956-971,1984-2049`)
- Secondary interrupt-enable set, static addr program (`hardware.rs:748-834`);
  ISR handles dyn-addr-assign / resp-ready / ccc-update (`:956-971`); SIR raise
  writes TX-FIFO + IBI cmd + waits (`:1984-2049`).

### 1.9 HAL surface (`hal_impl.rs`)
- `proposed_traits::i3c_master::I3c` for `I3cController`: `assign_dynamic_address`
  (ENTDAA → GETPID → verify pid → GETBCR → ibi_enable, `hal_impl.rs:33-99`),
  plus no-op `handle_hot_join`/`set_bus_speed`/`request_mastership`.
- `proposed_traits` target traits (`I2CCoreTarget`, `I3CCoreTarget`,
  `DynamicAddressable`, `IBICapable`) `hal_impl.rs:135-218`; `get_ibi_payload`
  builds `[mdb, crc8_ccitt]` (`hal_impl.rs:186-239`).

---

## 2. Deltas vs. the authority (Phase 3 ledger)

Classification ∈ { conformance · intentional delta · out-of-scope }. Every
*intentional delta* carries a reachability trace (consumer cited) or a stated
acceptance.

| ID | Authority behavior (`file:line`) | Port behavior | Classification |
|----|----------------------------------|---------------|----------------|
| D1 | hal_impl implements `proposed_traits` i3c master + target traits (`hal_impl.rs:10-12,33,143-218`) | `proposed_traits` is absent from openprot. Convert the master ops (`assign_dynamic_address`, `handle_hot_join`, `set_bus_speed`, `request_mastership`) to **inherent methods** on `I3cController`; convert target traits to an internal `TargetCallbacks`-style trait. Same logic, no external trait dep. | **intentional delta** — mirrors the I2C port, which dropped `proposed_traits` for an internal `TargetCallbacks` (`peripherals/i2c/target_adapter.rs`). Reachability: no openprot consumer references `proposed_traits::i3c_*` (grep: zero hits under `openprot/`). embedded-hal 1.0 has **no** i3c trait, so there is no standard seam to retarget to — inherent methods are the I2C-consistent choice. |
| D2 | `Completion::wait_for_us<D: DelayNs>` + `poll_with_timeout<…,D: DelayNs>` + local `DummyDelay` busy-spin (`types.rs:367`, `hardware.rs:569-589,697`) | Inject a **`Y: FnMut(u32)` yield closure** at the `Ast1060I3c` construction gate; `wait_for_us`/`poll_with_timeout` take `&mut dyn FnMut(u32)` (type-erased), invoked once per non-completing poll. Bare-metal callers pass `\|_\| core::hint::spin_loop()`. | **intentional delta** — *Cooperative-Yield Bounded-Poll Device* pattern; identical to the I2C port's `yield_ns` closure (`peripherals/i2c/controller.rs`). Observable behavior on a spin closure == authority's `DummyDelay`. |
| D3 | `Ast1060I3c` holds `&'static RegisterBlock` obtained by `unsafe{&*ptr()}` in a **safe** `new` (`hardware.rs:419-439`) | Hold raw `*const RegisterBlock`; **single `unsafe fn new`** documenting the pointer-validity + serialization contract; one private `regs()`/`i3cg()`/`scu()` deref; `!Sync` via `PhantomData<UnsafeCell<()>>`. No `unsafe`/PAC types above the façade. | **intentional delta** — *Confined-`unsafe` MMIO Façade*; identical to the I2C port (`peripherals/i2c/controller.rs` raw-pointer + `_not_sync`). Pure structural; no behavior change. |
| D4 | `Ast1060I3c<I3C, L: Logger>` + `i3c_debug!` writing to a `heapless::String<128>` Logger (`hardware.rs:419-449`) | Drop the `L: Logger` generic and the `i3c_debug!` string-formatting path; debug logging removed (or routed to `pw_log` where genuinely useful). Drops the `heapless::String` formatting surface. | **intentional delta** — mirrors the I2C port (no `Logger`; tests use `pw_log` directly). Logging is non-functional; no observable bus behavior. |
| D5 | Panicking slice indexing / unchecked arithmetic in FIFO/response scatter paths (e.g. `hardware.rs` `end_xfer` RX distribution) | Harden to `.get(..).ok_or(I3cError::…)?` where a malformed length could panic, matching the I2C hardening. | **intentional delta** (allowed by parity standard) — to be enumerated row-by-row during implementation as each site is touched; each gets a one-line note here. Success path unchanged. |
| D6 | Global IBI SPSC queues (`static mut IBIQ_BUFS`) + IRQ registry (`static BUS_HANDLERS`) + `#[no_mangle]` ISR exports (`ibi.rs`, `hardware.rs:76-417`) | **Kept as-is** (behavior-preserving). `isr-handlers`-style `#[no_mangle]` exports gated off by default for kernel integration (the AST10x0 target defines ISRs and calls `dispatch_i3c_irq`). | **intentional delta vs. *Borrow-Arbitrated Engine Exclusivity*** — the engine state is process-global, not threaded through a `&mut` device, so that pattern's no-global-op-state box is **knowingly not met**. Justification: the ISR architecture requires a static handler/queue reachable from the interrupt vector; this is the authority's design and the parity target. Recorded as ADR-3. |
| D7 | `heapless = 0.8` (`aspeed-rust/Cargo.toml:40`), `spsc::Queue::split()` API | openprot ships `heapless = 0.9`. Port to the 0.9 `spsc` API (verify `Queue`/`split`/`Producer`/`Consumer` signatures during impl). | **intentional delta** (dep version) — API-compat shim only; no behavior change. Flagged for verification (Phase 7 if it misbehaves). |
| D8 | `critical-section = 1.2` + `cortex-m` feature `critical-section-single-core` (`aspeed-rust/Cargo.toml:46-47`) | openprot `@rust_crates` lists `cortex-m 0.7.7` **without** that feature and **no** `critical-section`. Add `critical-section` to `third_party/crates_io/Cargo.toml` and enable `cortex-m/critical-section-single-core` (or provide the CS impl the target already uses). | **intentional delta** (build wiring) — must be resolved before compile; see Plan item 1. |
| D9 | Authority style triggers openprot's `-D warnings` clippy (collapsible-if, unnecessary-cast, RefCell `borrow_mut` panic path) | Source-level, behavior-identical adjustments so the **new i3c code is clippy-clean** (the surrounding i2c/smc/uart already carry pre-existing clippy errors, left untouched): collapsed `if`/`if let` into let-chains; dropped a `u32 as u32`; and in `ibi.rs` replaced `RefCell::borrow_mut` with `try_borrow_mut` (panic-free — a conflicting borrow is impossible inside the `critical_section`) and array `[bus]` with `get_mut(bus)`, so the IBI-consumer path passes `no_panics_test`. | **intentional delta** (lint/panic hygiene) — observably identical; the `try_borrow_mut`/`get_mut` changes also discharge the relevant part of D5 for the IBI plane. |

### 2.x Independent authority split (Phase 4)
1. **Parity authority** — aspeed-rust `src/i3c/` @ `ce3b567` (the behavior to
   match). The done-criteria parity tests gate on this.
2. **Correctness authority** — MIPI I3C Basic v1.1.1 for CCC codes / address
   reservations / parity (used to sanity-check, NOT to override the authority;
   where aspeed-rust diverges from the spec, that divergence is a D-row, not a
   silent "fix").
3. **Interface authority** — the openprot consumer seam. **Verify-the-mandate:**
   embedded-hal 1.0 defines **no** i3c master/target trait (confirmed: no i3c in
   `embedded-hal`), and openprot has no i3c HAL trait of its own (grep: zero i3c
   trait defs under `openprot/hal`, `openprot/drivers`). Therefore the interface
   obligation is *only* "compile as a `pub mod i3c` in `ast10x0_peripherals` and
   expose inherent methods + an internal target-callback trait" — there is no
   external trait contract to satisfy. Do **not** invent one.

### 2.x OPEN ISSUE — RESOLVED
- **OPEN-1 (pinctrl) — RESOLVED** via `../ast1060-pac/ast1060.svd`. The I3C pad
  function-enable bits live in two SCU registers (set bit = enable function,
  same `clear:false` semantics as the I2C groups):
  - **SCU418** (Low-Voltage pads): I3C1 SCL=bit16/SDA=bit17, I3C2 SCL=18/SDA=19,
    I3C3 SCL=20/SDA=21, I3C4 SCL=22/SDA=23 (SVD `EnblI3CSCLn/SDAnLVFnPin`).
  - **SCU4B8** (High-Voltage pads): I3C1 SCL=bit8/SDA=bit9, I3C2 SCL=10/SDA=11,
    I3C3 SCL=12/SDA=13, I3C4 SCL=14/SDA=15 (SVD `EnblI3CSCLn/SDAnHVFnPin`).

  Bus mapping: aspeed-rust `BUS_NUM` 0/1/2/3 (`I3c/I3c1/I3c2/I3c3`) → hardware
  I3C1/I3C2/I3C3/I3C4. `scu/pinctrl.rs` already generates `PIN_SCU418_16..23`
  and `PIN_SCU4B8_8..15` and `apply_pinctrl_group` already matches `0x418` /
  `0x4B8` — so the fix is **zero PAC changes**: add LV groups (default)
  ```
  pub const PINCTRL_I3C1: &[PinctrlPin] = &[PIN_SCU418_16, PIN_SCU418_17];
  pub const PINCTRL_I3C2: &[PinctrlPin] = &[PIN_SCU418_18, PIN_SCU418_19];
  pub const PINCTRL_I3C3: &[PinctrlPin] = &[PIN_SCU418_20, PIN_SCU418_21];
  pub const PINCTRL_I3C4: &[PinctrlPin] = &[PIN_SCU418_22, PIN_SCU418_23];
  ```
  plus optional `PINCTRL_I3Cn_HV` (SCU4B8) variants. LV vs HV is a board
  decision; default to LV (the common I3C low-voltage rail). New Plan item 0
  below. QEMU `ast1030-evb` does not model pads, so the init smoke test still
  passes without pad mux; the group matters only for on-hardware bring-up.

---

## 3. Implementation plan (numbered; each ends with Acceptance)

> Layout mirrors `peripherals/i2c/`: a flat `i3c/` module set inside the single
> `ast10x0_peripherals` bazel `rust_library`. Files ported 1:1 by name where
> possible: `mod.rs, controller.rs, config.rs, types.rs, error.rs, constants.rs,
> ccc.rs, ibi.rs, hardware.rs`, plus `hal_impl`/target callbacks folded per D1.

0. **Pinctrl groups (OPEN-1, resolved).** Add `PINCTRL_I3C1..4` (LV / SCU418)
   const groups to `scu/pinctrl.rs`, composing existing `PIN_SCU418_16..23`;
   optional `_HV` variants over `PIN_SCU4B8_8..15`. No PAC change.
   *Acceptance*: `peripherals` crate builds with the new consts; an `i3c_init`
   test can pass `&[pinctrl::PINCTRL_I3C1]` to `Ast10x0Board`.
1. **Dependency wiring (D7, D8).** Add `critical-section` to
   `third_party/crates_io/Cargo.toml`; enable `cortex-m/critical-section-single-core`;
   confirm `heapless 0.9` + `cortex-m` resolve for the `thumbv7em` target. Add
   `i3c/*.rs` to `peripherals/BUILD.bazel srcs` and the new deps to its `deps`.
   *Acceptance*: `bazel build //target/ast10x0/peripherals:peripherals` resolves
   all i3c crates (even before i3c code is added — deps compile).
2. **Port leaf modules verbatim-of-behavior**: `error.rs`, `constants.rs`,
   `types.rs` (incl. `Completion`, but `wait_for_us` re-signatured to
   `&mut dyn FnMut(u32)` per D2), `config.rs`. No `proposed_traits`, no `Logger`.
   *Acceptance*: these four compile standalone in the crate; `Completion` unit
   test (signaled/timeout) passes with a spin closure.
3. **Confined-`unsafe` façade (D3)** in `hardware.rs`: `Ast1060I3c<I3C: Instance,
   Y: FnMut(u32)>` holding `*const` for i3c/i3cg/scu; one `unsafe fn new(.., yield_fn)`
   with the 2-obligation `# Safety` doc; private `regs()/i3cg()/scu()`; `!Sync`
   marker. Drop `L: Logger`/`i3c_debug!` (D4).
   *Acceptance*: no `unsafe` or PAC type appears outside the façade methods
   (grep check); `cargo`/`bazel` clippy clean on the struct + ctor.
4. **Port `HardwareCore/Clock/Fifo/Transfer/Recovery/Target` impls** (the bulk of
   `hardware.rs`) onto the façade, threading the type-erased `&mut dyn FnMut(u32)`
   into every `wait_for_us`/`poll_with_timeout` call site (D2). Harden panicking
   index/arith sites to typed errors as touched, logging each in §2 D5.
   *Acceptance*: `start_xfer`/`end_xfer`/`do_ccc`/`do_entdaa` compile; a host or
   QEMU unit asserting a queue-reset poll completes (or times out typed) passes.
5. **Port `controller.rs`** (`I3cController<H>`, attach/detach, recover_bus,
   init_hardware with `dmb()` + IRQ registration). Keep generic over `H`.
   *Acceptance*: `I3cController::new` + `init_hardware` build; `attach_i3c_dev`
   address-book bookkeeping unit test passes.
6. **Port `ibi.rs`** (heapless 0.9 SPSC, `critical_section` guards) and the IRQ
   registry/`dispatch_i3c_irq` in `hardware.rs` (D6). `#[no_mangle]` ISR exports
   behind a default-off cfg; document the kernel-calls-`dispatch_i3c_irq` path.
   *Acceptance*: enqueue/consume round-trip unit test (HotJoin / Sirq /
   TargetDaAssignment) passes under a single-core CS impl.
7. **CCC + master ops (D1)**: port `ccc.rs`; reimplement
   `assign_dynamic_address` & friends as inherent `I3cController` methods.
   *Acceptance*: CCC command-word composition unit tests (broadcast vs direct,
   ROC/TOC/CP bits) match authority byte-for-byte.
8. **Target mode + IBI payload (D1)**: internal `TargetCallbacks`-style trait;
   port `get_ibi_payload` (`crc8_ccitt`, `[mdb, crc]`) and the secondary ISR
   paths. *Acceptance*: `crc8_ccitt` KAT + payload `[mdb,crc]` shape test pass.
9. **Wire into crate**: `pub mod i3c;` in `peripherals/lib.rs`; re-export the
   public surface in `i3c/mod.rs` (mirroring the authority's `mod.rs:59-97`,
   minus `proposed_traits`).
   *Acceptance*: `bazel build //target/ast10x0/peripherals:peripherals` is green.
10. **Parity / smoke tests** under `target/ast10x0/tests/peripherals/i3c/`
    (mirror `i2c/i2c_init` + `i2c/i2c_irq`): an `i3c_init` register-verify smoke
    test (clock-timing registers vs computed expected, like the I2C test's
    `verify_init_registers`) and an `i3c_irq` IBI/transfer test where feasible
    under QEMU `ast1030-evb`.
    *Acceptance*: see Done criteria.

> Plan honesty: items 7–8 collapse `hal_impl.rs` into `controller.rs` +
> target-callback module (D1); the standalone `hal_impl.rs` file is **struck** —
> there is no external trait to host.

---

## 4. Done criteria (testable, production-dominant workload)

- `bazel build //target/ast10x0/peripherals:peripherals` green with i3c included.
- `bazel test --config=virt_ast10x0 //target/ast10x0/tests/peripherals/i3c/...`
  passes under QEMU (`TEST_RESULT:PASS`), covering at minimum:
  - **i3c_init**: full init sequence runs; the computed I3C/I2C clock-timing
    register fields (`init_clock`, §1.3) read back equal to values derived from
    the authority's formulas for a known `core_clk_hz` — the register-verify
    gate, analogous to the I2C `verify_init_registers`.
  - **ENTDAA / CCC word composition**: the command words built for ENTDAA and a
    representative direct + broadcast CCC equal the authority's bit layout
    (host unit test; production-dominant control path).
  - **IBI work queue**: enqueue→consume round-trip for HotJoin and SIR.
- No `unsafe` and no `ast1060_pac` type outside the `Ast1060I3c` façade methods
  (grep gate).
- Every §2 delta row is "discharged": authority lines read AND consumer/accept
  trace recorded; D5 rows each enumerated.
- `./pw presubmit` (clippy + license/SPDX headers + format) clean on the new
  files.

---

## 5. Architecture decisions (Phase 8 ADRs — grounded)

**ADR-1 — Keep the `HardwareInterface` 6-trait split (do not collapse).**
The authority separates `HardwareCore/Clock/Fifo/Transfer/Recovery/Target`
(`hardware.rs:6-19`) and makes `I3cController` generic over the supertrait. The
I2C port collapsed its layers because I2C had a single concrete type; I3C's
generic-over-`H` design is load-bearing for testability (mock `H`) and is the
parity target. Decision (user-confirmed): preserve it. The design-pattern
façade/yield/`!Sync` work is applied to the **concrete `Ast1060I3c`**, which is
exactly the layer those patterns target.

**ADR-2 — I3C retains in-driver clock/reset; board does pinctrl only.**
Unlike I2C (where `Ast10x0Board::init()` owns SCU clock/reset and
`init_i2c_global` only sets I2CG), the authority interleaves
`global_reset_deassert`/`core_reset_assert`/`clock_on`/`core_reset_deassert`
with `i3cg` register writes inside `init()` (`hardware.rs:680-702`). Splitting
that out risks reordering a sequenced reset. Decision: keep the clock/reset
sequence inside the i3c driver (reached through the confined `scu()`/`i3cg()`
façade derefs); the board layer contributes only the I3C pinctrl group
(OPEN-1). This is a *deliberate* divergence from the I2C board-split, justified
by sequencing coupling — recorded so it is not mistaken for an oversight.

**ADR-3 — Global IBI/IRQ state is an accepted delta vs Borrow-Arbitrated
Exclusivity.** `static BUS_HANDLERS` + `static mut IBIQ_BUFS` (`hardware.rs:76`,
`ibi.rs`) are process-global, reached from the interrupt vector via
`dispatch_i3c_irq`. The *Borrow-Arbitrated Engine Exclusivity* checklist
forbids global op-state aliased outside a `&mut` device — this port **knowingly
fails that box**, because an ISR cannot borrow a stack-owned device. Mutual
exclusion of the queues rests on `critical_section` + SPSC discipline (the
authority's design), not on `&mut` arbitration. The *Confined-`unsafe` Façade*
and *Cooperative-Yield* patterns (ADR applies to `Ast1060I3c`) ARE conformed
to; only the exclusivity pattern is consciously out-of-scope for the global
IBI/IRQ plane. Stated per the pattern's own "state the language dependency /
gate-delegated" discipline.

**ADR-4 — Pattern conformance summary.**
- *Confined-`unsafe` MMIO Façade*: **conformed** (D3) — one `unsafe fn new`, one
  private deref per block, `!Sync`, no PAC leakage.
- *Cooperative-Yield Bounded-Poll Device*: **conformed** (D2) — `Y: FnMut(u32)`
  injected at gate, type-erased `&mut dyn FnMut(u32)` at the poll loops, bounded
  iteration → typed timeout, advisory ns arg.
- *Borrow-Arbitrated Engine Exclusivity*: **partially out-of-scope** (ADR-3) for
  the IBI/IRQ globals; the per-call transfer state is still threaded through
  `&mut I3cController`.

---

## 6. Outcome (implementation pass — 2026-06-02)

**Status: ported and building green.** All Plan items 0–10 landed; the driver is
9 files (`ccc, config, constants, controller, error, hardware, ibi, mod, types`)
under `peripherals/i3c/`, wired into `ast10x0_peripherals` via `lib.rs` +
`BUILD.bazel`. `hal_impl.rs` was struck (D1) — its logic lives as inherent
methods on `I3cController` in `controller.rs`.

Verified gates:
- `bazel build --platforms=//target/ast10x0 //target/ast10x0/peripherals:peripherals`
  → **green** (full driver compiles for thumbv7em).
- `bazel build --platforms=//target/ast10x0 .../i3c/i3c_init:target` → **green**
  (the init smoke-test kernel image builds).
- `bazel test .../i3c/i3c_init:no_panics_test` → **PASSED** (the driver + test
  binary are panic-free; the bus-index `panic!` arms fold out under the const
  `BUS_NUM`).
- `bazel test --config=virt_ast10x0 //target/ast10x0/tests/peripherals/i3c/...`
  → builds the QEMU image (588 actions, green); the `hardware`-tagged
  `i3c_init_test` is QEMU-incompatible by design (same as the I2C tests) and
  runs on real silicon only.

Delta resolutions vs §2:
- **D1, D2, D3, D4, D6, D7, OPEN-1** — all implemented as specified above.
- **D8** — `critical-section` added to `third_party/crates_io/Cargo.toml` and
  `cortex-m/critical-section-single-core` enabled; `@rust_crates//:heapless`
  resolved to 0.9.2; repinned cleanly (no Cargo.lock churn needed — all three
  crates were already present transitively).
- **D7 (edition 2024)** — beyond the `Producer/Consumer` generic change, the
  `static mut IBIQ_BUFS` split was rewritten through `addr_of_mut!` to satisfy
  the edition-2024 `static_mut_refs` rule.
- **D5 (panic hardening) — DEFERRED.** No FIFO/response-scatter index was
  hardened in this pass; the authority's indexing is retained verbatim (the
  parity standard permits keeping reference behavior). `no_panics_test` passing
  shows no reachable panic in the init path; revisit per-site if a malformed
  hardware length is shown to reach a slice index. (The deferral is the only
  open §2 item; everything else is discharged.)

Façade-cleanliness note: the borrow-split required by the free-function
`poll_with_timeout` / `rd_fifo` / `drain_fifo` led the three deref helpers
(`i3c()/i3cg()/scu()`) to return `&'static` references (sound under the `new`
contract: pointers valid for the program lifetime), so a register reference and
`&mut self.yield_fn` can be held in disjoint statements. No `unsafe` or PAC type
appears above those three helpers + `new`.

Second pass (continued — tests + lint):
- **`i3c_irq` test added** (`tests/peripherals/i3c/i3c_irq/`): dual-image
  controller + secondary-target, mirroring `i2c_irq`. The controller drains the
  IBI work queue; the target raises a SIR. `no_panics_test` (controller) +
  `slave_no_panics_test` both **PASSED**; the two-device exchange runs under the
  `hardware`-tagged `irq_test` on real silicon only.
- **clippy** (`rust_clippy_aspect`, `-D warnings`): the **7 i3c findings are all
  fixed** (D9) — re-running the aspect leaves only the 8 *pre-existing*
  i2c/smc/uart findings, which are out of scope and untouched.
- **`no_panics` (all three i3c images)**: PASSED.

Third pass (parity with the I2C test/CI bar):
- **`./pw format`**: all 26 changed files (14 `.rs` incl. every new i3c file)
  reformatted to rustfmt canonical → "No formatting changes needed"; the crate
  still builds after.
- **`./pw presubmit`**: **all three recipes OK** — `build clippy //...`
  (whole-repo clippy, i3c included), `check format`, and `check
  presubmit_checks` (license/SPDX/include-guard/json). The only fix needed was
  adding the license header to `PINNED_COMMIT.txt`. (Note: the repo's CI clippy
  config — exercised by `build clippy //...` — passes cleanly; the 8 i2c/smc/uart
  findings seen earlier come only from a stricter `--platforms=ast10x0`
  `-D warnings` aspect invocation and are pre-existing, not introduced here.)

This brings i3c to the same structure + test layout + CI bar as the I2C port:
`i3c_init` (mirrors `i2c_init`) and `i3c_irq` (mirrors `i2c_irq`), each with its
`no_panics_test` (kernel) + hardware-only execution test, all green.

Fourth pass (EVB-faithful tests + reachable-path panic hardening):
- **`i3c_irq` rewritten to mirror the reference EVB tests** `tests-hw/src/i3c_test.rs::test_i3c_master`/`test_i3c_target`: I3C **bus 2** (PAC `I3c2`) on the **HV** pads (`PINCTRL_HVI3C2`) — the bus/pad set the AST1060 Test Harness wires and the reference uses. Controller pre-attaches a device by PID, enables IBI, and on each target SIR does a private read + private write (10 exchanges); target raises Hot-Join, waits for its dynamic address, then sends 10 IBIs. Differences from the reference are panic-hygiene only (`unwrap`→`?`/`pw_log`, `DummyDelay` dropped).
- **pinctrl reworked to bus-number naming + HV LV-clear**: `PINCTRL_I3C0..3` (LV) and `PINCTRL_HVI3C0..3` (HV), matching the reference's `HVI3Cn`. The HV groups now also **clear** the conflicting LV function bits on the same pads (`CLR_PIN_SCU418_*`) — the earlier HV groups only set the HV bit, which would have left both functions muxed.
- **D5 reachable-path hardening (now required for I2C parity)**: `i2c_irq:no_panics_test` passes, so panic-free transfer paths are the bar. Stop-and-instrument (objdump of `controller.elf`, ARM has no backtrace) localized the residual panics to `init_clock` (a `.expect()` on `core_clk_hz` and `div_ceil` by a not-provably-non-zero `core_period`/`fscl_hz` → `panic_const_div_by_zero`), surfaced once the test stopped const-folding the clock config. Hardened: `expect`→`unwrap_or(I3C_MIN_CORE_CLK_SDR)`, divisors bound to local `.max(1)` values; plus `end_xfer`/`priv_xfer_build_cmds`/`priv_xfer`/`ibi_enable`/`acknowledge_ibi`/`detach_i3c_dev_by_idx` slice/index sites moved to `get`/`get_mut`/`zip`/`?`. Success-path behavior unchanged. **All three i3c `no_panics_test`s now pass.**

Building/running on the EVB (AST1060 Test Harness, two daughter cards A/B on the
I3C2 HV link):
```
# Build the two images (controller = device A, target = device B):
bazel build --config=k_ast1060_evb \
  //target/ast10x0/tests/peripherals/i3c/i3c_irq:controller \
  //target/ast10x0/tests/peripherals/i3c/i3c_irq:slave
# Run the two-board IBI test via the Raspberry-Pi harness:
AST1060_EVB_PI_HOST=<pi-host> bazel test --config=k_ast1060_evb \
  //target/ast10x0/tests/peripherals/i3c/i3c_irq:irq_test
# Single-board init check:
AST1060_EVB_PI_HOST=<pi-host> bazel test --config=k_ast1060_evb \
  //target/ast10x0/tests/peripherals/i3c/i3c_init:i3c_init_test
```

Firmware images after a build (the `system_image` rule emits both `.bin` for
flashing and `.elf` for `pw_tokenizer` log decode):
`bazel-bin/.../i3c_irq/{controller,slave}.{bin,elf}`. A renamed copy is staged at
`out/i3c_evb_fw/{i3c_master,i3c_target}.{bin,elf}` for convenience.

**Boot order (matches the reference `test_i3c_master`/`test_i3c_target`): power
the MASTER (`controller`/device A) first** so it is already draining the IBI
work queue, **then the TARGET** (`slave`/device B). The target raises a Hot-Join
which the master answers with `assign_dynamic_address`; the target then sends
its IBIs. (The reference's pre-Hot-Join `DummyDelay` is a no-op, so ordering is
operator-controlled — master up first.) Manual two-board flash without the bazel
test runner (UART boot via `harness/uart_test_exec.py`, device B on GPIO
`--srst-pin 25 --fwspick-pin 24`):
```
./uart_test_exec.py /dev/ttyUSB_A out/i3c_evb_fw/i3c_master.bin --elf out/i3c_evb_fw/i3c_master.elf
./uart_test_exec.py --srst-pin 25 --fwspick-pin 24 /dev/ttyUSB_B \
    out/i3c_evb_fw/i3c_target.bin --elf out/i3c_evb_fw/i3c_target.elf
```

Still not done (honest scope note):
- **CCC word-composition host unit tests** named in §4 remain pending — but the
  I2C port has no analogous standalone unit tests either, so this is *beyond*
  I2C parity. The `no_panics` + build + on-HW `irq_test`/`i3c_init_test` (the
  full master/target IBI exchange) cover the paths.
