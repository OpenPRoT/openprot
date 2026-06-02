// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Confined-`unsafe` MMIO façade for the AST1060 I2C controller.
//!
//! Sibling of `target/ast10x0/peripherals/hace/registers.rs` — the catalog
//! reference for the *Confined-`unsafe` MMIO Façade* pattern. All `unsafe`
//! required to touch the I2C and I2CBUFF register blocks is confined here:
//! one `unsafe fn new` (the entire perimeter, discharged once by the caller)
//! and one private deref per block (`i2c()`, `buff()`). The driver state
//! machine (`controller`/`master`/`slave`/…) reaches hardware **only** through
//! these two safe accessors, so no driver code constructs a raw register
//! pointer.

use core::marker::PhantomData;

use ast1060_pac::{i2c::RegisterBlock, i2cbuff::RegisterBlock as BuffRegisterBlock};

/// Safe façade over one AST1060 controller's `(I2C, I2CBUFF)` register pair.
///
/// `Copy` and pointer-only by design (per the pattern): it *confines*
/// `unsafe` and *restricts threading*; it does **not** enforce exclusive
/// access — that is delegated to the gate's caller and to the owning
/// `Ast1060I2c` (one instance per bus, every op an `&mut` borrow).
#[derive(Copy, Clone)]
pub struct Ast1060I2cRegisters {
    i2c: *mut RegisterBlock,
    buff: *mut BuffRegisterBlock,
    /// `*mut` ⇒ `!Send`/`!Sync`: the register pointers cannot be shared
    /// across threads without explicit synchronization.
    _not_send: PhantomData<*mut ()>,
}

impl Ast1060I2cRegisters {
    /// The **entire** `unsafe` perimeter for AST1060 I2C MMIO — discharged
    /// exactly once, here, by the caller.
    ///
    /// # Safety
    ///
    /// - `i2c` and `buff` are valid, non-null pointers to the I2C and
    ///   I2CBUFF register blocks of the **same** controller, and remain
    ///   valid for the lifetime of every `Ast1060I2c` built from this value.
    /// - Access to the controller is serialized by the caller (the i2c
    ///   server owns one instance per bus; all ops are `&mut`).
    #[must_use]
    pub const unsafe fn new(i2c: *const RegisterBlock, buff: *const BuffRegisterBlock) -> Self {
        Self {
            i2c: i2c as *mut RegisterBlock,
            buff: buff as *mut BuffRegisterBlock,
            _not_send: PhantomData,
        }
    }

    /// Sole `unsafe` deref of the I2C block; justified by `new`'s invariant.
    #[inline]
    pub(crate) fn i2c(&self) -> &RegisterBlock {
        // SAFETY: `new` guarantees a valid pointer for this value's lifetime;
        // access is serialized by the owning `Ast1060I2c` (`&mut` per op).
        unsafe { &*self.i2c }
    }

    /// Sole `unsafe` deref of the I2CBUFF block; same justification as `i2c`.
    #[inline]
    pub(crate) fn buff(&self) -> &BuffRegisterBlock {
        // SAFETY: see `i2c`.
        unsafe { &*self.buff }
    }
}
