// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Register-field mux mechanism: inline `FieldWrite`s coalesced to one RMW per register at compile time.

use crate::resource::{Capability, FieldWrite, Pin, Routes};
use core::marker::PhantomData;

/// The size of the compile-time coalescing scratch [`RegBatch`] backs its `ops` array with — 16
/// is arbitrary, just comfortably above any real board's count.
pub(crate) const MAX_REGS: usize = 16;

/// Central batch of register writes, coalesced to one op per distinct offset: `[..len]` of `ops`
/// live. Identity-typed (`B`, e.g. a chip's `ScuBlock`), so one block's batch can't be applied to
/// another's — but coalescing itself never checks an accessor's `SIZE`; that proof lives in
/// [`ProvenInBounds`], not here (see its doc for why).
#[derive(Clone, Copy)]
pub struct RegBatch<B> {
    /// The compile-time coalescing scratch: one op per distinct register offset, merging per-pin
    /// writes that land on the same register into one. Only `[..len]` of it is ever live, the
    /// rest is zeroed padding.
    // TODO: the dead slots are not trimmed. `as_slice()` runs at runtime against a const-promoted
    // `RegBatch`, so the backing allocation is always MAX_REGS wide (196 bytes on thumbv7m) with the
    // unused entries zeroed. Whether they reach the binary is the optimizer's call: at -O1/-O3 the
    // apply loop folds to immediate instructions and nothing lands, but at opt-level=0 — Bazel's
    // default fastbuild, which `--config=k_ast1060_evb` builds with — the whole table lands plus a
    // runtime loop. Trimming it structurally needs `generic_const_exprs`, dropped deliberately; the
    // waste is bounded at one `scu::route` monomorphization per image until that comes back.
    ops: [FieldWrite; MAX_REGS],
    /// Number of live ops.
    len: usize,
    _block: PhantomData<B>,
}

impl<B> RegBatch<B> {
    /// An empty accumulator. Crate-private: with [`coalesce_routes`] and [`append`](Self::append)
    /// (also crate-private), this is the only way to put anything into a `RegBatch` — nothing
    /// outside this crate can construct one with any op in it, which is what keeps every
    /// `MuxRoutes` impl's `COALESCED` traceable back to a real pin's own [`Routes`] impl, not a
    /// hand-picked offset. No sealed-trait machinery needed for that; it falls out of visibility.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            ops: [FieldWrite {
                offset: 0,
                set: 0,
                clr: 0,
            }; MAX_REGS],
            len: 0,
            _block: PhantomData,
        }
    }

    /// Coalesce one write into the batch, grouped by register offset via [`find`](Self::find).
    /// Does not check `w.offset` against any accessor's `SIZE` — intentionally: this is a plain
    /// `const fn`, callable from both compile-time and ordinary runtime code, and whether its
    /// `assert!`s would ever run at runtime depends on the caller, not on this fn. [`ProvenBatch`]
    /// carries the bounds proof instead, specifically because it's built inside an associated
    /// const's initializer: evaluating one is always the compiler's job, with no runtime-reachable
    /// call path, regardless of where it's referenced from.
    const fn push(&mut self, w: FieldWrite) {
        let j = self.find(w.offset);
        if j == self.len {
            assert!(self.len < MAX_REGS, "MAX_REGS too small");
            self.ops[self.len] = w;
            self.len += 1;
        } else {
            self.ops[j].coalesce(w);
        }
    }

    ///merges via `FieldWrite::coalesce`. Crate-private, can only be called by fns with pin token authority provided.
    pub(crate) const fn append(&mut self, writes: &[FieldWrite]) {
        let mut i = 0;
        while i < writes.len() {
            self.push(writes[i]);
            i += 1;
        }
    }

    /// Merge another batch's live ops in, coalescing per register — the tuple/handle-list fold step.
    pub(crate) const fn append_batch(&mut self, other: &RegBatch<B>) {
        let mut i = 0;
        while i < other.len {
            self.push(other.ops[i]);
            i += 1;
        }
    }

    /// Live op count.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True if no route has been appended yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The live ops as a tight slice — `[..len]` of the fixed scratch.
    #[must_use]
    pub const fn as_slice(&self) -> &[FieldWrite] {
        self.ops.split_at(self.len).0
    }

    /// Index of `offset` in `ops[..len]`, or `len` if absent — group-by-register for [`append`].
    const fn find(&self, offset: u32) -> usize {
        let mut j = 0;
        while j < self.len {
            if self.ops[j].offset == offset {
                return j;
            }
            j += 1;
        }
        self.len
    }
}

impl<B> Default for RegBatch<B> {
    fn default() -> Self {
        Self::new()
    }
}

/// A byte offset already proven to lie inside accessor `A`'s real extent. Private field: outside
/// this crate, an `Offset<A>` cannot be brought into existence, only passed along or built via
/// [`at`](Self::at)'s compile-time check.
pub struct Offset<A> {
    byte: u32,
    _a: PhantomData<A>,
}

impl<A> Clone for Offset<A> {
    fn clone(&self) -> Self {
        Self {
            byte: self.byte,
            _a: PhantomData,
        }
    }
}

impl<A: RegisterAccess> Offset<A> {
    /// Build a proven offset from a const generic — the bound is forced at compile time, the same
    /// way [`ProvenBatch`] forces it over a whole table: there is no way to name `OFFSET` except as
    /// this literal, so there is no way to call this with one that wasn't just checked against `A::SIZE`.
    #[must_use]
    pub const fn at<const OFFSET: u32>() -> Self {
        const {
            assert!(
                (OFFSET as usize) + 4 <= A::SIZE,
                "register offset reaches past this block's real extent"
            );
        }
        Self {
            byte: OFFSET,
            _a: PhantomData,
        }
    }

    /// Mint without a const-generic literal — only for code that already ran the same bounds check
    /// over data, e.g. [`ProvenBatch::prove`]. Crate-private: this is the one minting surface to audit.
    #[must_use]
    pub(crate) const fn trusted(byte: u32) -> Self {
        Self {
            byte,
            _a: PhantomData,
        }
    }

    /// The wrapped offset, for the accessor's own `read`/`write` body to add to its base.
    #[must_use]
    pub const fn get(&self) -> u32 {
        self.byte
    }
}

/// Implemented once per target register block — the address and the `unsafe` both live in the
/// target's impl, never in `hal`. Keyed on `Self` (not a separable identity trait) so a bogus impl
/// can only ever mint offsets accepted by its own (equally bogus) read/write body: there is no
/// shared "vend a `BASE`/`SIZE`" trait a forged impl could ride in on.
pub trait RegisterAccess: Sized + Default {
    /// Identity marker so routing data (`MuxRoutes`/`RegBatch`) can target "this logical block"
    /// independent of how it's accessed.
    type Block;
    /// This accessor's real physical base address — used by grant checks (`util_region::covers`)
    /// to prove a manifest region actually contains this block; never turned into a pointer in
    /// `hal` itself, only compared as a plain integer.
    const BASE: usize;
    /// This accessor's real byte extent — an offset at or past this is out of bounds.
    const SIZE: usize;
    /// Read the 32-bit register at `offset`.
    fn read(&self, offset: Offset<Self>) -> u32;
    /// Write `val` to the 32-bit register at `offset`.
    fn write(&self, offset: Offset<Self>, val: u32);

    /// Read the 32-bit register at byte offset `OFFSET`. `OFFSET` is a const generic, so the bounds
    /// check is forced at compile time the same way [`Offset::at`] forces it.
    #[must_use]
    fn read_at<const OFFSET: u32>(&self) -> u32 {
        self.read(Offset::at::<OFFSET>())
    }

    /// Write `val` to the 32-bit register at byte offset `OFFSET`. Same proof as [`read_at`](Self::read_at).
    fn write_at<const OFFSET: u32>(&self, val: u32) {
        self.write(Offset::at::<OFFSET>(), val)
    }
}

/// One proven register op: a read offset and a write offset (usually the same one; GPIO's
/// `in_level`/`out_level` read-swap is why they're kept separate), both already checked in-bounds
/// for `A`, plus the bits to fold in.
pub struct ProvenOp<A> {
    pub(crate) read: Offset<A>,
    pub(crate) write: Offset<A>,
    pub(crate) set: u32,
    pub(crate) clr: u32,
}

/// A [`RegBatch`] whose every offset has been proven to fit inside accessor `A`'s real extent —
/// the only way to get one is this crate's own `prove`, which checks every entry once; `ops`
/// then replays those exact, already-checked offsets as [`Offset<A>`] values. `apply` never mints
/// an `Offset` of its own, it only ever receives one this type already proved.
pub struct ProvenBatch<A: RegisterAccess> {
    batch: RegBatch<A::Block>,
    _a: PhantomData<A>,
}

impl<A: RegisterAccess> ProvenBatch<A> {
    /// Check every offset in `batch` against `A::SIZE` once, so [`ops`](Self::ops) can trust every
    /// one it ever yields. Split out from `RegBatch::push` specifically so the check can only ever
    /// run at compile time: `push` is a plain `const fn`, runnable from ordinary runtime code too,
    /// but this only ever runs from inside an associated const's initializer (see [`ProvenInBounds`]),
    /// which the compiler evaluates regardless of where it's referenced from.
    pub(crate) const fn prove(batch: RegBatch<A::Block>) -> Self {
        let ops = batch.as_slice();
        let mut i = 0;
        while i < ops.len() {
            assert!(
                (ops[i].offset as usize) + 4 <= A::SIZE,
                "register offset reaches past this block's real extent"
            );
            i += 1;
        }
        Self {
            batch,
            _a: PhantomData,
        }
    }

    /// This batch's ops, each already wrapped as a proven [`Offset<A>`] — the read and write side
    /// are the same offset here (field_mux's RMW never swaps registers the way GPIO's does).
    pub(crate) fn ops(&self) -> impl Iterator<Item = ProvenOp<A>> + '_ {
        self.batch.as_slice().iter().map(|op| ProvenOp {
            read: Offset::trusted(op.offset),
            write: Offset::trusted(op.offset),
            set: op.set,
            clr: op.clr,
        })
    }
}

/// Apply handle `H`'s coalesced routes to accessor `A`: one RMW (`reg = (reg & !clr) | set`) per
/// register. No `unsafe` here: `A::read`/`A::write` are safe trait methods whose own impl (in the
/// target crate) holds the `unsafe`; this function never dereferences anything.
pub fn apply<A: RegisterAccess + 'static, H: MuxRoutes<A::Block>>(acc: &A, _handle: &H) {
    for op in <H as ProvenInBounds<A>>::PROVEN.ops() {
        acc.write(op.write, (acc.read(op.read) & !op.clr) | op.set);
    }
}

/// Proof that every offset in `H::COALESCED` fits inside accessor `A`'s real extent, carried as a
/// value (not a bare `()`): the proof and the data it proves things about are the same object, so
/// `apply` has no way to reach `A::read`/`A::write` with an offset this never checked.
pub trait ProvenInBounds<A: RegisterAccess + 'static>: MuxRoutes<A::Block> {
    /// Evaluating this forces the bounds check; referencing it is what makes it run.
    const PROVEN: &'static ProvenBatch<A>;
}

impl<A: RegisterAccess + 'static, H: MuxRoutes<A::Block>> ProvenInBounds<A> for H {
    const PROVEN: &'static ProvenBatch<A> = &ProvenBatch::prove(H::COALESCED);
}

/// Coalesce a list of per-pin routes into one batch (one RMW per register) — the compile-time
/// fold. Crate-private along with [`RegBatch::new`] — see that doc for why.
#[must_use]
pub(crate) const fn coalesce_routes<B>(routes: &[&[FieldWrite]]) -> RegBatch<B> {
    let mut b = RegBatch::new();
    let mut i = 0;
    while i < routes.len() {
        b.append(routes[i]);
        i += 1;
    }
    b
}

/// A bound handle (or tuple of handles) whose mux routes target block `B`, folded to one RMW per
/// register at compile time in [`COALESCED`](MuxRoutes::COALESCED); the applier just flushes that
/// const. Anyone can implement this trait, but `coalesce_routes` and `RegBatch::new` being
/// crate-private means no implementor outside this crate can give `COALESCED` any op that didn't
/// already come from a real pin's own [`Routes`] impl — see those docs for why.
pub trait MuxRoutes<B> {
    /// This handle's routes, coalesced to one op per register — folded per monomorphization at compile time.
    const COALESCED: RegBatch<B>;
}

/// Route a handle through a shared reference: the applier reads only the type, so a borrow folds identically — non-Copy owners route without being consumed.
impl<B, H: MuxRoutes<B>> MuxRoutes<B> for &H {
    const COALESCED: RegBatch<B> = H::COALESCED;
}

/// Impl [`MuxRoutes`] for every tuple arity by folding each member's `COALESCED` into one batch at
/// compile time; recurses on the tail, so one invocation covers all arities down to the 1-tuple.
/// Tuple positions are named `P0..` (not `A..`) so they don't collide with the block parameter `B`.
macro_rules! impl_muxroutes_tuple {
    () => {};
    ($head:ident $(, $tail:ident)*) => {
        impl<B, $head: MuxRoutes<B> $(, $tail: MuxRoutes<B>)*> MuxRoutes<B> for ($head, $($tail,)*) {
            const COALESCED: RegBatch<B> = {
                let mut out = RegBatch::new();
                out.append_batch(&$head::COALESCED);
                $( out.append_batch(&$tail::COALESCED); )*
                out
            };
        }
        impl_muxroutes_tuple!($($tail),*);
    };
}

// This list caps flat-tuple arity, not the real limit: the impl is generic over any MuxRoutes
// member, so a group past this count nests, e.g. `((a, b, c), (d, e, f), ...)`, rather than
// needing the list widened.
impl_muxroutes_tuple!(P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);

/// A pin consumed purely to route it — routing only ever flushes the SCU's mux-select bits, never
/// a capability's own registers, so there is nothing to grant access to and no block to bind
/// through. This is the kernel's routing-only alternative to binding a full capability handle
/// (e.g. `GpioBlock`/`GpioPin`) just to reach [`MuxRoutes`].
pub struct Route<C, P, B> {
    _cap: PhantomData<C>,
    _pin: PhantomData<P>,
    _block: PhantomData<B>,
}

impl<C: Capability, P: Routes<C> + Pin, B> Route<C, P, B> {
    /// Consume a pin token purely to route it.
    #[must_use]
    pub fn new(_pin: P) -> Self {
        Self {
            _cap: PhantomData,
            _pin: PhantomData,
            _block: PhantomData,
        }
    }
}

impl<C: Capability, P: Routes<C>, B> MuxRoutes<B> for Route<C, P, B> {
    const COALESCED: RegBatch<B> = coalesce_routes(&[<P as Routes<C>>::ROUTE]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{clear, set};
    use core::cell::Cell;

    struct TestBlock;

    struct TestAccess {
        regs: Cell<[u32; 0x100 / 4]>,
    }

    impl Default for TestAccess {
        fn default() -> Self {
            Self {
                regs: Cell::new([0; 0x100 / 4]),
            }
        }
    }

    impl RegisterAccess for TestAccess {
        type Block = TestBlock;
        const BASE: usize = 0;
        const SIZE: usize = 0x100;

        fn read(&self, offset: Offset<Self>) -> u32 {
            self.regs.get()[(offset.get() / 4) as usize]
        }

        fn write(&self, offset: Offset<Self>, val: u32) {
            let mut regs = self.regs.get();
            regs[(offset.get() / 4) as usize] = val;
            self.regs.set(regs);
        }
    }

    #[test]
    fn append_merges_same_offset() {
        let mut batch = RegBatch::<TestBlock>::new();
        batch.append(&[set(0x10, 0), set(0x10, 1)]);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.as_slice()[0].offset, 0x10);
        assert_eq!(batch.as_slice()[0].set, 0b11);
    }

    #[test]
    fn append_keeps_distinct_offsets_separate() {
        let mut batch = RegBatch::<TestBlock>::new();
        batch.append(&[set(0x10, 0), clear(0x14, 0)]);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn new_batch_is_empty() {
        let batch = RegBatch::<TestBlock>::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn append_batch_folds_in_another_batchs_ops() {
        let mut a = RegBatch::<TestBlock>::new();
        a.append(&[set(0x10, 0)]);
        let mut b = RegBatch::<TestBlock>::new();
        b.append(&[set(0x10, 1), clear(0x14, 0)]);

        a.append_batch(&b);
        assert_eq!(a.len(), 2);
        let merged = a
            .as_slice()
            .iter()
            .find(|op| op.offset == 0x10)
            .expect("offset 0x10 present");
        assert_eq!(merged.set, 0b11);
    }

    #[test]
    fn coalesce_routes_folds_every_pin_into_one_batch() {
        let a: &[FieldWrite] = &[set(0x10, 0)];
        let b: &[FieldWrite] = &[set(0x10, 1), clear(0x14, 2)];

        let batch: RegBatch<TestBlock> = coalesce_routes(&[a, b]);
        assert_eq!(batch.len(), 2);
    }

    struct TestCap;
    impl Capability for TestCap {
        type Data = ();
    }

    struct TestPin;
    impl Pin for TestPin {}
    impl Routes<TestCap> for TestPin {
        const ROUTE: &'static [FieldWrite] = &[set(0x10, 0), clear(0x14, 1)];
        const DATA: () = ();
    }

    #[test]
    fn route_coalesces_straight_off_the_pins_route_with_no_grant_needed() {
        let route = Route::<TestCap, TestPin, TestBlock>::new(TestPin);
        assert_eq!(Route::<TestCap, TestPin, TestBlock>::COALESCED.len(), 2);
        let _ = route; // consumed purely to prove `new` takes the pin by value
    }

    // `ProvenInBounds::PROVEN`'s failure case (an offset past `A::SIZE`) is a compile error, not a
    // runtime panic, so it can't be exercised with `#[should_panic]` the way `push`'s old in-fn
    // assert could. This only proves the mechanism itself evaluates cleanly for a route that's
    // actually in bounds; the rejecting case was confirmed by hand (a temporary out-of-range
    // `ROUTE` entry on `TestPin` fails the build, as expected) rather than left unverified.
    #[test]
    fn proven_in_bounds_accepts_a_route_within_the_block() {
        let _ = <Route<TestCap, TestPin, TestBlock> as ProvenInBounds<TestAccess>>::PROVEN;
    }

    #[test]
    fn apply_performs_a_real_read_modify_write_with_no_unsafe() {
        let acc = TestAccess::default();
        let route = Route::<TestCap, TestPin, TestBlock>::new(TestPin);
        apply(&acc, &route);
        assert_eq!(acc.read(Offset::trusted(0x10)), 0b1);
        assert_eq!(acc.read(Offset::trusted(0x14)), 0);
    }
}
