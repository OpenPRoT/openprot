// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Register-field mux mechanism: inline `FieldWrite`s coalesced to one RMW per register at compile time.

use crate::resource::{Capability, FieldWrite, Pin, Routes};
use core::marker::PhantomData;

/// Fixed-size compile-time coalescing scratch; only `[..len]` of it is ever live.
// TODO: the dead slots are not trimmed. `as_slice()` runs at runtime against a const-promoted
// `RegBatch`, so the backing allocation is always MAX_REGS wide (196 bytes on thumbv7m) with the
// unused entries zeroed. Whether they reach the binary is the optimizer's call: at -O1/-O3 the
// apply loop folds to immediate instructions and nothing lands, but at opt-level=0 — Bazel's
// default fastbuild, which `--config=k_ast1060_evb` builds with — the whole table lands plus a
// runtime loop. Trimming it structurally needs `generic_const_exprs`, dropped deliberately; the
// waste is bounded at one `scu::route` monomorphization per image until that comes back.
pub(crate) const MAX_REGS: usize = 16;

/// Central batch of register writes, coalesced to one op per distinct offset: `[..len]` of `ops` live.
#[derive(Clone, Copy)]
pub struct RegBatch {
    /// Register ops, one per distinct offset; only `[..len]` are valid.
    ops: [FieldWrite; MAX_REGS],
    /// Number of live ops.
    len: usize,
}

impl RegBatch {
    /// An empty accumulator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ops: [FieldWrite {
                offset: 0,
                set: 0,
                clr: 0,
            }; MAX_REGS],
            len: 0,
        }
    }

    /// Coalesce one write into the batch, grouped by register offset via [`find`](Self::find).
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
    pub(crate) const fn append_batch(&mut self, other: &RegBatch) {
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

impl Default for RegBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// A register block a coalesced mux batch can be applied to: one read + one write per offset.
/// A target impls these two methods once; `apply` replaces a per-offset match in the applier.
pub trait RegBlock {
    /// Read the 32-bit register at `offset`.
    fn read_reg(&self, offset: u32) -> u32;
    /// Write `val` to the 32-bit register at `offset`.
    fn write_reg(&self, offset: u32, val: u32);
}

/// Apply an exact-N mux batch to a block: one RMW (`reg = (reg & !clr) | set`) per register. Panic-free — a flat walk over the tight slice, no indexing.
pub fn apply(block: &impl RegBlock, batch: &[FieldWrite]) {
    for op in batch {
        block.write_reg(op.offset, (block.read_reg(op.offset) & !op.clr) | op.set);
    }
}

/// Coalesce a list of per-pin routes into one batch (one RMW per register) — the compile-time fold.
#[must_use]
pub const fn coalesce_routes(routes: &[&[FieldWrite]]) -> RegBatch {
    let mut b = RegBatch::new();
    let mut i = 0;
    while i < routes.len() {
        b.append(routes[i]);
        i += 1;
    }
    b
}

/// A bound handle (or tuple of handles) whose SCU mux routes are folded to one RMW per register at
/// compile time in [`COALESCED`](MuxRoutes::COALESCED); the applier just flushes that const.
pub trait MuxRoutes {
    /// This handle's routes, coalesced to one op per register — folded per monomorphization at compile time.
    const COALESCED: RegBatch;
}

/// Route a handle through a shared reference: the applier reads only the type, so a borrow folds identically — non-Copy owners route without being consumed.
impl<H: MuxRoutes> MuxRoutes for &H {
    const COALESCED: RegBatch = H::COALESCED;
}

/// Impl [`MuxRoutes`] for every tuple arity by folding each member's `COALESCED` into one batch at
/// compile time; recurses on the tail, so one invocation covers all arities down to the 1-tuple.
macro_rules! impl_muxroutes_tuple {
    () => {};
    ($head:ident $(, $tail:ident)*) => {
        impl<$head: MuxRoutes $(, $tail: MuxRoutes)*> MuxRoutes for ($head, $($tail,)*) {
            const COALESCED: RegBatch = {
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
impl_muxroutes_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

/// A pin consumed purely to route it — routing only ever flushes the SCU's mux-select bits, never
/// a capability's own registers, so there is nothing to grant access to and no block to bind
/// through. This is the kernel's routing-only alternative to binding a full capability handle
/// (e.g. `GpioBlock`/`GpioPin`) just to reach [`MuxRoutes`].
pub struct Route<C, P> {
    _cap: PhantomData<C>,
    _pin: PhantomData<P>,
}

impl<C: Capability, P: Routes<C> + Pin> Route<C, P> {
    /// Consume a pin token purely to route it.
    #[must_use]
    pub fn new(_pin: P) -> Self {
        Self {
            _cap: PhantomData,
            _pin: PhantomData,
        }
    }
}

impl<C: Capability, P: Routes<C>> MuxRoutes for Route<C, P> {
    const COALESCED: RegBatch = coalesce_routes(&[<P as Routes<C>>::ROUTE]);
}

/// A silicon-fixed singleton register block (e.g. the SCU): `BASE` names it, no pin token carries it.
///
/// # Safety
/// `BASE` must name a valid, `'static`, aligned register block usable for `u32` volatile access at every offset its driver touches.
#[allow(unsafe_code)]
pub unsafe trait Block {
    /// The silicon-fixed base of this singleton register block.
    const BASE: *const u8;
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
    #[must_use]
    pub fn read_reg(&self, offset: u32) -> u32 {
        // SAFETY: `base` names a valid register block sized for `u32` access at every offset used.
        #[allow(unsafe_code)]
        // nosemgrep
        unsafe {
            self.base.add(offset as usize).cast::<u32>().read_volatile()
        }
    }

    /// Write `val` to the 32-bit register at byte `offset`.
    pub fn write_reg(&self, offset: u32, val: u32) {
        // SAFETY: `base` names a valid register block sized for `u32` access at every offset used.
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

impl<B> RegBlock for Mmio<B> {
    fn read_reg(&self, offset: u32) -> u32 {
        Mmio::read_reg(self, offset)
    }
    fn write_reg(&self, offset: u32, val: u32) {
        Mmio::write_reg(self, offset, val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{clear, set};

    #[test]
    fn append_merges_same_offset() {
        let mut batch = RegBatch::new();
        batch.append(&[set(0x10, 0), set(0x10, 1)]);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.as_slice()[0].offset, 0x10);
        assert_eq!(batch.as_slice()[0].set, 0b11);
    }

    #[test]
    fn append_keeps_distinct_offsets_separate() {
        let mut batch = RegBatch::new();
        batch.append(&[set(0x10, 0), clear(0x14, 0)]);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn new_batch_is_empty() {
        let batch = RegBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn append_batch_folds_in_another_batchs_ops() {
        let mut a = RegBatch::new();
        a.append(&[set(0x10, 0)]);
        let mut b = RegBatch::new();
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

        let batch = coalesce_routes(&[a, b]);
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
        let route = Route::<TestCap, TestPin>::new(TestPin);
        assert_eq!(Route::<TestCap, TestPin>::COALESCED.len(), 2);
        let _ = route; // consumed purely to prove `new` takes the pin by value
    }
}
