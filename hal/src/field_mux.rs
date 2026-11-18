// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Register-field mux mechanism: inline `FieldWrite`s coalesced to one RMW per register at compile time.

use crate::resource::{Capability, FieldWrite, Pin, Routes};
use core::marker::PhantomData;

/// Max distinct register offsets [`RegBatch`] can hold after coalescing (merging per-pin writes
/// that land on the same register into one). Backs a fixed-size array; `[..len]` of it is live,
/// the rest is zeroed padding — 16 is arbitrary, just comfortably above any real board's count.
// TODO: the dead slots are not trimmed. `as_slice()` runs at runtime against a const-promoted
// `RegBatch`, so the backing allocation is always MAX_REGS wide (196 bytes on thumbv7m) with the
// unused entries zeroed. Whether they reach the binary is the optimizer's call: at -O1/-O3 the
// apply loop folds to immediate instructions and nothing lands, but at opt-level=0 — Bazel's
// default fastbuild, which `--config=k_ast1060_evb` builds with — the whole table lands plus a
// runtime loop. Trimming it structurally needs `generic_const_exprs`, dropped deliberately; the
// waste is bounded at one `scu::route` monomorphization per image until that comes back.
pub(crate) const MAX_REGS: usize = 16;

/// Central batch of register writes, coalesced to one op per distinct offset: `[..len]` of `ops`
/// live. Block-typed, so one block's batch can't be applied to another's — but coalescing itself
/// never checks `B::SIZE`; that proof lives in [`ProvenInBounds`], not here (see its doc for why).
#[derive(Clone, Copy)]
pub struct RegBatch<B> {
    /// Register ops, one per distinct offset; only `[..len]` are valid.
    ops: [FieldWrite; MAX_REGS],
    /// Number of live ops.
    len: usize,
    _block: PhantomData<B>,
}

impl<B: Block> RegBatch<B> {
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
    /// Does not check `w.offset` against `B::SIZE` — intentionally: this is a plain `const fn`,
    /// callable from both compile-time and ordinary runtime code, and whether its `assert!`s would
    /// ever run at runtime depends on the caller, not on this fn. [`ProvenInBounds`] carries the
    /// bounds proof instead, specifically because it's an associated const: evaluating one is
    /// always the compiler's job, with no runtime-reachable call path, regardless of where it's
    /// referenced from.
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

impl<B: Block> Default for RegBatch<B> {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply handle `H`'s coalesced routes to its block: one RMW (`reg = (reg & !clr) | set`) per
/// register. Panic-free, at compile time *and* at runtime: referencing `ProvenInBounds::PROVEN`
/// below evaluates its bounds check purely at compile time (see that trait's own doc for why),
/// and the loop itself is a flat walk over an already-built tight slice, no indexing, no asserts
/// of its own. Safe: `PROVEN` having been referenced at all is what proves every offset in
/// `H::COALESCED` fits `B::SIZE` — not a trust-me comment, a proof the caller couldn't skip.
pub fn apply<B: Block, H: MuxRoutes<B>>(block: &Mmio<B>, _handle: &H) {
    let () = <H as ProvenInBounds<B>>::PROVEN;
    for op in H::COALESCED.as_slice() {
        // SAFETY: `ProvenInBounds::PROVEN` above proves every offset in `H::COALESCED` is
        // in-bounds for `B`.
        #[allow(unsafe_code)]
        unsafe {
            block.write_reg(op.offset, (block.read_reg(op.offset) & !op.clr) | op.set);
        }
    }
}

/// Proof that every offset in `H::COALESCED` fits inside block `B`'s real extent — split out from
/// `RegBatch::push` specifically so the check can only ever run at compile time.
///
/// `push` is a plain `const fn`: whether a given call to it runs at compile time or at runtime is
/// decided by its caller, not by `push` itself — a `const fn` called from ordinary runtime code
/// runs (and can panic) at runtime, same as any other function. An associated const has no such
/// ambiguity: the compiler evaluates `PROVEN`'s initializer to produce its value regardless of
/// where or how it's referenced, even from inside an otherwise-ordinary runtime function body (see
/// [`apply`]). That's what makes this trait, not a check inside `push`, the place this proof lives.
pub trait ProvenInBounds<B: Block>: MuxRoutes<B> {
    /// Evaluating this forces the bounds check; referencing it is what makes it run.
    const PROVEN: ();
}

impl<B: Block, H: MuxRoutes<B>> ProvenInBounds<B> for H {
    const PROVEN: () = {
        let ops = H::COALESCED.as_slice();
        let mut i = 0;
        while i < ops.len() {
            assert!(
                (ops[i].offset as usize) + 4 <= B::SIZE,
                "register offset reaches past this block's real extent"
            );
            i += 1;
        }
    };
}

/// Coalesce a list of per-pin routes into one batch (one RMW per register) — the compile-time
/// fold. Crate-private along with [`RegBatch::new`] — see that doc for why.
#[must_use]
pub(crate) const fn coalesce_routes<B: Block>(routes: &[&[FieldWrite]]) -> RegBatch<B> {
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
pub trait MuxRoutes<B: Block> {
    /// This handle's routes, coalesced to one op per register — folded per monomorphization at compile time.
    const COALESCED: RegBatch<B>;
}

/// Route a handle through a shared reference: the applier reads only the type, so a borrow folds identically — non-Copy owners route without being consumed.
impl<B: Block, H: MuxRoutes<B>> MuxRoutes<B> for &H {
    const COALESCED: RegBatch<B> = H::COALESCED;
}

/// Impl [`MuxRoutes`] for every tuple arity by folding each member's `COALESCED` into one batch at
/// compile time; recurses on the tail, so one invocation covers all arities down to the 1-tuple.
/// Tuple positions are named `P0..` (not `A..`) so they don't collide with the block parameter `B`.
macro_rules! impl_muxroutes_tuple {
    () => {};
    ($head:ident $(, $tail:ident)*) => {
        impl<B: Block, $head: MuxRoutes<B> $(, $tail: MuxRoutes<B>)*> MuxRoutes<B> for ($head, $($tail,)*) {
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

impl<C: Capability, P: Routes<C>, B: Block> MuxRoutes<B> for Route<C, P, B> {
    const COALESCED: RegBatch<B> = coalesce_routes(&[<P as Routes<C>>::ROUTE]);
}

/// A silicon-fixed singleton register block (e.g. the SCU): `BASE` names it, no pin token carries it.
///
/// # Safety
/// `BASE` must name a valid, `'static`, aligned register block usable for `u32` volatile access at
/// every offset less than `SIZE`; `SIZE` must be the real byte extent of that register block (e.g.
/// `size_of::<ast1060_pac::scu::RegisterBlock>()`), not an approximation — it's what an offset gets
/// checked against before the block is ever touched.
#[allow(unsafe_code)]
pub unsafe trait Block {
    /// The silicon-fixed base of this singleton register block.
    const BASE: *const u8;
    /// The real byte extent of this register block — an offset at or past this is out of bounds.
    const SIZE: usize;
}

/// Confined MMIO applier over block `B`, phantom-typed so one block's applier can't stand in for another's.
pub struct Mmio<B> {
    base: *const u8,
    _block: PhantomData<B>,
}

impl<B> Clone for Mmio<B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<B> Copy for Mmio<B> {}

impl<B> Mmio<B> {
    /// HAL-internal ctor: base always rides on authored `DATA` or a `Block::BASE`, whose validity was promised upstream.
    #[must_use]
    pub(crate) const fn from_raw(base: *const u8) -> Self {
        Self {
            base,
            _block: PhantomData,
        }
    }

    /// Read the 32-bit register at byte `offset`.
    /// # Safety
    /// `offset` must be a valid, in-bounds byte offset for this register block — this fn only
    /// knows `base` is valid (promised by `from_raw`/[`block`](Self::block)),
    /// not that `offset` lands inside it; that's on the caller.
    #[must_use]
    #[allow(unsafe_code)]
    pub unsafe fn read_reg(&self, offset: u32) -> u32 {
        // SAFETY: caller ensures `offset` is in-bounds for this block, per this fn's own doc.
        #[allow(unsafe_code)]
        // nosemgrep
        unsafe {
            self.base.add(offset as usize).cast::<u32>().read_volatile()
        }
    }

    /// Write `val` to the 32-bit register at byte `offset`.
    /// # Safety
    /// `offset` must be a valid, in-bounds byte offset for this register block — this fn only
    /// knows `base` is valid (promised by `from_raw`/[`block`](Self::block)),
    /// not that `offset` lands inside it; that's on the caller.
    #[allow(unsafe_code)]
    pub unsafe fn write_reg(&self, offset: u32, val: u32) {
        // SAFETY: caller ensures `offset` is in-bounds for this block, per this fn's own doc.
        #[allow(unsafe_code)]
        // nosemgrep
        unsafe {
            (self.base.add(offset as usize) as *mut u32).write_volatile(val)
        }
    }
}

impl<B: Block> Mmio<B> {
    /// Mint a token-less singleton's applier from its `Block::BASE`; safe, the `unsafe impl Block` made the promise.
    #[must_use]
    pub const fn block() -> Self {
        Self::from_raw(B::BASE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{clear, set};

    struct TestBlock;
    #[allow(unsafe_code)]
    unsafe impl Block for TestBlock {
        const BASE: *const u8 = core::ptr::null();
        const SIZE: usize = 0x100;
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

    // `ProvenInBounds::PROVEN`'s failure case (an offset past `B::SIZE`) is a compile error, not a
    // runtime panic, so it can't be exercised with `#[should_panic]` the way `push`'s old in-fn
    // assert could. This only proves the mechanism itself evaluates cleanly for a route that's
    // actually in bounds; the rejecting case was confirmed by hand (a temporary out-of-range
    // `ROUTE` entry on `TestPin` fails the build, as expected) rather than left unverified.
    #[test]
    fn proven_in_bounds_accepts_a_route_within_the_block() {
        let () = <Route<TestCap, TestPin, TestBlock> as ProvenInBounds<TestBlock>>::PROVEN;
    }
}
