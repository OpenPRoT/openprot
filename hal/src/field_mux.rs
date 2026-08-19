// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Register-field mux mechanism: inline `FieldWrite`s coalesced to one RMW per register at compile time.

use crate::resource::FieldWrite;
use core::marker::PhantomData;

/// Fixed-size compile-time coalescing scratch; narrowed to the exact live count before apply, so it never reaches the binary.
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

    /// Live op count — the exact `N` [`into_ops`](Self::into_ops) narrows to so the fixed scratch tail never reaches the binary.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True if no route has been appended yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Consume the batch into its exact `[FieldWrite; N]` ops (no zeroed tail); const-panics unless `N == len`.
    #[must_use]
    pub const fn into_ops<const N: usize>(self) -> [FieldWrite; N] {
        assert!(self.len == N, "into_ops: N must equal batch len");
        let mut out = [FieldWrite {
            offset: 0,
            set: 0,
            clr: 0,
        }; N];
        let mut k = 0;
        while k < N {
            out[k] = self.ops[k];
            k += 1;
        }
        out
    }

    // HACK: the MAX_REGS-wide `ops` rides to rodata; `into_ops` trims it but needs generic_const_exprs.
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
    /// Distinct-register count of [`COALESCED`](Self::COALESCED) — the exact array length to narrow to before apply.
    const LEN: usize = Self::COALESCED.len();
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

// HACK: tuple arity caps pins-per-apply at this ident count (no variadic generics); widen the list if needed.
impl_muxroutes_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

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

    /// True if `bit` is set in the register at byte `offset`.
    #[must_use]
    pub fn read_bit(&self, offset: u32, bit: u8) -> bool {
        self.read_reg(offset) & (1 << bit) != 0
    }

    /// Read-modify-write a `mask`-wide field at `shift` in the register at `offset` to `val`.
    pub fn modify_field(&self, offset: u32, shift: u8, mask: u32, val: u32) {
        let cur = self.read_reg(offset);
        self.write_reg(offset, (cur & !(mask << shift)) | ((val & mask) << shift));
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
