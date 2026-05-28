// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HACE device binding with cooperative yield.

use super::constants::DEFAULT_POLL_BUDGET;
use super::context::{CryptoContext, HashContext, acquire_crypto_ctx, acquire_shared_ctx};
use super::registers::HaceRegisters;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
}

/// The one owned binding over the HACE engine.
///
/// Exclusivity member of the *owned-peripheral* family (`design-patterns` ::
/// `borrow-arbitrated-engine-exclusivity`): this value is **not**
/// `Copy`/`Clone` (the `*mut HashContext` field also makes it structurally
/// move-only), and every operation handle — `HaceDigest`, `HaceHmac` — is
/// produced *only* by an exclusive `&mut HaceDevice` borrow-split. Two
/// concurrent ops of the one engine, of any kind, are therefore a borrow-check
/// error: the Rust `&mut`-exclusivity rule is the arbiter, replacing the Zephyr
/// reference's `in_use`/`-EBUSY` caller-serialization discipline. No runtime
/// busy flag and no hardware busy-bit read exists anywhere in this driver.
///
/// `ctx` is the engine's operation state. It must be a `.ram_nc`,
/// `#[repr(C, align(64))]` static (DMA targets — SG list / `buffer` /
/// `digest`; `goal.md` §1.3/§5.1) so it cannot be an inline by-value field of
/// this (stack-placeable) device; instead the device holds the *sole* pointer
/// to it, acquired once at the construction gate via
/// [`acquire_shared_ctx`](super::context::acquire_shared_ctx) — there is no
/// free accessor handing it out elsewhere (Checklist box 2). The live
/// `&mut HashContext` exists *only* transiently inside an operation, reborrowed
/// through the device's `&mut`. Single-instance-per-engine is gate-delegated to
/// the documented `unsafe fn new*` contract (Checklist box 3), as in the
/// sibling SBC port; that residual static is the pattern's stated hardware
/// liability, not a soundness gap.
pub struct HaceDevice<Y: FnMut(u32)> {
    pub(crate) regs: HaceRegisters,
    /// Sole pointer to the section-placed [`HashContext`]. Reborrowed as a
    /// transient `&mut` through `&mut self` by each operation's `from_device`
    /// (a disjoint-field split alongside `yield_fn`); never aliased outside
    /// the device.
    pub(crate) ctx: *mut HashContext,
    /// Sole pointer to the section-placed [`CryptoContext`] (AES path). Same
    /// discipline as `ctx`: reborrowed as a transient `&mut` through
    /// `&mut self` by `AesCipher::from_device` (disjoint-field split alongside
    /// `yield_fn`); never aliased outside the device. AES is the engine's
    /// third borrow-arbitrated operation (goal.md §2.3 delta A1 / §5.1).
    pub(crate) crypto_ctx: *mut CryptoContext,
    /// Cooperative yield hook invoked between completion polls.
    /// Argument is a suggested wait window in nanoseconds.
    pub(crate) yield_fn: Y,
    pub(crate) poll_budget: u32,
}

impl<Y: FnMut(u32)> HaceDevice<Y> {
    /// Create a device bound to a raw HACE register block with a caller-provided
    /// cooperative yield strategy.
    ///
    /// # Safety
    /// Caller must uphold the same safety contract as [`HaceRegisters::new`].
    /// This type is non-reentrant: only one `HaceDevice` may be active at a time.
    pub unsafe fn new_with_yield(
        base: *const ast1060_pac::hace::RegisterBlock,
        yield_fn: Y,
    ) -> Self {
        Self {
            // SAFETY: Caller upholds register-pointer validity/ownership.
            regs: unsafe { HaceRegisters::new(base) },
            // SAFETY: the `unsafe fn new*` single-instance contract makes this
            // the sole live device, hence the sole holder of these pointers.
            ctx: unsafe { acquire_shared_ctx() },
            crypto_ctx: unsafe { acquire_crypto_ctx() },
            yield_fn,
            poll_budget: DEFAULT_POLL_BUDGET,
        }
    }

    /// Create a device bound to a raw HACE register block.
    ///
    /// # Safety
    /// Caller must uphold the same safety contract as [`HaceRegisters::new`].
    /// This type is non-reentrant: only one `HaceDevice` may be active at a time.
    pub unsafe fn new(base: *const ast1060_pac::hace::RegisterBlock, yield_fn: Y) -> Self {
        // SAFETY: Same contract as this wrapper.
        unsafe { Self::new_with_yield(base, yield_fn) }
    }

    /// Create a device bound to the singleton HACE instance with a caller-provided
    /// cooperative yield strategy.
    ///
    /// # Safety
    /// Caller must coordinate singleton access globally.
    /// This type is non-reentrant: only one `HaceDevice` may be active at a time.
    pub unsafe fn new_global_with_yield(yield_fn: Y) -> Self {
        Self {
            // SAFETY: Caller coordinates singleton access.
            regs: unsafe { HaceRegisters::new_global() },
            // SAFETY: the `unsafe fn new*` single-instance contract makes this
            // the sole live device, hence the sole holder of these pointers.
            ctx: unsafe { acquire_shared_ctx() },
            crypto_ctx: unsafe { acquire_crypto_ctx() },
            yield_fn,
            poll_budget: DEFAULT_POLL_BUDGET,
        }
    }

    /// Create a device bound to the singleton HACE instance.
    ///
    /// # Safety
    /// Caller must coordinate singleton access globally.
    /// This type is non-reentrant: only one `HaceDevice` may be active at a time.
    pub unsafe fn new_global(yield_fn: Y) -> Self {
        // SAFETY: Same contract as this wrapper.
        unsafe { Self::new_global_with_yield(yield_fn) }
    }

    /// Override polling timeout budget for operation completion.
    #[must_use]
    pub const fn with_timeout_polls(mut self, timeout_polls: u32) -> Self {
        self.poll_budget = timeout_polls;
        self
    }
}
