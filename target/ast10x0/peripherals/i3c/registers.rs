// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Confined-`unsafe` MMIO façade over the per-bus I3C register blocks.
//!
//! One driver manages multiple bus instances: the bus is selected at
//! **runtime** by index (no per-instance type parameter), mirroring the
//! reference `aspeed-rust` driver. All `unsafe` needed to touch the I3C,
//! I3C-global, and SCU register blocks is confined to this type — one
//! `unsafe fn` constructor and three private deref helpers — so the rest of
//! the driver (`hardware.rs` upward) is `unsafe`-free for MMIO.

use core::marker::PhantomData;

use super::constants::MAX_BUSES;

/// Safe wrapper around the I3C / I3C-global / SCU hardware registers of one bus.
///
/// This struct consolidates all unsafe I3C MMIO access. All register
/// operations go through this single point, making it easy to audit safety
/// invariants — the same shape as `SmcRegisters` in `smc/registers.rs`.
///
/// Not `Copy`/`Clone`: an `I3cRegisters` represents exclusive ownership of
/// one bus's register blocks.
pub struct I3cRegisters {
    i3c: *const ast1060_pac::i3c::RegisterBlock,
    i3cg: *const ast1060_pac::i3cglobal::RegisterBlock,
    scu: *const ast1060_pac::scu::RegisterBlock,
    bus: u8,
    // `*const ()` marker keeps the handle `!Send` and `!Sync`. An
    // `I3cRegisters` represents exclusive ownership of one bus's register
    // blocks; it must not be shared between threads or moved into another
    // execution context where it could alias the controller it owns.
    _not_send_sync: PhantomData<*const ()>,
}

impl I3cRegisters {
    /// Create the register façade for `bus` (0..[`MAX_BUSES`]).
    ///
    /// Returns `None` if `bus` is out of range — every accessor below is
    /// therefore panic-free: a constructed façade always holds valid pointers
    /// and an in-range bus index.
    ///
    /// # Safety
    ///
    /// This is the entire `unsafe` perimeter for I3C MMIO (Delta D3):
    /// - The AST1060 PAC singleton pointers (`I3c*::ptr()`,
    ///   `I3cglobal::ptr()`, `Scu::ptr()`) must point to valid register
    ///   blocks for the program's lifetime (they do on AST1060 hardware).
    /// - Access through the returned façade must be serialized by the caller
    ///   (the type is `!Sync`); only one owner per physical bus may be
    ///   active at a time.
    #[must_use]
    pub const unsafe fn new(bus: u8) -> Option<Self> {
        let i3c = match bus {
            0 => ast1060_pac::I3c::ptr(),
            1 => ast1060_pac::I3c1::ptr(),
            2 => ast1060_pac::I3c2::ptr(),
            3 => ast1060_pac::I3c3::ptr(),
            _ => return None,
        };
        // Redundant with the match above, but keeps the invariant explicit if
        // MAX_BUSES and the match ever diverge.
        if bus as usize >= MAX_BUSES {
            return None;
        }
        Some(Self {
            i3c,
            i3cg: ast1060_pac::I3cglobal::ptr(),
            scu: ast1060_pac::Scu::ptr(),
            bus,
            _not_send_sync: PhantomData,
        })
    }

    /// Bus index this façade was constructed for (always `< MAX_BUSES`).
    #[inline]
    #[must_use]
    pub fn bus(&self) -> u8 {
        self.bus
    }

    /// The only repeated interior `unsafe` for the I3C block.
    ///
    /// Returns a `'static` reference: the constructor's contract guarantees
    /// the pointer is valid for the program lifetime, so the borrow is not
    /// tied to `&self`. This lets a register reference and a `&mut yield_fn`
    /// be held in disjoint statements at the bounded-poll sites without a
    /// borrow clash.
    #[inline]
    pub(crate) fn i3c(&self) -> &'static ast1060_pac::i3c::RegisterBlock {
        // SAFETY: `new` guarantees a valid pointer for the program lifetime;
        // access is serialized by the caller (the type is `!Sync`).
        unsafe { &*self.i3c }
    }

    /// The only repeated interior `unsafe` for the I3C-global block. See
    /// [`i3c`](Self::i3c).
    #[inline]
    pub(crate) fn i3cg(&self) -> &'static ast1060_pac::i3cglobal::RegisterBlock {
        // SAFETY: see `i3c`.
        unsafe { &*self.i3cg }
    }

    /// The only repeated interior `unsafe` for the SCU block. See
    /// [`i3c`](Self::i3c).
    #[inline]
    pub(crate) fn scu(&self) -> &'static ast1060_pac::scu::RegisterBlock {
        // SAFETY: see `i3c`.
        unsafe { &*self.scu }
    }
}
