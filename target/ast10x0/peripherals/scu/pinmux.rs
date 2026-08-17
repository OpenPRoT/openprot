// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU routing: `apply_mux` flushes the boot pin-mux (each pin's authored route) in one panic-free pass.

use super::pins::ScuBlock;
use super::registers::unlock_scu_writes;
use crate::Mmio;
use openprot_hal::field_mux::MuxRoutes;
use openprot_hal::resource::FieldWrite;

/// DEPRECATED: legacy per-pin mux datum for `apply_pinctrl_group`; superseded by `FieldWrite` + `RegBatch`.
/// Describes a single pin mux operation: which SCU register bit to set (or clear).
#[derive(Clone, Copy, Debug)]
pub struct PinctrlPin {
    /// SCU register offset (0x410, 0x414, 0x690, etc.)
    pub offset: u32,
    /// Bit position within the register (0-31)
    pub bit: u32,
    /// true = clear bit, false = set bit
    pub clear: bool,
}

impl PinctrlPin {
    /// Route a pin by setting its mux bit: the fact a `pins!` row states inline.
    #[must_use]
    pub const fn new(offset: u32, bit: u32) -> Self {
        Self {
            offset,
            bit,
            clear: false,
        }
    }
}

/// Flush the compile-time-coalesced pin-mux to the SCU in one panic-free pass — one RMW per distinct register.
pub fn apply_mux(mux: &[FieldWrite]) {
    let scu = Mmio::<ScuBlock>::block();
    unlock_scu_writes(&scu);
    openprot_hal::field_mux::apply(&scu, mux);
}

/// Route the whole board in one call: hand *every* pin handle as a single tuple so all routes fold to one RMW per register at compile time — call this exactly once per board.
pub fn route<H: MuxRoutes>(_handle: &H) {
    let scu = Mmio::<ScuBlock>::block();
    unlock_scu_writes(&scu);
    // HACK: full MAX_REGS-wide COALESCED lands in rodata; exact-N trim dropped to shed generic_const_exprs.
    openprot_hal::field_mux::apply(&scu, const { &H::COALESCED }.as_slice());
}
