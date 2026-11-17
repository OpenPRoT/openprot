// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU routing: `route` flushes the boot pin-mux (each pin's authored route) in one panic-free pass.
//!
//! `route` is not gated by a `Region` or recorded as a claim: `ScuBlock` is a silicon-fixed
//! singleton (see `field_mux::Block`), not something a process is granted through `system.json5`.
//! It is meant to run once, from privileged boot code, same as
//! `ScuRegisters::new_global_unlocked`. Closing that gap — so two processes racing to mux the SCU
//! is as visible as two processes racing to drive the same GPIO pin — is tracked as follow-up
//! work, not done here.

use super::pins::ScuBlock;
use super::registers::unlock_scu_writes;
use crate::Mmio;
use openprot_hal::field_mux::MuxRoutes;

/// Route the whole board in one call: hand *every* pin handle as a single tuple so all routes fold to one RMW per register at compile time — call this exactly once per board.
pub fn route<H: MuxRoutes>(_handle: H) {
    let scu = Mmio::<ScuBlock>::block();
    unlock_scu_writes(&scu);
    // HACK: full MAX_REGS-wide COALESCED lands in rodata; exact-N trim dropped to shed generic_const_exprs.
    openprot_hal::field_mux::apply(&scu, const { &H::COALESCED }.as_slice());
}
